//! Canal seguro entre la ventana y el motor (regla de seguridad 1 de CLAUDE.md).
//!
//! - Escucha **solo en 127.0.0.1**, en un puerto al azar: nada de la red llega.
//! - **Token secreto** nuevo en cada arranque (32 bytes del generador del
//!   sistema): sin él no se sirve la página ni se acepta el WebSocket.
//! - **Host** debe ser `127.0.0.1:<puerto>` (contra *DNS rebinding*) y el
//!   **Origin** del WebSocket `http://127.0.0.1:<puerto>` (contra páginas web
//!   maliciosas abiertas en el navegador). Sin CORS.
//! - Límites: 16 conexiones, cabeceras de 8 KB, mensajes de 64 KB, 5 s para
//!   mandar el pedido.

pub mod comprobar;
pub mod importar;
pub mod sesion;
pub mod sonido;
pub mod texto;
pub mod proyector;

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Error as ErrorWs, Message};

pub use sesion::{Modo, Sesion, TareaPreparar};
pub use sonido::{Destino, Sonido};
pub use texto::{Idioma, Texto};
pub use proyector::{motivo, AvisoProyector, OrdenProyector, Proyector};

const PLANTILLA: &str = include_str!("../ui/index.html");
/// Textos de la interfaz en español e inglés (también los consejos y la
/// ayuda). Se prueban en `tests/textos.rs`.
pub const TEXTOS: &str = include_str!("../ui/textos.json");

/// Los textos que recibe la ventana: `ui/textos.json` más los consejos de las
/// 7 rampas cortas (`rampa_inicio`…`rampa_fin`), que viven en el motor de
/// audio (`otaconvert_audio::consejos`) para no tener dos copias.
pub fn textos() -> &'static str {
    static T: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        use otaconvert_audio::consejos::{Idioma as I, TipoRampa};
        let mut v: serde_json::Value = serde_json::from_str(TEXTOS).expect("ui/textos.json");
        for (clave, idioma) in [("es", I::Es), ("en", I::En)] {
            let Some(c) = v.get_mut(clave).and_then(|x| x.get_mut("consejos")).and_then(|x| x.as_object_mut()) else { continue };
            for (tipo, nombre) in TipoRampa::TODAS.into_iter().zip(["inicio", "reanudar", "pausa", "parada", "salto", "volumen", "fin"]) {
                let k = tipo.consejo(idioma);
                c.insert(
                    format!("rampa_{nombre}"),
                    serde_json::json!({
                        "titulo": k.titulo, "que_hace": k.que_hace, "activar": k.activar,
                        "ejemplo_activar": k.ejemplo_activar, "desactivar": k.desactivar, "ejemplo_desactivar": k.ejemplo_desactivar,
                    }),
                );
            }
        }
        v.to_string()
    })
}

/// La página con los textos dentro (se arma una vez). `</` se escapa para
/// que un texto nunca pueda cerrar el `<script>`.
fn pagina() -> &'static str {
    static P: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    P.get_or_init(|| PLANTILLA.replace("\"__TEXTOS__\"", &textos().replace("</", "<\\/")))
}

const MAX_CONEXIONES: usize = 16;
const MAX_CABECERAS: usize = 8 * 1024;
const MAX_MENSAJE: usize = 64 * 1024;
const ESPERA_PEDIDO: Duration = Duration::from_secs(5);
const GUARDADO_AUTOMATICO: Duration = Duration::from_secs(30);
const REFRESCO: Duration = Duration::from_millis(250);

struct Contexto {
    puerto: u16,
    token: String,
    sesion: Arc<Mutex<Sesion>>,
    clientes: Mutex<Vec<Sender<String>>>,
    conexiones: AtomicUsize,
    /// Diálogo "Buscar archivos…" del sistema (lo pone el motor; en las pruebas no hay).
    buscar_archivos: Mutex<Option<BuscarArchivos>>,
    equipo: Mutex<Option<(ListaEquipo, ElegirSalida)>>,
    comprobar_equipo: Mutex<Option<comprobar::ComprobarEquipo>>,
}

/// Abre el diálogo de Windows para elegir archivos y devuelve los elegidos (vacío = canceló).
pub type BuscarArchivos = Arc<dyn Fn() -> Vec<std::path::PathBuf> + Send + Sync>;

/// Salidas de sonido y pantallas de este equipo (etapa 5B), en JSON para la ventana.
pub type ListaEquipo = Arc<dyn Fn() -> serde_json::Value + Send + Sync>;
/// Cambia una salida: `que` = "principal" | "auriculares" | "pantalla"; `id` = cuál
/// (`None` = la predeterminada de Windows, o quitar Auriculares). Lo hace el motor,
/// que tiene las tarjetas abiertas; si falla, queda la que estaba.
pub type ElegirSalida = Arc<dyn Fn(&str, Option<String>) -> Result<(), Texto> + Send + Sync>;

pub struct Servidor {
    pub puerto: u16,
    token: String,
    ctx: Arc<Contexto>,
}

/// Token de 32 bytes al azar, en hexadecimal.
fn nuevo_token() -> io::Result<String> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Compara sin cortar en la primera diferencia (no revela cuánto acertó).
fn iguales(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn token_de(consulta: Option<&str>) -> Option<&str> {
    consulta?.split('&').find_map(|p| p.strip_prefix("t="))
}

impl Servidor {
    /// Arranca el servidor y el guardado automático. Vive mientras viva el proceso.
    pub fn iniciar(sesion: Arc<Mutex<Sesion>>) -> io::Result<Self> {
        let escucha = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
        let puerto = escucha.local_addr()?.port();
        let token = nuevo_token()?;
        let ctx = Arc::new(Contexto {
            puerto,
            token: token.clone(),
            sesion: sesion.clone(),
            clientes: Mutex::new(Vec::new()),
            conexiones: AtomicUsize::new(0),
            buscar_archivos: Mutex::new(None),
            equipo: Mutex::new(None),
            comprobar_equipo: Mutex::new(None),
        });
        let c = ctx.clone();
        std::thread::Builder::new().name("servidor".into()).spawn(move || {
            for entrante in escucha.incoming() {
                let Ok(stream) = entrante else { continue };
                if c.conexiones.fetch_add(1, Ordering::SeqCst) >= MAX_CONEXIONES {
                    c.conexiones.fetch_sub(1, Ordering::SeqCst);
                    continue; // se cierra al soltarla
                }
                let c2 = c.clone();
                let _ = std::thread::Builder::new().name("conexion".into()).spawn(move || {
                    atender(stream, &c2);
                    c2.conexiones.fetch_sub(1, Ordering::SeqCst);
                });
            }
        })?;
        std::thread::Builder::new().name("guardado-automatico".into()).spawn(move || loop {
            std::thread::sleep(GUARDADO_AUTOMATICO);
            if let Ok(mut s) = sesion.lock() {
                s.guardar_si_hace_falta();
            }
        })?;
        // Prepara en segundo plano los audios que vienen (sin tener la sesión
        // bloqueada mientras lee el disco) y refresca lo que suena.
        let c3 = ctx.clone();
        std::thread::Builder::new().name("preparador".into()).spawn(move || loop {
            let tarea = c3.sesion.lock().ok().and_then(|mut s| s.siguiente_a_preparar());
            match tarea {
                Some(t) => {
                    let r = t.hacer();
                    let estado = c3.sesion.lock().ok().map(|mut s| {
                        s.resultado_preparar(t.cue, r);
                        s.estado_json()
                    });
                    if let Some(e) = estado {
                        difundir(&c3, e);
                    }
                }
                None => std::thread::sleep(Duration::from_millis(100)),
            }
        })?;
        let c4 = ctx.clone();
        std::thread::Builder::new().name("refresco".into()).spawn(move || {
            let mut ultimo = String::new();
            loop {
                std::thread::sleep(REFRESCO);
                let estado = c4.sesion.lock().ok().map(|mut s| {
                    s.limpiar();
                    s.estado_json()
                });
                if let Some(e) = estado {
                    if e != ultimo {
                        ultimo = e.clone();
                        difundir(&c4, e);
                    }
                }
            }
        })?;
        Ok(Self { puerto, token, ctx })
    }

    /// Dirección que abre la ventana (o el navegador).
    /// El motor pone aquí el diálogo de Windows "Buscar archivos…".
    pub fn al_buscar_archivos(&self, f: BuscarArchivos) {
        if let Ok(mut b) = self.ctx.buscar_archivos.lock() {
            *b = Some(f);
        }
    }

    /// Luz verde (etapa 5C): lo que el motor revisa del equipo (protector de
    /// pantalla, batería, pendrive…) además del show.
    pub fn al_comprobar_equipo(&self, f: comprobar::ComprobarEquipo) {
        if let Ok(mut b) = self.ctx.comprobar_equipo.lock() {
            *b = Some(f);
        }
    }

    /// Etapa 5B: listar y elegir las salidas de sonido y la pantalla del proyector.
    pub fn al_elegir_salidas(&self, lista: ListaEquipo, elegir: ElegirSalida) {
        if let Ok(mut b) = self.ctx.equipo.lock() {
            *b = Some((lista, elegir));
        }
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/?t={}", self.puerto, self.token)
    }
}

fn atender(stream: TcpStream, ctx: &Arc<Contexto>) {
    let _ = stream.set_read_timeout(Some(ESPERA_PEDIDO));
    let _ = stream.set_nodelay(true);
    let mut vista = [0u8; 16];
    let n = stream.peek(&mut vista).unwrap_or(0);
    if vista[..n].starts_with(b"GET /ws") {
        websocket(stream, ctx);
    } else {
        http(stream, ctx);
    }
}

fn responder(mut s: &TcpStream, estado: &str, extra: &str, cuerpo: &str) {
    let _ = write!(
        s,
        "HTTP/1.1 {estado}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nConnection: close\r\n{extra}\r\n{cuerpo}",
        cuerpo.len()
    );
    let _ = s.flush();
}

fn http(mut stream: TcpStream, ctx: &Contexto) {
    // Leer las cabeceras hasta la línea vacía, sin pasar de 8 KB.
    let mut buf = Vec::with_capacity(1024);
    let mut trozo = [0u8; 1024];
    loop {
        match stream.read(&mut trozo) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&trozo[..n]),
        }
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > MAX_CABECERAS {
            responder(&stream, "431 Request Header Fields Too Large", "", "");
            // Cerrar con datos sin leer hace que Windows corte la conexión de
            // golpe (RST) y el otro lado puede perder la respuesta: se avisa
            // el fin y se descarta lo que siga llegando (con límite y tiempo).
            let _ = stream.shutdown(std::net::Shutdown::Write);
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            let mut descartado = 0usize;
            while descartado < 1024 * 1024 {
                match stream.read(&mut trozo) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => descartado += n,
                }
            }
            return;
        }
    }
    let texto = String::from_utf8_lossy(&buf);
    let mut lineas = texto.split("\r\n");
    let primera = lineas.next().unwrap_or("");
    let mut partes = primera.split(' ');
    let (metodo, destino) = (partes.next().unwrap_or(""), partes.next().unwrap_or(""));
    let cabeceras: Vec<(String, String)> = lineas
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let cabecera = |n: &str| cabeceras.iter().find(|(k, _)| k == n).map(|(_, v)| v.as_str());
    if metodo != "GET" {
        responder(&stream, "405 Method Not Allowed", "Allow: GET\r\n", "");
        return;
    }
    if cabecera("host") != Some(&format!("127.0.0.1:{}", ctx.puerto)) {
        responder(&stream, "403 Forbidden", "", "<p>Acceso no permitido.</p>");
        return;
    }
    let (ruta, consulta) = match destino.split_once('?') {
        Some((r, q)) => (r, Some(q)),
        None => (destino, None),
    };
    if !matches!(ruta, "/" | "/miniatura" | "/medio") {
        responder(&stream, "404 Not Found", "", "");
        return;
    }
    if !token_de(consulta).is_some_and(|t| iguales(t, &ctx.token)) {
        responder(&stream, "403 Forbidden", "", "<p>Abre Otaconvert desde el propio programa.</p>");
        return;
    }
    match ruta {
        "/" => {
            let csp = format!(
                "Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src ws://127.0.0.1:{}; img-src 'self' data:; media-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\r\n",
                ctx.puerto
            );
            responder(&stream, "200 OK", &csp, pagina());
        }
        // Solo se entrega lo de un cue del show, por su número: la ventana
        // nunca puede pedir una ruta de archivo.
        _ => {
            let cue = consulta
                .and_then(|q| q.split('&').find_map(|p| p.strip_prefix("cue=")))
                .and_then(|c| c.parse::<usize>().ok());
            let archivo = cue.and_then(|i| {
                let s = ctx.sesion.lock().ok()?;
                if ruta == "/miniatura" {
                    s.miniatura_lista(i)
                } else {
                    s.medio_visual(i)
                }
            });
            match archivo {
                Some(a) => enviar_archivo(&stream, &a, cabecera("range"), ruta == "/miniatura"),
                None => responder(&stream, "404 Not Found", "", ""),
            }
        }
    }
}

/// Tipo de contenido por extensión (solo los medios que se muestran).
fn tipo_de(ruta: &std::path::Path) -> &'static str {
    match ruta.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("webp") => "image/webp",
        Some("mp4" | "m4v") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("webm") => "video/webm",
        Some("mkv") => "video/x-matroska",
        _ => "application/octet-stream",
    }
}

/// Entrega un archivo (entero o por partes, para el vídeo de la vista
/// previa), copiándolo por trozos: nunca se carga entero en memoria.
/// `guardar`: la foto se puede guardar en la caché de la ventana (su dirección
/// lleva la versión, que cambia si cambia el archivo).
fn enviar_archivo(mut s: &TcpStream, ruta: &std::path::Path, rango: Option<&str>, guardar: bool) {
    use std::io::{Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(ruta) else {
        responder(s, "404 Not Found", "", "");
        return;
    };
    let total = f.metadata().map(|m| m.len()).unwrap_or(0);
    let tipo = tipo_de(ruta);
    // "bytes=inicio-fin" (fin opcional). Rangos raros o múltiples: archivo entero.
    let parte = rango.and_then(|r| r.strip_prefix("bytes=")).filter(|r| !r.contains(',')).and_then(|r| {
        let (a, b) = r.split_once('-')?;
        let ini: u64 = a.trim().parse().ok()?;
        let fin: u64 = if b.trim().is_empty() { total.saturating_sub(1) } else { b.trim().parse().ok()? };
        (ini <= fin && fin < total).then_some((ini, fin))
    });
    if rango.is_some() && parte.is_none() && total > 0 {
        let _ = write!(s, "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    }
    let cache = if guardar { "private, max-age=31536000, immutable" } else { "no-store" };
    let comunes = format!("Accept-Ranges: bytes\r\nCache-Control: {cache}\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n");
    let (ini, largo) = match parte {
        Some((a, b)) => {
            let _ = write!(
                s,
                "HTTP/1.1 206 Partial Content\r\nContent-Type: {tipo}\r\nContent-Length: {}\r\nContent-Range: bytes {a}-{b}/{total}\r\n{comunes}\r\n",
                b - a + 1
            );
            (a, b - a + 1)
        }
        None => {
            let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: {tipo}\r\nContent-Length: {total}\r\n{comunes}\r\n");
            (0, total)
        }
    };
    if f.seek(SeekFrom::Start(ini)).is_ok() {
        let _ = io::copy(&mut (&mut f).take(largo), &mut s);
    }
    let _ = s.flush();
}

fn difundir(ctx: &Contexto, texto: String) {
    if let Ok(mut c) = ctx.clientes.lock() {
        c.retain(|tx| tx.send(texto.clone()).is_ok());
    }
}

/// Trae archivos al show (copiando a `Medios/` lo que está afuera) y agrega
/// sus cues después de `despues_de`. La copia se hace **sin** tener la sesión
/// tomada: mientras se copia un vídeo grande, el GO y Parar todo siguen.
fn traer_archivos(ctx: &Contexto, rutas: Vec<std::path::PathBuf>, despues_de: Option<usize>) {
    if rutas.is_empty() {
        return;
    }
    let carpeta = match ctx.sesion.lock() {
        Ok(mut s) => {
            if s.modo == sesion::Modo::Funcion {
                s.aviso = Some(Texto::nuevo("En Función no se puede cambiar el show. Pasa a Edición con Tab.", "The show can't be changed in Live mode. Switch to Edit with Tab."));
                let e = s.estado_json();
                drop(s);
                difundir(ctx, e);
                return;
            }
            s.aviso = Some(Texto::nuevo(format!("Agregando {} archivo(s)…", rutas.len()), format!("Adding {} file(s)…", rutas.len())));
            s.carpeta()
        }
        Err(_) => return,
    };
    if let Ok(s) = ctx.sesion.lock() {
        let e = s.estado_json();
        drop(s);
        difundir(ctx, e);
    }
    let resultado = importar::planear(&carpeta, &rutas).and_then(|p| importar::copiar(&p));
    let Ok(mut s) = ctx.sesion.lock() else { return };
    match resultado.and_then(|rel| s.agregar_cues(&rel, despues_de)) {
        Ok(nuevos) => {
            s.aviso = Some(Texto::nuevo(format!("Se agregaron {} cue(s).", nuevos.len()), format!("{} cue(s) added.", nuevos.len())));
        }
        Err(e) => s.aviso = Some(e),
    }
    let e = s.estado_json();
    drop(s);
    difundir(ctx, e);
}

fn ejecutar(ctx: &Arc<Contexto>, texto: &str) {
    let v = serde_json::from_str::<serde_json::Value>(texto).unwrap_or_default();
    let orden = v.get("orden").and_then(|o| o.as_str()).map(str::to_string);
    let indice = || v.get("cue").and_then(|c| c.as_u64()).and_then(|c| usize::try_from(c).ok());
    let despues_de = v.get("despues_de").and_then(|c| c.as_u64()).and_then(|c| usize::try_from(c).ok());
    // Traer archivos de afuera (arrastrados o con el diálogo de Windows): en otro hilo.
    match orden.as_deref() {
        Some("importar") => {
            let rutas: Vec<std::path::PathBuf> = v
                .get("rutas")
                .and_then(|r| r.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str()).take(1000).map(std::path::PathBuf::from).collect())
                .unwrap_or_default();
            // En otro hilo: mientras se copia, esta ventana sigue pudiendo dar GO o Parar todo.
            let c = ctx.clone();
            let _ = std::thread::Builder::new().name("importar".into()).spawn(move || traer_archivos(&c, rutas, despues_de));
            return;
        }
        Some("buscar_archivos") => {
            let f = ctx.buscar_archivos.lock().ok().and_then(|b| b.clone());
            if let Some(f) = f {
                let c = ctx.clone();
                let _ = std::thread::Builder::new().name("buscar-archivos".into()).spawn(move || traer_archivos(&c, f(), despues_de));
            }
            return;
        }
        // Luz verde (etapa 5C): revisar el show antes de la función. Solo en
        // Edición (decodifica todos los audios: no se hace en plena función),
        // en otro hilo y sin tener la sesión tomada.
        Some("comprobar") => {
            let pedido = match ctx.sesion.lock() {
                Ok(mut s) => {
                    let r = if s.comprobando.is_some() {
                        Err(Texto::nuevo("La comprobación ya está en marcha.", "The check is already running."))
                    } else if s.modo == sesion::Modo::Funcion {
                        Err(Texto::nuevo("La comprobación se hace en Edición (revisa todos los audios). Pasa a Edición con Tab.", "The check runs in Edit mode (it reads every audio file). Switch to Edit with Tab."))
                    } else {
                        let p = s.pedido_comprobacion();
                        s.comprobando = Some((0, p.cues.len()));
                        s.anotar("COMPROBAR", "Comprobación antes de la función");
                        Ok(p)
                    };
                    if let Err(e) = &r {
                        s.aviso = Some(e.clone());
                    }
                    let e = s.estado_json();
                    drop(s);
                    difundir(ctx, e);
                    r
                }
                Err(_) => return,
            };
            let Ok(pedido) = pedido else { return };
            let equipo = ctx.comprobar_equipo.lock().ok().and_then(|b| b.clone());
            let c = ctx.clone();
            let _ = std::thread::Builder::new().name("comprobar".into()).spawn(move || {
                let mut ultimo = std::time::Instant::now();
                let mut puntos = comprobar::revisar(&pedido, |h, t| {
                    if let Ok(mut s) = c.sesion.lock() {
                        s.comprobando = Some((h, t));
                        // Avance a la ventana, como mucho 4 veces por segundo.
                        if ultimo.elapsed() > Duration::from_millis(250) || h == t {
                            ultimo = std::time::Instant::now();
                            let e = s.estado_json();
                            drop(s);
                            difundir(&c, e);
                        }
                    }
                });
                if let Some(f) = equipo {
                    puntos.extend(f(&pedido.carpeta));
                }
                let malos = puntos.iter().filter(|p| p.nivel == comprobar::Nivel::Problema).count();
                let avisos = puntos.iter().filter(|p| p.nivel == comprobar::Nivel::Aviso).count();
                if let Ok(mut s) = c.sesion.lock() {
                    let m = otaconvert_show::marca_de_tiempo();
                    let hora = format!("{}:{}", m.get(9..11).unwrap_or(""), m.get(11..13).unwrap_or(""));
                    s.comprobando = None;
                    s.comprobacion = Some((puntos, hora));
                    s.anotar("COMPROBAR", &format!("{malos} problema(s), {avisos} aviso(s)"));
                    let e = s.estado_json();
                    drop(s);
                    difundir(&c, e);
                }
            });
            return;
        }
        // Etapa 5B: salidas de sonido y pantalla del proyector de este equipo.
        Some("equipo_lista") => {
            let f = ctx.equipo.lock().ok().and_then(|b| b.as_ref().map(|(l, _)| l.clone()));
            if let Some(f) = f {
                let c = ctx.clone();
                let _ = std::thread::Builder::new().name("equipo-lista".into()).spawn(move || {
                    let mut v = f();
                    v["tipo"] = serde_json::json!("equipo");
                    difundir(&c, v.to_string());
                });
            }
            return;
        }
        Some("elegir_salida") => {
            let que = v.get("que").and_then(|q| q.as_str()).unwrap_or("").to_string();
            let id = v.get("id").and_then(|i| i.as_str()).map(|s| s.chars().take(512).collect::<String>());
            let f = ctx.equipo.lock().ok().and_then(|b| b.as_ref().map(|(_, e)| e.clone()));
            let c = ctx.clone();
            // En otro hilo y sin tener la sesión tomada: abrir una tarjeta o
            // la proyección tarda, y el motor necesita la sesión para hacerlo.
            let _ = std::thread::Builder::new().name("elegir-salida".into()).spawn(move || {
                let r = match (que.as_str(), f) {
                    ("principal" | "auriculares" | "pantalla", Some(f)) => f(&que, id),
                    (_, None) => Err(Texto::nuevo("Este programa no puede cambiar salidas.", "This program can't change outputs.")),
                    _ => Err(Texto::nuevo("Salida no válida.", "Invalid output.")),
                };
                if let Ok(mut s) = c.sesion.lock() {
                    s.aviso = r.err();
                    s.salidas_respondidas += 1;
                    let e = s.estado_json();
                    drop(s);
                    difundir(&c, e);
                }
            });
            return;
        }
        Some("medios_carpeta") => {
            let carpeta = match ctx.sesion.lock() {
                Ok(s) => s.carpeta(),
                Err(_) => return,
            };
            let lista = importar::medios_de_la_carpeta(&carpeta);
            difundir(ctx, serde_json::json!({ "tipo": "medios", "lista": lista }).to_string());
            return;
        }
        _ => {}
    }
    let estado = {
        let Ok(mut s) = ctx.sesion.lock() else { return };
        match orden.as_deref() {
            // Etapa 5A: editar la lista de cues.
            Some("agregar") => {
                let archivos: Vec<String> = v
                    .get("archivos")
                    .and_then(|r| r.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).take(1000).collect())
                    .unwrap_or_default();
                if let Err(e) = s.agregar_cues(&archivos, despues_de) {
                    s.aviso = Some(e);
                }
            }
            Some("quitar") => {
                let nombre = indice().and_then(|i| s.show.cues.get(i)).map(|c| c.nombre.clone()).unwrap_or_default();
                let r = match indice() {
                    Some(i) => s.quitar_cue(i),
                    None => Err(Texto::nuevo("Cue no válido.", "Invalid cue.")),
                };
                s.aviso = Some(match r {
                    // Quitar no pide confirmación: se puede deshacer.
                    Ok(()) => Texto::nuevo(
                        format!("Se quitó «{nombre}» de la lista (el archivo sigue en la carpeta). Ctrl+Z lo vuelve a poner."),
                        format!("«{nombre}» was removed from the list (the file stays in the folder). Ctrl+Z puts it back."),
                    ),
                    Err(e) => e,
                });
            }
            Some("mover") => {
                let a = v.get("hasta").and_then(|c| c.as_u64()).and_then(|c| usize::try_from(c).ok());
                let r = match (indice(), a) {
                    (Some(i), Some(h)) => s.mover_cue(i, h),
                    _ => Err(Texto::nuevo("Cue no válido.", "Invalid cue.")),
                };
                if let Err(e) = r {
                    s.aviso = Some(e);
                }
            }
            // Etapa 5B: Edición → Ajustes del show.
            Some("ajustes") => {
                let r = match v.get("cambios") {
                    Some(c) => s.cambiar_ajustes(c),
                    None => Err(Texto::nuevo("Ajustes no válidos.", "Invalid settings.")),
                };
                if let Err(e) = r {
                    s.aviso = Some(e);
                }
            }
            Some("deshacer") => {
                // Un aviso anterior (p. ej. «Se quitó…») ya no vale.
                s.aviso = s.deshacer().err();
            }
            Some("rehacer") => {
                s.aviso = s.rehacer().err();
            }
            Some("modo_edicion") => s.cambiar_modo(sesion::Modo::Edicion),
            Some("modo_funcion") => s.cambiar_modo(sesion::Modo::Funcion),
            Some("ir_a") => match indice() {
                Some(i) => s.ir_a(i),
                None => s.aviso = Some(Texto::nuevo("Cue no válido.", "Invalid cue.")),
            },
            Some("editar_cue") => {
                let forzar = v.get("forzar").and_then(|f| f.as_bool()).unwrap_or(false);
                let r = match (indice(), v.get("cambios")) {
                    (Some(i), Some(c)) => s.editar_cue(i, c, forzar),
                    _ => Err(Texto::nuevo("Cambios no válidos.", "Invalid changes.")),
                };
                if let Err(e) = r {
                    s.aviso = Some(e);
                }
            }
            Some("cerrar_aviso") => s.aviso = None,
            Some("elegir_desde") => {
                // "Salir desde aquí": segundos del archivo, o null = desde el inicio.
                let r = match indice() {
                    Some(i) => s.elegir_desde(i, v.get("s").and_then(|x| x.as_f64())),
                    None => Err(Texto::nuevo("Cue no válido.", "Invalid cue.")),
                };
                if let Err(e) = r {
                    s.aviso = Some(e);
                }
            }
            // Etapa 4C: las 4 esquinas del Proyector ([[x,y] × 4], fracciones de la pantalla).
            Some("esquinas") => {
                let leidas: Option<Vec<[f64; 2]>> = v.get("esquinas").and_then(|e| e.as_array()).map(|a| {
                    a.iter()
                        .filter_map(|p| {
                            let p = p.as_array()?;
                            Some([p.first()?.as_f64()?, p.get(1)?.as_f64()?])
                        })
                        .collect()
                });
                let r = match leidas.and_then(|l| <[[f64; 2]; 4]>::try_from(l).ok()) {
                    Some(e) => s.cambiar_esquinas(e),
                    None => Err(Texto::nuevo("Esquinas no válidas.", "Invalid corners.")),
                };
                if let Err(e) = r {
                    s.aviso = Some(e);
                }
            }
            Some("patron") => {
                if let Err(e) = s.mostrar_patron(v.get("si").and_then(|x| x.as_bool()).unwrap_or(false)) {
                    s.aviso = Some(e);
                }
            }
            Some("idioma") => match v.get("idioma").and_then(|i| i.as_str()).and_then(texto::idioma_de) {
                Some(i) => s.cambiar_idioma(i),
                None => s.aviso = Some(Texto::nuevo("Idioma no válido.", "Invalid language.")),
            },
            Some("go") => {
                s.go();
            }
            Some("parar_todo") => s.parar_todo(),
            Some("pausa") => s.pausa(),
            Some("fundir_todo") => s.fundir_todo(),
            Some("fundir_ultimo") => s.fundir_ultimo(),
            // Panel Sonando (etapa 5C): parar o fundir solo uno (también en Función).
            Some("parar_uno") => match v.get("id").and_then(|x| x.as_u64()) {
                Some(id) => s.parar_uno(id as u32),
                None => s.aviso = Some(Texto::nuevo("Cue no válido.", "Invalid cue.")),
            },
            Some("fundir_uno") => match v.get("id").and_then(|x| x.as_u64()) {
                Some(id) => s.fundir_uno(id as u32),
                None => s.aviso = Some(Texto::nuevo("Cue no válido.", "Invalid cue.")),
            },
            Some("espera_arriba") => s.mover_espera(-1),
            Some("espera_abajo") => s.mover_espera(1),
            Some("guardar") => {
                if let Err(e) = s.guardar() {
                    s.aviso = Some(Texto::nuevo(format!("No se pudo guardar: {e}"), format!("Could not save: {e}")));
                }
            }
            Some("estado") => {}
            _ => {
                s.aviso = Some(Texto::nuevo("Orden desconocida.", "Unknown command."));
            }
        }
        s.estado_json()
    };
    difundir(ctx, estado);
}

fn websocket(stream: TcpStream, ctx: &Arc<Contexto>) {
    let puerto = ctx.puerto;
    let token = ctx.token.clone();
    let revisar = move |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
        let cab = |n: &str| req.headers().get(n).and_then(|v| v.to_str().ok());
        let host_ok = cab("host") == Some(&format!("127.0.0.1:{puerto}"));
        let origen_ok = cab("origin") == Some(&format!("http://127.0.0.1:{puerto}"));
        let token_ok = token_de(req.uri().query()).is_some_and(|t| iguales(t, &token));
        if req.uri().path() == "/ws" && host_ok && origen_ok && token_ok {
            Ok(resp)
        } else {
            let mut e = ErrorResponse::new(Some("Acceso no permitido".into()));
            *e.status_mut() = tungstenite::http::StatusCode::FORBIDDEN;
            Err(e)
        }
    };
    let config = WebSocketConfig::default().max_message_size(Some(MAX_MENSAJE)).max_frame_size(Some(MAX_MENSAJE));
    let Ok(mut ws) = tungstenite::accept_hdr_with_config(stream, revisar, Some(config)) else { return };
    // Lecturas cortas para poder enviar los cambios que lleguen de otras ventanas.
    let _ = ws.get_ref().set_read_timeout(Some(Duration::from_millis(100)));
    let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
    if let Ok(mut c) = ctx.clientes.lock() {
        c.push(tx);
    }
    let inicial = ctx.sesion.lock().map(|s| s.estado_json()).unwrap_or_default();
    if ws.send(Message::text(inicial)).is_err() {
        return;
    }
    loop {
        match ws.read() {
            Ok(Message::Text(t)) => ejecutar(ctx, t.as_str()),
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(ErrorWs::Io(e)) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {}
            Err(_) => break,
        }
        while let Ok(m) = rx.try_recv() {
            if ws.send(Message::text(m)).is_err() {
                return;
            }
        }
    }
}
