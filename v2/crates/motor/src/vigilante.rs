//! Vigilante (Fase 0): un proceso pequeño que arranca el motor, escucha su
//! "latido" y, si el motor se cae o se cuelga, lo vuelve a arrancar y le dice
//! por dónde iba el sonido. Objetivo: menos de 1 s sin sonido.
//!
//! Protocolo por la salida estándar del motor hijo (una línea cada vez):
//! - `LISTO`: la salida de audio ya está sonando.
//! - `LATIDO <ms>`: sigue vivo; posición del sonido en milisegundos.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use otaconvert_audio::prueba::Azar;
use otaconvert_audio::{Comando, Detector, Pista};

/// Sin latido durante este tiempo = colgado.
const SIN_LATIDO: Duration = Duration::from_millis(300);
const LATIDO: Duration = Duration::from_millis(100);
const LIMITE_ARRANQUE: Duration = Duration::from_secs(5);

/// Que un fallo cierre el proceso enseguida, sin ventanas de error de Windows
/// que lo dejen abierto esperando a que alguien toque "Aceptar".
pub fn sin_ventanas_de_error() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Diagnostics::Debug::{
            SetErrorMode, SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX, SEM_NOOPENFILEERRORBOX,
        };
        // SAFETY: solo cambia el modo de errores de este proceso.
        unsafe { SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX) };
    }
}

/// Modo hijo: abre la salida, suena un tono suave en bucle desde `desde_ms`,
/// avisa LISTO y late cada 100 ms. Con `fallo` se cae o se cuelga a propósito.
pub fn hijo(salida: Option<&str>, desde_ms: u64, fallo: Option<(String, u64)>) -> Result<bool, String> {
    sin_ventanas_de_error();
    let mut s = crate::abrir(50, salida, |fs| Detector::umbral_para_tonos(0.05, fs as f32 / 100.0, fs))?;
    let fs = s.frecuencia;
    // Tono de periodo exacto (100 cuadros) y largo de 1 s: el bucle es continuo.
    let largo = 100 * (fs as usize / 100);
    let pista = Arc::new(Pista::tono(fs as f32 / 100.0, 0.05, largo, 1, fs));
    let desde = ((desde_ms * fs as u64 / 1000) as usize) % largo;
    s.control
        .enviar(Comando::Iniciar { id: 1, pista, volumen: 1.0, desde, bucle: true, fundido: 0 })
        .map_err(|_| "cola llena")?;
    let espera = Instant::now();
    while s.medidas.llamadas.load(Relaxed) < 2 {
        if espera.elapsed() > LIMITE_ARRANQUE {
            return Err("la salida de audio no arrancó".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut out = std::io::stdout();
    let _ = writeln!(out, "LISTO");
    let inicio = Instant::now();
    loop {
        std::thread::sleep(LATIDO);
        let ms = desde_ms + inicio.elapsed().as_millis() as u64;
        let _ = writeln!(out, "LATIDO {ms}");
        let _ = out.flush();
        if let Some((tipo, en)) = &fallo {
            if inicio.elapsed() >= Duration::from_millis(*en) {
                if tipo == "cuelgue" {
                    // Sigue vivo pero ya no late: el vigilante debe notarlo.
                    loop {
                        std::thread::sleep(Duration::from_secs(3600));
                    }
                }
                std::process::abort();
            }
        }
        s.control.recoger_basura();
    }
}

enum Linea {
    Listo(Instant),
    Latido(u64, Instant),
    Fin,
}

fn lanzar(salida: Option<&str>, desde_ms: u64, fallo: Option<(&str, u64)>) -> Result<(Child, Receiver<Linea>), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut c = Command::new(exe);
    c.arg("--hijo-audio").arg(desde_ms.to_string());
    if let Some(s) = salida {
        c.arg("--salida").arg(s);
    }
    if let Some((tipo, en)) = fallo {
        c.arg("--fallo").arg(tipo).arg(en.to_string());
    }
    let mut hijo = c.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
    let stdout = hijo.stdout.take().ok_or("sin salida del hijo")?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for l in BufReader::new(stdout).lines() {
            let Ok(l) = l else { break };
            let ahora = Instant::now();
            let msg = if l == "LISTO" {
                Linea::Listo(ahora)
            } else if let Some(ms) = l.strip_prefix("LATIDO ").and_then(|m| m.parse().ok()) {
                Linea::Latido(ms, ahora)
            } else {
                continue;
            };
            if tx.send(msg).is_err() {
                return;
            }
        }
        let _ = tx.send(Linea::Fin);
    });
    Ok((hijo, rx))
}

fn esperar_listo(rx: &Receiver<Linea>) -> Result<Instant, String> {
    let fin = Instant::now() + LIMITE_ARRANQUE;
    loop {
        match rx.recv_timeout(fin.saturating_duration_since(Instant::now())) {
            Ok(Linea::Listo(t)) => return Ok(t),
            Ok(Linea::Fin) | Err(RecvTimeoutError::Disconnected) => return Err("el motor se cerró al arrancar".into()),
            Err(RecvTimeoutError::Timeout) => return Err("el motor no arrancó a tiempo".into()),
            Ok(Linea::Latido(..)) => {}
        }
    }
}

struct Resultado {
    tipo: &'static str,
    deteccion_ms: f64,
    arranque_ms: f64,
}

/// Prueba: `ciclos` fallos a propósito (caída y cuelgue alternados).
pub fn prueba(ciclos: u64, salida: Option<&str>) -> Result<bool, String> {
    sin_ventanas_de_error();
    println!("Prueba del vigilante: {ciclos} fallos a propósito (caídas y cuelgues). Se oirá un tono suave.");
    let mut azar = Azar::nuevo(7);
    let mut pos_ms = 0u64;
    let mut resultados: Vec<Resultado> = Vec::new();
    let mut pendiente: Option<(&'static str, Instant, f64)> = None;

    for ciclo in 0..=ciclos {
        let tipo: &'static str = if ciclo % 2 == 0 { "caída" } else { "cuelgue" };
        let ultimo_ciclo = ciclo == ciclos;
        let falla_en = 1500 + azar.hasta(1500);
        let fallo = (!ultimo_ciclo).then_some((if tipo == "caída" { "caida" } else { "cuelgue" }, falla_en));
        // Si venimos de un fallo, el sonido debe seguir donde iba.
        let desde = match &pendiente {
            Some((_, detectado, _)) => pos_ms + detectado.elapsed().as_millis() as u64,
            None => pos_ms,
        };
        let (mut hijo, rx) = lanzar(salida, desde, fallo)?;
        let listo = esperar_listo(&rx)?;
        if let Some((t, detectado, det_ms)) = pendiente.take() {
            let arranque = listo.duration_since(detectado).as_secs_f64() * 1000.0;
            resultados.push(Resultado { tipo: t, deteccion_ms: det_ms, arranque_ms: arranque });
            println!(
                "  {}: detectado en {:.0} ms, sonando de nuevo en {:.0} ms · total {:.0} ms",
                t,
                det_ms,
                arranque,
                det_ms + arranque
            );
        }
        if ultimo_ciclo {
            std::thread::sleep(Duration::from_millis(500));
            let _ = hijo.kill();
            let _ = hijo.wait();
            break;
        }

        // Vigilar hasta el fallo.
        let mut ultimo = listo;
        loop {
            match rx.recv_timeout(Duration::from_millis(20)) {
                Ok(Linea::Latido(ms, t)) => {
                    pos_ms = ms;
                    ultimo = t;
                }
                Ok(_) | Err(_) => {}
            }
            let caido = hijo.try_wait().map_err(|e| e.to_string())?.is_some();
            let colgado = ultimo.elapsed() > SIN_LATIDO;
            if caido || colgado {
                if !caido {
                    let _ = hijo.kill();
                    let _ = hijo.wait();
                }
                let detectado = Instant::now();
                // El fallo ocurre justo después del último latido.
                let det_ms = detectado.duration_since(ultimo).as_secs_f64() * 1000.0;
                pendiente = Some((tipo, detectado, det_ms));
                break;
            }
        }
    }

    let total = |r: &Resultado| r.deteccion_ms + r.arranque_ms;
    let maximo = resultados.iter().map(total).fold(0.0, f64::max);
    let media = resultados.iter().map(total).sum::<f64>() / resultados.len().max(1) as f64;
    println!("— Informe —");
    println!("Fallos provocados: {}", resultados.len());
    for t in ["caída", "cuelgue"] {
        let de: Vec<&Resultado> = resultados.iter().filter(|r| r.tipo == t).collect();
        if !de.is_empty() {
            let m = de.iter().map(|r| total(r)).fold(0.0, f64::max);
            println!("  {t}: {} · peor caso {:.0} ms sin sonido", de.len(), m);
        }
    }
    println!("Tiempo sin sonido: media {media:.0} ms · máximo {maximo:.0} ms (objetivo < 1000 ms)");
    let ok = resultados.len() as u64 == ciclos && maximo < 1000.0;
    println!("{}", if ok { "RESULTADO: BIEN" } else { "RESULTADO: HAY PROBLEMAS" });
    Ok(ok)
}
