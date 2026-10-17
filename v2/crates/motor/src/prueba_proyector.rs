//! Prueba automática del proceso de proyección de verdad (etapa 4A):
//! `otaconvert-motor --prueba-proyector [cambios] [--pantalla X]`.
//!
//! Usa colores lisos generados por mpv y mira el píxel central de la
//! proyección en la pantalla. Comprueba:
//! 1. Cambios entre cues sin ningún negro ni color raro.
//! 2. Nada se ve antes de su GO (precargar con el telón arriba sigue en negro).
//! 3. Parar todo (negro) y Fundir terminan en negro.
//! 4. Si el proceso se cae, vuelve en negro y avisa cuánto tardó.
//!
//! Con una sola pantalla, la prueba usa un recuadro en el centro de la pantalla.

use std::time::{Duration, Instant};

use otaconvert_servidor::{AvisoProyector, OrdenProyector, Proyector};

use crate::proyector_cliente::ProyectorProceso;

const COLORES: [u32; 6] = [0xD04848, 0x48B060, 0x4868D0, 0xD0B848, 0xB848B8, 0x48B8B8];
const TOLERANCIA: i32 = 48;

fn pixel(x: i32, y: i32) -> (i32, i32, i32) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
        // SAFETY: DC de toda la pantalla, liberado enseguida.
        let c = unsafe {
            let dc = GetDC(std::ptr::null_mut());
            let c = GetPixel(dc, x, y);
            ReleaseDC(std::ptr::null_mut(), dc);
            c
        };
        ((c & 0xFF) as i32, ((c >> 8) & 0xFF) as i32, ((c >> 16) & 0xFF) as i32)
    }
    #[cfg(not(windows))]
    {
        let _ = (x, y);
        (0, 0, 0)
    }
}

fn parecido(p: (i32, i32, i32), rgb: u32) -> bool {
    let (r, g, b) = (((rgb >> 16) & 0xFF) as i32, ((rgb >> 8) & 0xFF) as i32, (rgb & 0xFF) as i32);
    (p.0 - r).abs() <= TOLERANCIA && (p.1 - g).abs() <= TOLERANCIA && (p.2 - b).abs() <= TOLERANCIA
}

fn negro(p: (i32, i32, i32)) -> bool {
    p.0 < 32 && p.1 < 32 && p.2 < 32
}

fn orden_precargar(id: u64, rgb: u32) -> OrdenProyector {
    OrdenProyector::Precargar {
        id,
        ruta: format!("av://lavfi:color=c=0x{rgb:06X}:s=640x360:r=60").into(),
        bucle: true,
        imagen: false,
        inicio_s: 0.0,
        desde_s: None,
        encuadre: Default::default(),
        fin_s: None,
    }
}

/// Espera un aviso que cumpla `f`, hasta `limite`.
fn esperar(p: &mut ProyectorProceso, limite: Duration, f: impl Fn(&AvisoProyector) -> bool) -> Result<AvisoProyector, String> {
    let fin = Instant::now() + limite;
    while Instant::now() < fin {
        for a in p.recoger() {
            if let AvisoProyector::Error { motivo, .. } | AvisoProyector::Caido { motivo } = &a {
                return Err(motivo.es.clone());
            }
            if f(&a) {
                return Ok(a);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err("la proyección no respondió a tiempo".into())
}

/// ¿El vídeo se mueve en el proyector? `--prueba-movimiento --archivo <vídeo>`:
/// precarga el vídeo, le da GO y mira 16 puntos de la imagen cada 200 ms
/// durante 3 s. Si la imagen no cambia, el vídeo quedó congelado.
pub fn movimiento(archivo: &std::path::Path) -> Result<bool, String> {
    let ruta = std::path::absolute(archivo).map_err(|e| format!("{}: {e}", archivo.display()))?;
    let lista = crate::pantallas::listar();
    let p = lista.first().ok_or("no hay pantallas")?;
    let (x, y, w, h) = (p.x + p.ancho / 2 - 320, p.y + p.alto / 2 - 180, 640, 360);
    println!("Prueba de movimiento: {} en un recuadro de 640 × 360 en el centro de la pantalla.", ruta.display());
    let mut pr = ProyectorProceso::abrir(x, y, w, h, false)?;
    if !pr.esperar_hola(Duration::from_secs(10)) {
        return Err("la proyección no arrancó".into());
    }
    pr.enviar(OrdenProyector::Precargar { id: 1, ruta, bucle: true, imagen: false, inicio_s: 0.0, fin_s: None, desde_s: None, encuadre: Default::default() });
    esperar(&mut pr, Duration::from_secs(10), |a| matches!(a, AvisoProyector::Listo { id: 1, .. }))?;
    pr.enviar(OrdenProyector::Mostrar { id: 1 });
    std::thread::sleep(Duration::from_millis(500));
    let puntos: Vec<(i32, i32)> = (0..16).map(|k| (x + 20 + (k % 4) * 150, y + 30 + (k / 4) * 90)).collect();
    let foto = || puntos.iter().map(|&(a, b)| pixel(a, b)).collect::<Vec<_>>();
    let mut anterior = foto();
    let mut cambios = 0;
    for _ in 0..15 {
        std::thread::sleep(Duration::from_millis(200));
        let _ = pr.recoger();
        let ahora = foto();
        if ahora != anterior {
            cambios += 1;
        }
        anterior = ahora;
    }
    println!("Muestras en que la imagen cambió: {cambios} de 15");
    let ok = cambios >= 10;
    println!("{}", if ok { "RESULTADO: BIEN (el vídeo se mueve)" } else { "RESULTADO: HAY PROBLEMAS (el vídeo parece congelado)" });
    Ok(ok)
}

pub fn ejecutar(cambios: u64, pantalla: Option<&str>) -> Result<bool, String> {
    // Solo en esta prueba la proyección acepta colores generados por mpv.
    std::env::set_var("OTACONVERT_PRUEBA_VIDEO", "1");
    let lista = crate::pantallas::listar();
    let (x, y, w, h) = match pantalla.and_then(|p| p.parse::<usize>().ok()).and_then(|n| lista.get(n.wrapping_sub(1))) {
        Some(p) => (p.x, p.y, p.ancho, p.alto),
        None => match lista.iter().find(|p| !p.principal) {
            Some(p) => (p.x, p.y, p.ancho, p.alto),
            // Una sola pantalla: un recuadro de 640×360 en el centro.
            None => {
                let p = lista.first().ok_or("no hay pantallas")?;
                (p.x + p.ancho / 2 - 320, p.y + p.alto / 2 - 180, 640, 360)
            }
        },
    };
    let (cx, cy) = (x + w / 2, y + h / 2);
    println!("Proyección de prueba en {w} × {h} (posición {x}, {y}). Se verán colores; no hace falta tocar nada.");

    let mut p = ProyectorProceso::abrir(x, y, w, h, false)?;
    if !p.esperar_hola(Duration::from_secs(10)) {
        return Err("la proyección no arrancó".into());
    }
    std::thread::sleep(Duration::from_millis(300));
    let mut fallos = 0u32;
    let mut informe = Vec::new();

    // 2. Al abrir y mientras precarga, todo negro (el telón tapa).
    p.enviar(orden_precargar(0, COLORES[0]));
    let mut vistos_antes = 0;
    let t = Instant::now();
    let mut listo = false;
    while t.elapsed() < Duration::from_millis(800) {
        if !negro(pixel(cx, cy)) {
            vistos_antes += 1;
        }
        if !listo {
            listo = p.recoger().iter().any(|a| matches!(a, AvisoProyector::Listo { id: 0, .. }));
        }
    }
    if !listo {
        esperar(&mut p, Duration::from_secs(5), |a| matches!(a, AvisoProyector::Listo { id: 0, .. }))?;
    }
    informe.push(format!("Antes del primer GO (precargando): muestras no negras {vistos_antes}"));
    if vistos_antes > 0 {
        fallos += 1;
    }

    // 1. Cambios sin negros.
    let (mut negros, mut raros, mut muestras, mut sin_cambio) = (0u64, 0u64, 0u64, 0u64);
    let mut latencias = Vec::new();
    p.enviar(OrdenProyector::Mostrar { id: 0 });
    std::thread::sleep(Duration::from_millis(300));
    p.enviar(orden_precargar(1, COLORES[1]));
    esperar(&mut p, Duration::from_secs(5), |a| matches!(a, AvisoProyector::Listo { id: 1, .. }))?;
    for i in 1..=cambios as usize {
        let (anterior, nuevo) = (COLORES[(i - 1) % COLORES.len()], COLORES[i % COLORES.len()]);
        let t0 = Instant::now();
        p.enviar(OrdenProyector::Mostrar { id: i as u64 });
        let mut visto = None;
        loop {
            let c = pixel(cx, cy);
            muestras += 1;
            if negro(c) {
                negros += 1;
            } else if !parecido(c, anterior) && !parecido(c, nuevo) {
                raros += 1;
            }
            if visto.is_none() && parecido(c, nuevo) {
                visto = Some(t0.elapsed());
            }
            match visto {
                Some(v) if t0.elapsed() > v + Duration::from_millis(80) => break,
                None if t0.elapsed() > Duration::from_secs(1) => {
                    sin_cambio += 1;
                    break;
                }
                _ => {}
            }
            let _ = p.recoger();
        }
        if let Some(v) = visto {
            latencias.push(v.as_secs_f64() * 1000.0);
        }
        // El siguiente se precarga mientras se sigue mirando (no debe verse).
        p.enviar(orden_precargar(i as u64 + 1, COLORES[(i + 1) % COLORES.len()]));
        let fin = Instant::now() + Duration::from_secs(5);
        let mut ok = false;
        while Instant::now() < fin && !ok {
            let c = pixel(cx, cy);
            muestras += 1;
            if negro(c) {
                negros += 1;
            } else if !parecido(c, nuevo) {
                raros += 1;
            }
            ok = p.recoger().iter().any(|a| matches!(a, AvisoProyector::Listo { id, .. } if *id == i as u64 + 1));
        }
        if !ok {
            return Err("la precarga no terminó a tiempo".into());
        }
    }
    let media = if latencias.is_empty() { 0.0 } else { latencias.iter().sum::<f64>() / latencias.len() as f64 };
    let maxima = latencias.iter().cloned().fold(0.0, f64::max);
    informe.push(format!(
        "Cambios: {cambios} · muestras {muestras} · negras {negros} · de otro color {raros} · no vistos en 1 s {sin_cambio}"
    ));
    informe.push(format!("Tiempo del GO a la imagen: media {media:.1} ms · máxima {maxima:.1} ms"));
    if negros > 0 || raros > 0 || sin_cambio > 0 {
        fallos += 1;
    }

    // 3a. Parar todo: negro enseguida.
    let t = Instant::now();
    p.enviar(OrdenProyector::Negro);
    let mut ms_negro = None;
    while t.elapsed() < Duration::from_millis(500) {
        if negro(pixel(cx, cy)) {
            ms_negro = Some(t.elapsed().as_secs_f64() * 1000.0);
            break;
        }
    }
    informe.push(format!("Parar todo → negro en {}", ms_negro.map(|m| format!("{m:.0} ms")).unwrap_or("NO".into())));
    if ms_negro.is_none() {
        fallos += 1;
    }

    // 3b. Fundir en 1 s: a los 1,3 s, negro.
    let siguiente = cambios + 1;
    p.enviar(OrdenProyector::Mostrar { id: siguiente });
    std::thread::sleep(Duration::from_millis(400));
    let antes_fundir = pixel(cx, cy);
    p.enviar(OrdenProyector::Fundir { ms: 1000 });
    std::thread::sleep(Duration::from_millis(500));
    let medio = pixel(cx, cy);
    std::thread::sleep(Duration::from_millis(800));
    let _ = p.recoger();
    let despues = pixel(cx, cy);
    let fundio = !negro(antes_fundir) && negro(despues);
    informe.push(format!(
        "Fundir 1 s: antes {antes_fundir:?} · a la mitad {medio:?} · al final {despues:?} → {}",
        if fundio { "bien" } else { "NO" }
    ));
    if !fundio {
        fallos += 1;
    }

    // 4. Caída: vuelve en negro.
    p.enviar(orden_precargar(900, COLORES[0]));
    esperar(&mut p, Duration::from_secs(5), |a| matches!(a, AvisoProyector::Listo { id: 900, .. }))?;
    p.enviar(OrdenProyector::Mostrar { id: 900 });
    std::thread::sleep(Duration::from_millis(300));
    p.matar_para_probar();
    let t = Instant::now();
    let r = esperar(&mut p, Duration::from_secs(5), |a| matches!(a, AvisoProyector::Reiniciado { .. }));
    let total = t.elapsed();
    std::thread::sleep(Duration::from_millis(300));
    let tras = pixel(cx, cy);
    match r {
        Ok(AvisoProyector::Reiniciado { ms }) => {
            informe.push(format!(
                "Caída: de nuevo abierta en {} ms (desde que se notó: {ms} ms) · se ve {}",
                total.as_millis(),
                if negro(tras) { "negro (bien)" } else { "otra cosa" }
            ));
            if !negro(tras) || total > Duration::from_secs(1) {
                fallos += 1;
            }
        }
        _ => {
            informe.push("Caída: NO volvió".into());
            fallos += 1;
        }
    }

    println!("— Informe —");
    for l in &informe {
        println!("{l}");
    }
    println!("{}", if fallos == 0 { "RESULTADO: BIEN" } else { "RESULTADO: HAY PROBLEMAS" });
    Ok(fallos == 0)
}
