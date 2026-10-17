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
    let proy = Proyeccion::abrir_con(rect, CAPAS, modo)?;
    let mut capas: Vec<Capa> = Vec::with_capacity(CAPAS);
    for i in 0..CAPAS {
        capas.push(Capa { rep: Reproductor::nuevo(lib.clone(), proy.capa(i), &[])?, estado: Estado::Libre });
    }

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
                                    let dur = c.rep.leer("duration").and_then(|d| d.parse::<f64>().ok());
                                    c.estado = Estado::Lista { id };
                                    avisar(json!({ "a": "listo", "id": id, "duracion": dur }));
                                }
                            }
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
    let Some(i) = capas.iter().position(|c| c.estado == Estado::Libre) else {
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
