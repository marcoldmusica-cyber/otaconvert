//! Proceso de proyección (etapa 4A): `otaconvert-motor --proyector <x> <y> <ancho> <alto> <completa|vista>`.
//!
//! Es un proceso aparte del motor: si se cae, el sonido sigue y el vigilante
//! del motor lo vuelve a abrir. Recibe órdenes por la entrada estándar (una
//! línea JSON por orden) y avisa por la salida estándar (una línea JSON por
//! aviso). Lo usa `proyector_cliente.rs`.
//!
//! Capas: `CAPAS` reproductores mpv, cada uno en su ventana hija, y encima un
//! **telón** negro. El vídeo nuevo se precarga en pausa debajo del telón; con
//! `mostrar` sube encima de todo y arranca; el anterior se retira un momento
//! después, cuando el nuevo ya tapa. Así nunca hay un cuadro negro entre
//! vídeos y nunca se ve un vídeo antes de su GO.
//!
//! Órdenes (`"o"`):
//! - `precargar {id, ruta, bucle, imagen, inicio, fin}` → `listo {id, duracion}` o `error {id, motivo}`
//! - `mostrar {id}` → `mostrando {id}` o `error {id, motivo}`
//! - `negro` (Parar todo), `fundir {ms}` (a negro), `pausa {si}`, `quitar {id}`, `salir`
//!
//! Avisos (`"a"`): además `fin {id}` (un vídeo sin bucle llegó al final y se
//! queda en su último cuadro) y `latido` cada 100 ms.
//!
//! Etapa 4C:
//! - `precargar` trae además el `encuadre` del cue (modo, zoom, mover, recortes).
//! - `esquinas {h: [8 números]}`: las 4 esquinas del Proyector, con un shader
//!   propio de libplacebo en la etapa final (`SHADER`). Si la tarjeta gráfica
//!   no puede con él, mpv lo apaga solo ("Failed executing hook, disabling") y
//!   la imagen se sigue viendo sin deformar: se avisa `aviso {codigo: "esquinas"}`.
//! - `patron {si}`: un patrón de prueba encima de todo para encajar las esquinas.

use std::io::{BufRead, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use otaconvert_video::mpv::{ruta_junto_al_exe, Evento, Libmpv, Reproductor};
use otaconvert_video::ventana::{Modo, Proyeccion, Rect};
use serde_json::{json, Value};

/// Una capa mostrando, una que se retira y hasta dos precargadas.
pub const CAPAS: usize = 4;
const LATIDO: Duration = Duration::from_millis(100);
/// Cuánto se deja la capa anterior debajo de la nueva antes de soltarla.
const RETIRAR_TRAS: Duration = Duration::from_millis(300);
const LIMITE_CARGA: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq)]
enum Estado {
    Libre,
    /// `buscar`: punto al que saltar al estar cargado (salir desde aquí).
    Cargando { id: u64, desde: Instant, buscar: Option<f64> },
    Lista { id: u64 },
    Mostrando { id: u64, bucle: bool, fin_avisado: bool },
    Retirando { hasta: Instant },
}

struct Capa {
    rep: Reproductor,
    estado: Estado,
    /// Recortes del cue que se carga (arriba, abajo, izquierda, derecha), en
    /// fracciones: se aplican cuando ya se sabe el tamaño de la imagen.
    recorte: [f32; 4],
}

/// Shader de las 4 esquinas (homografía inversa, probado con `--prueba-esquinas`).
pub const SHADER: &str = r#"//!PARAM h0
//!TYPE DYNAMIC float
1.0
//!PARAM h1
//!TYPE DYNAMIC float
0.0
//!PARAM h2
//!TYPE DYNAMIC float
0.0
//!PARAM h3
//!TYPE DYNAMIC float
0.0
//!PARAM h4
//!TYPE DYNAMIC float
1.0
//!PARAM h5
//!TYPE DYNAMIC float
0.0
//!PARAM h6
//!TYPE DYNAMIC float
0.0
//!PARAM h7
//!TYPE DYNAMIC float
0.0

//!HOOK OUTPUT
//!BIND HOOKED
//!DESC Otaconvert: superficie de 4 esquinas
vec4 hook() {
    vec2 p = HOOKED_pos;
    vec3 q = vec3(h0 * p.x + h1 * p.y + h2, h3 * p.x + h4 * p.y + h5, h6 * p.x + h7 * p.y + 1.0);
    if (q.z <= 0.0)
        return vec4(0.0, 0.0, 0.0, 1.0);
    vec2 s = q.xy / q.z;
    if (s.x < 0.0 || s.y < 0.0 || s.x > 1.0 || s.y > 1.0)
        return vec4(0.0, 0.0, 0.0, 1.0);
    return HOOKED_tex(s);
}
"#;

/// Lo que mpv escribe cuando apaga un shader que no pudo usar.
const SHADER_FALLO: &str = "Failed executing hook";

/// Carpeta de datos del programa (como la ventana): junto al exe si es la
/// versión portable, si no `%LOCALAPPDATA%\Otaconvert`.
fn carpeta_datos() -> Option<std::path::PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    if exe_dir.join("portable.txt").is_file() {
        return Some(exe_dir.join("datos"));
    }
    std::env::var_os("LOCALAPPDATA").map(|d| std::path::PathBuf::from(d).join("Otaconvert"))
}

/// Patrón de prueba (1920×1080): cuadrícula, bordes, diagonales, círculo y
/// una esquina de cada color (arriba-izq. roja, arriba-der. verde,
/// abajo-der. azul, abajo-izq. amarilla) para saber cuál se está moviendo.
pub fn escribir_patron(ruta: &std::path::Path) -> std::io::Result<()> {
    let (w, h) = (1920u32, 1080u32);
    let tam = w * 3 * h;
    let mut d = Vec::with_capacity(54 + tam as usize);
    d.extend_from_slice(b"BM");
    d.extend_from_slice(&(54 + tam).to_le_bytes());
    d.extend_from_slice(&0u32.to_le_bytes());
    d.extend_from_slice(&54u32.to_le_bytes());
    d.extend_from_slice(&40u32.to_le_bytes());
    d.extend_from_slice(&(w as i32).to_le_bytes());
    d.extend_from_slice(&(h as i32).to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&24u16.to_le_bytes());
    d.extend_from_slice(&[0u8; 24]);
    let esquina = 160;
    for fila in (0..h).rev() {
        for x in 0..w {
            let y = fila;
            let (fx, fy) = (x as f64 / w as f64, y as f64 / h as f64);
            let borde = x < 6 || y < 6 || x >= w - 6 || y >= h - 6;
            let grilla = x % 240 < 3 || y % 135 < 3;
            let diag = ((fx - fy).abs() * w as f64) < 3.0 || ((fx - (1.0 - fy)).abs() * w as f64) < 3.0;
            let (cx, cy) = (x as f64 - w as f64 / 2.0, y as f64 - h as f64 / 2.0);
            let r = (cx * cx + cy * cy).sqrt();
            let circulo = (r - 400.0).abs() < 3.0;
            let c: (u8, u8, u8) = if x < esquina && y < esquina {
                (220, 50, 50)
            } else if x >= w - esquina && y < esquina {
                (50, 200, 80)
            } else if x >= w - esquina && y >= h - esquina {
                (60, 90, 230)
            } else if x < esquina && y >= h - esquina {
                (230, 200, 40)
            } else if borde || grilla || diag || circulo {
                (240, 240, 240)
            } else {
                (12, 12, 12)
            };
            d.extend_from_slice(&[c.2, c.1, c.0]);
        }
    }
    std::fs::write(ruta, d)
}

/// Opciones de mpv para el encuadre de un cue (todas, para no heredar las del cue anterior).
fn aplicar_encuadre(rep: &Reproductor, e: &Value) -> [f32; 4] {
    let f = |k: &str, d: f64| e[k].as_f64().filter(|v| v.is_finite()).unwrap_or(d);
    let modo = e["modo"].as_str().unwrap_or("ajustar");
    let zoom = f("zoom", 1.0).clamp(0.1, 10.0);
    let _ = rep.propiedad("panscan", if modo == "llenar" { "1.0" } else { "0.0" });
    let _ = rep.propiedad("keepaspect", if modo == "estirar" { "no" } else { "yes" });
    let _ = rep.propiedad("video-zoom", &format!("{:.6}", zoom.log2()));
    let _ = rep.propiedad("video-pan-x", &format!("{:.6}", f("mover_x", 0.0).clamp(-2.0, 2.0)));
    let _ = rep.propiedad("video-pan-y", &format!("{:.6}", f("mover_y", 0.0).clamp(-2.0, 2.0)));
    let _ = rep.propiedad("video-crop", "");
    let r = |k: &str| (f(k, 0.0) as f32).clamp(0.0, 0.45);
    [r("recorte_arriba"), r("recorte_abajo"), r("recorte_izquierda"), r("recorte_derecha")]
}

/// Con la imagen ya cargada (se sabe su tamaño), aplica los recortes.
fn aplicar_recorte(rep: &Reproductor, rec: [f32; 4]) {
    if rec.iter().all(|v| *v <= 0.0) {
        return;
    }
    let n = |k: &str| rep.leer(k).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let (w, h) = (n("video-params/w"), n("video-params/h"));
    if w < 2.0 || h < 2.0 {
        return;
    }
    let (arr, aba, izq, der) = (rec[0] as f64, rec[1] as f64, rec[2] as f64, rec[3] as f64);
    let x = (w * izq).round();
    let y = (h * arr).round();
    let cw = (w * (1.0 - izq - der)).round().max(2.0);
    let ch = (h * (1.0 - arr - aba)).round().max(2.0);
    let _ = rep.propiedad("video-crop", &format!("{cw}x{ch}+{x}+{y}"));
}

/// Fundido a negro en curso: baja el brillo de la capa que se ve.
struct Fundido {
    capa: usize,
    inicio: Instant,
    dur: Duration,
}

fn avisar(v: Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

/// Solo archivos locales con ruta completa (las órdenes vienen del motor,
/// pero igual se comprueba: nada de direcciones de internet ni rutas de red).
/// `av://lavfi:` se permite solo en las pruebas (`OTACONVERT_PRUEBA_VIDEO=1`).
fn ruta_valida(ruta: &str) -> bool {
    if ruta.starts_with("av://lavfi:") {
        return std::env::var_os("OTACONVERT_PRUEBA_VIDEO").is_some();
    }
    let p = std::path::Path::new(ruta);
    p.is_absolute() && !ruta.starts_with("\\\\") && !ruta.starts_with("//") && !ruta.contains("://") && p.is_file()
}

pub fn ejecutar(args: &[String]) -> Result<bool, String> {
    let n = |i: usize| args.get(i).and_then(|s| s.parse::<i32>().ok()).ok_or("faltan x, y, ancho y alto");
    let rect = Rect { x: n(1)?, y: n(2)?, ancho: n(3)?, alto: n(4)? };
    let modo = if args.get(5).map(String::as_str) == Some("vista") { Modo::VistaPrevia } else { Modo::Completa };
    let lib = Libmpv::cargar(&ruta_junto_al_exe()?)?;
    // Una capa más, la última, para el patrón de prueba.
    let proy = Proyeccion::abrir_con(rect, CAPAS + 1, modo)?;
    // El shader de las esquinas y el patrón, escritos en la carpeta de datos
    // del programa. Si no se puede, todo funciona igual pero sin esquinas.
    let datos = carpeta_datos().map(|d| d.join("proyeccion"));
    let shader = datos.as_ref().and_then(|d| {
        std::fs::create_dir_all(d).ok()?;
        let r = d.join("esquinas.glsl");
        std::fs::write(&r, SHADER).ok()?;
        Some(r.to_string_lossy().into_owned())
    });
    let patron = datos.as_ref().and_then(|d| {
        let r = d.join("patron.bmp");
        escribir_patron(&r).ok()?;
        Some(r.to_string_lossy().into_owned())
    });
    let extra: Vec<(&str, &str)> = match &shader {
        Some(s) => vec![("glsl-shaders", s.as_str())],
        None => Vec::new(),
    };
    let mut capas: Vec<Capa> = Vec::with_capacity(CAPAS);
    for i in 0..CAPAS {
        let rep = Reproductor::nuevo(lib.clone(), proy.capa(i), &extra)?;
        // Para notar si la tarjeta gráfica apaga el shader.
        let _ = rep.pedir_mensajes("error");
        capas.push(Capa { rep, estado: Estado::Libre, recorte: [0.0; 4] });
    }
    let mut esquinas_aviso = shader.is_none();
    if shader.is_none() {
        avisar(json!({ "a": "aviso", "codigo": "esquinas" }));
    }
    // El reproductor del patrón se crea solo cuando se pide (y se suelta al
    // quitarlo): así no ocupa memoria durante la función.
    let mut patron_rep: Option<Reproductor> = None;
    let mut opts_esquinas = String::new();

    // Las órdenes llegan por la entrada estándar; si se cierra, el motor ya no está: salir.
    let (tx, rx) = mpsc::channel::<Value>();
    std::thread::Builder::new()
        .name("ordenes".into())
        .spawn(move || {
            for linea in std::io::stdin().lock().lines() {
                let Ok(l) = linea else { break };
                if l.len() > 64 * 1024 {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<Value>(&l) {
                    if tx.send(v).is_err() {
                        return;
                    }
                }
            }
            let _ = tx.send(json!({ "o": "salir" }));
        })
        .map_err(|e| e.to_string())?;

    avisar(json!({ "a": "hola", "capas": CAPAS }));
    let mut fundido: Option<Fundido> = None;
    let mut ultimo_latido = Instant::now();
    loop {
        // 1. Órdenes (espera hasta 5 ms si no hay ninguna).
        let mut primera = rx.recv_timeout(Duration::from_millis(5)).ok();
        while let Some(o) = primera.take().or_else(|| rx.try_recv().ok()) {
            match o["o"].as_str().unwrap_or("") {
                "salir" => return Ok(true),
                "precargar" => precargar(&proy, &mut capas, &o),
                "mostrar" => {
                    let id = o["id"].as_u64().unwrap_or(0);
                    mostrar(&proy, &mut capas, id);
                    fundido = None;
                }
                "negro" => {
                    fundido = None;
                    negro(&proy, &mut capas);
                }
                "fundir" => {
                    let ms = o["ms"].as_u64().unwrap_or(3000).clamp(0, 600_000);
                    if let Some(i) = capas.iter().position(|c| matches!(c.estado, Estado::Mostrando { .. })) {
                        fundido = Some(Fundido { capa: i, inicio: Instant::now(), dur: Duration::from_millis(ms) });
                    }
                }
                "pausa" => {
                    let si = o["si"].as_bool().unwrap_or(true);
                    for c in &capas {
                        if matches!(c.estado, Estado::Mostrando { .. }) {
                            let _ = c.rep.propiedad("pause", if si { "yes" } else { "no" });
                        }
                    }
                }
                "esquinas" => {
                    let h: Vec<f64> = o["h"].as_array().map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
                    if h.len() == 8 && h.iter().all(|v| v.is_finite()) {
                        let opts = (0..8).map(|k| format!("h{k}={:.9}", h[k])).collect::<Vec<_>>().join(",");
                        for c in &capas {
                            let _ = c.rep.propiedad("glsl-shader-opts", &opts);
                        }
                        if let Some(r) = &patron_rep {
                            let _ = r.propiedad("glsl-shader-opts", &opts);
                        }
                        opts_esquinas = opts;
                    }
                }
                "patron" => {
                    let si = o["si"].as_bool().unwrap_or(false);
                    match (&patron, si) {
                        (Some(p), true) => {
                            if patron_rep.is_none() {
                                patron_rep = Reproductor::nuevo(lib.clone(), proy.capa(CAPAS), &extra).ok().and_then(|r| {
                                    let _ = r.propiedad("image-display-duration", "inf");
                                    if !opts_esquinas.is_empty() {
                                        let _ = r.propiedad("glsl-shader-opts", &opts_esquinas);
                                    }
                                    r.precargar(p, LIMITE_CARGA).ok().map(|_| r)
                                });
                            }
                            if let Some(r) = &patron_rep {
                                let _ = r.propiedad("pause", "no");
                                proy.arriba(CAPAS);
                            }
                        }
                        _ => {
                            proy.abajo(CAPAS);
                            patron_rep = None;
                        }
                    }
                }
                "quitar" => {
                    let id = o["id"].as_u64().unwrap_or(0);
                    for c in &mut capas {
                        if matches!(c.estado, Estado::Lista { id: x } | Estado::Cargando { id: x, .. } if x == id) {
                            let _ = c.rep.comando(&["stop"]);
                            c.estado = Estado::Libre;
                        }
                    }
                }
                _ => {}
            }
        }

        // El patrón no avisa nada que interese: sus eventos se descartan.
        if let Some(r) = &patron_rep {
            r.vaciar_eventos();
        }
        // 2. Lo que cuenta mpv de cada capa.
        let ahora = Instant::now();
        for c in &mut capas {
            loop {
                match c.rep.evento(0.0) {
                    Evento::Nada => break,
                    Evento::Listo => {
                        if let Estado::Cargando { id, desde, buscar } = c.estado {
                            match buscar {
                                // Salir desde un punto: se salta ahí (en pausa) y se
                                // espera el primer cuadro de ese punto antes de avisar.
                                Some(s) => {
                                    let _ = c.rep.comando(&["seek", &format!("{s:.3}"), "absolute+exact"]);
                                    c.estado = Estado::Cargando { id, desde, buscar: None };
                                }
                                None => {
                                    aplicar_recorte(&c.rep, c.recorte);
                                    let dur = c.rep.leer("duration").and_then(|d| d.parse::<f64>().ok());
                                    c.estado = Estado::Lista { id };
                                    avisar(json!({ "a": "listo", "id": id, "duracion": dur }));
                                }
                            }
                        }
                    }
                    Evento::Mensaje => {
                        if !esquinas_aviso && c.rep.tomar_mensajes().iter().any(|m| m.contains(SHADER_FALLO)) {
                            esquinas_aviso = true;
                            avisar(json!({ "a": "aviso", "codigo": "esquinas" }));
                        }
                    }
                    Evento::FinArchivo(true) => {
                        if let Estado::Cargando { id, .. } = c.estado {
                            c.estado = Estado::Libre;
                            avisar(json!({ "a": "error", "id": id, "motivo": "abrir" }));
                        }
                    }
                    _ => {}
                }
            }
            match c.estado {
                Estado::Cargando { id, desde, .. } if ahora.duration_since(desde) > LIMITE_CARGA => {
                    let _ = c.rep.comando(&["stop"]);
                    c.estado = Estado::Libre;
                    avisar(json!({ "a": "error", "id": id, "motivo": "tiempo" }));
                }
                Estado::Retirando { hasta } if ahora >= hasta => {
                    let _ = c.rep.comando(&["stop"]);
                    let _ = c.rep.propiedad("brightness", "0");
                    c.estado = Estado::Libre;
                }
                Estado::Mostrando { id, bucle: false, fin_avisado: false } => {
                    if c.rep.leer("eof-reached").as_deref() == Some("yes") {
                        c.estado = Estado::Mostrando { id, bucle: false, fin_avisado: true };
                        avisar(json!({ "a": "fin", "id": id }));
                    }
                }
                _ => {}
            }
        }

        // 3. Fundido a negro: brillo de 0 a −100 y, al final, el telón.
        if let Some(f) = &fundido {
            let t = (ahora.duration_since(f.inicio).as_secs_f64() / f.dur.as_secs_f64().max(0.001)).min(1.0);
            // Curva t²: el brillo de mpv oscurece rápido al principio; así la
            // bajada se ve pareja (a la mitad del tiempo, todavía se ve bien).
            let _ = capas[f.capa].rep.propiedad("brightness", &format!("{:.1}", -100.0 * t * t));
            if t >= 1.0 {
                fundido = None;
                negro(&proy, &mut capas);
            }
        }

        if ultimo_latido.elapsed() >= LATIDO {
            ultimo_latido = Instant::now();
            avisar(json!({ "a": "latido" }));
        }
    }
}

fn precargar(proy: &Proyeccion, capas: &mut [Capa], o: &Value) {
    let id = o["id"].as_u64().unwrap_or(0);
    // Ya está (o se está cargando): no se repite.
    for c in capas.iter() {
        match c.estado {
            Estado::Lista { id: x } if x == id => {
                avisar(json!({ "a": "listo", "id": id, "duracion": c.rep.leer("duration").and_then(|d| d.parse::<f64>().ok()) }));
                return;
            }
            Estado::Cargando { id: x, .. } if x == id => return,
            _ => {}
        }
    }
    let ruta = o["ruta"].as_str().unwrap_or("");
    if !ruta_valida(ruta) {
        avisar(json!({ "a": "error", "id": id, "motivo": "ruta" }));
        return;
    }
    // La última capa es la del patrón de prueba: no se usa para los cues.
    let Some(i) = capas[..CAPAS].iter().position(|c| c.estado == Estado::Libre) else {
        avisar(json!({ "a": "error", "id": id, "motivo": "capas" }));
        return;
    };
    let c = &mut capas[i];
    let bucle = o["bucle"].as_bool().unwrap_or(false);
    let imagen = o["imagen"].as_bool().unwrap_or(false);
    let inicio = o["inicio"].as_f64().unwrap_or(0.0).max(0.0);
    let fin = o["fin"].as_f64().filter(|f| *f > inicio);
    proy.abajo(i);
    c.rep.vaciar_eventos();
    c.recorte = aplicar_encuadre(&c.rep, &o["encuadre"]);
    let ok = (|| -> Result<(), String> {
        c.rep.propiedad("pause", "yes")?;
        c.rep.propiedad("brightness", "0")?;
        c.rep.propiedad("loop-file", if bucle { "inf" } else { "no" })?;
        c.rep.propiedad("image-display-duration", if imagen { "inf" } else { "1" })?;
        c.rep.propiedad("start", &format!("{inicio:.3}"))?;
        c.rep.propiedad("end", &fin.map(|f| format!("{f:.3}")).unwrap_or_else(|| "none".into()))?;
        c.rep.comando(&["loadfile", ruta, "replace"])
    })();
    match ok {
        Ok(()) => c.estado = Estado::Cargando { id, desde: Instant::now(), buscar: o["desde"].as_f64().filter(|s| s.is_finite() && *s > inicio) },
        Err(e) => avisar(json!({ "a": "error", "id": id, "motivo": e })),
    }
}

fn mostrar(proy: &Proyeccion, capas: &mut [Capa], id: u64) {
    // El mismo cue que ya se ve: vuelve a empezar.
    if let Some(c) = capas.iter_mut().find(|c| matches!(c.estado, Estado::Mostrando { id: x, .. } if x == id)) {
        // A su inicio (el `start` con que se precargó), no al segundo 0 del
        // archivo: así la imagen va con el sonido, que vuelve al inicio del cue.
        let inicio = c.rep.leer("start").and_then(|s| s.trim().parse::<f64>().ok()).filter(|s| s.is_finite() && *s >= 0.0).unwrap_or(0.0);
        let _ = c.rep.comando(&["seek", &format!("{inicio:.3}"), "absolute+exact"]);
        let _ = c.rep.propiedad("brightness", "0");
        let _ = c.rep.propiedad("pause", "no");
        avisar(json!({ "a": "mostrando", "id": id }));
        return;
    }
    let Some(i) = capas.iter().position(|c| c.estado == (Estado::Lista { id })) else {
        avisar(json!({ "a": "error", "id": id, "motivo": "no_listo" }));
        return;
    };
    // La nueva sube encima de todo (también del telón) y arranca…
    proy.arriba(i);
    let bucle = capas[i].rep.leer("loop-file").is_some_and(|v| v != "no");
    let _ = capas[i].rep.propiedad("pause", "no");
    capas[i].estado = Estado::Mostrando { id, bucle, fin_avisado: false };
    // …y la anterior se retira cuando la nueva ya la tapa.
    let hasta = Instant::now() + RETIRAR_TRAS;
    for (k, c) in capas.iter_mut().enumerate() {
        if k != i && matches!(c.estado, Estado::Mostrando { .. }) {
            let _ = c.rep.propiedad("pause", "yes");
            c.estado = Estado::Retirando { hasta };
        }
    }
    avisar(json!({ "a": "mostrando", "id": id }));
}

/// Parar todo: el telón tapa todo al instante y se sueltan las capas que se veían.
/// Las precargadas siguen listas para el próximo GO.
fn negro(proy: &Proyeccion, capas: &mut [Capa]) {
    proy.telon_arriba();
    let hasta = Instant::now() + RETIRAR_TRAS;
    for c in capas.iter_mut() {
        if matches!(c.estado, Estado::Mostrando { .. }) {
            let _ = c.rep.propiedad("pause", "yes");
            c.estado = Estado::Retirando { hasta };
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::ruta_valida;

    #[test]
    fn solo_archivos_locales() {
        let exe = std::env::current_exe().unwrap();
        assert!(ruta_valida(&exe.to_string_lossy()));
        for mala in ["", "relativa/video.mp4", "\\\\servidor\\c\\v.mp4", "//servidor/v.mp4", "https://x.com/v.mp4", "C:\\no-existe\\v.mp4"] {
            assert!(!ruta_valida(mala), "{mala}");
        }
        if std::env::var_os("OTACONVERT_PRUEBA_VIDEO").is_none() {
            assert!(!ruta_valida("av://lavfi:color=c=red"));
        }
    }
}
