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

pub use sesion::Sesion;
pub use sonido::{Destino, Sonido};
pub use texto::{Idioma, Texto};
pub use proyector::{AvisoProyector, OrdenProyector, Proyector};

const PLANTILLA: &str = include_str!("../ui/index.html");
/// Textos de la interfaz en español e inglés (también los consejos y la
/// ayuda). Se prueban en `tests/textos.rs`.
pub const TEXTOS: &str = include_str!("../ui/textos.json");

/// La página con los textos dentro (se arma una vez). `</` se escapa para
/// que un texto nunca pueda cerrar el `<script>`.
fn pagina() -> &'static str {
    static P: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    P.get_or_init(|| PLANTILLA.replace("\"__TEXTOS__\"", &TEXTOS.replace("</", "<\\/")))
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
}

pub struct Servidor {
    pub puerto: u16,
    token: String,
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
                Some((i, ruta, fs)) => {
                    let r = otaconvert_audio::preparar::preparar(&ruta, fs);
                    let estado = c3.sesion.lock().ok().map(|mut s| {
                        s.resultado_preparar(i, r);
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
        Ok(Self { puerto, token })
    }

    /// Dirección que abre la ventana (o el navegador).
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/?t={}", self.puerto, self.token)
    }
}

fn atender(stream: TcpStream, ctx: &Contexto) {
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
                Some(a) => enviar_archivo(&stream, &a, cabecera("range")),
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
fn enviar_archivo(mut s: &TcpStream, ruta: &std::path::Path, rango: Option<&str>) {
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
    let comunes = "Accept-Ranges: bytes\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n";
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

fn ejecutar(ctx: &Contexto, texto: &str) {
    let v = serde_json::from_str::<serde_json::Value>(texto).unwrap_or_default();
    let orden = v.get("orden").and_then(|o| o.as_str()).map(str::to_string);
    let indice = || v.get("cue").and_then(|c| c.as_u64()).and_then(|c| usize::try_from(c).ok());
    let estado = {
        let Ok(mut s) = ctx.sesion.lock() else { return };
        match orden.as_deref() {
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

fn websocket(stream: TcpStream, ctx: &Contexto) {
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
