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

/// Transiciones de la Vista Estudio con el proceso de proyección de verdad:
/// `--prueba-transiciones [--pantalla N]`. Fundido cruzado (nunca negro, a la
/// mitad la mezcla), blackout (pasa por negro y sale el nuevo), cuchilla, y
/// Parar todo en plena transición (negro al instante, y lo siguiente sale bien).
pub fn transiciones(pantalla: Option<&str>) -> Result<bool, String> {
    use otaconvert_show::{TipoTransicion, Transicion};
    std::env::set_var("OTACONVERT_PRUEBA_VIDEO", "1");
    let lista = crate::pantallas::listar();
    let (x, y, w, h) = match pantalla.and_then(|p| p.parse::<usize>().ok()).and_then(|n| lista.get(n.wrapping_sub(1))) {
        Some(p) => (p.x, p.y, p.ancho, p.alto),
        None => match lista.iter().find(|p| !p.principal) {
            Some(p) => (p.x, p.y, p.ancho, p.alto),
            None => {
                let p = lista.first().ok_or("no hay pantallas")?;
                (p.x + p.ancho / 2 - 320, p.y + p.alto / 2 - 180, 640, 360)
            }
        },
    };
    let (cx, cy) = (x + w / 2, y + h / 2);
    println!("Prueba de transiciones en {w} × {h} (posición {x}, {y}). Se verán colores que se mezclan y pasan por negro.");
    let mut p = ProyectorProceso::abrir(x, y, w, h, false)?;
    if !p.esperar_hola(Duration::from_secs(10)) {
        return Err("la proyección no arrancó".into());
    }
    let rgb = |c: u32| (((c >> 16) & 0xFF) as i32, ((c >> 8) & 0xFF) as i32, (c & 0xFF) as i32);
    let dist = |a: (i32, i32, i32), b: (i32, i32, i32)| (a.0 - b.0).abs().max((a.1 - b.1).abs()).max((a.2 - b.2).abs());
    let mezcla = |a: (i32, i32, i32), b: (i32, i32, i32), t: f64| {
        let m = |x: i32, y: i32| (x as f64 + (y - x) as f64 * t).round() as i32;
        (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
    };
    let mut informe = Vec::new();
    let mut fallos = 0;
    let mut anotar = |ok: bool, t: String| {
        informe.push(format!("{}  {t}", if ok { "BIEN    " } else { "PROBLEMA" }));
        if !ok {
            fallos += 1;
        }
    };
    let cargar = |p: &mut ProyectorProceso, id: u64, c: u32| -> Result<(), String> {
        p.enviar(orden_precargar(id, c));
        esperar(p, Duration::from_secs(5), |a| matches!(a, AvisoProyector::Listo { id: x, .. } if *x == id)).map(|_| ())
    };
    // Mira el centro durante `dur` y cuenta: negras, fuera de lo esperado (según `ok`).
    let mirar = |p: &mut ProyectorProceso, dur: Duration, ok: &dyn Fn((i32, i32, i32)) -> bool| {
        let (mut n, mut negras, mut malas, mut minimo) = (0, 0, 0, 999);
        let mut ejemplos: Vec<(i32, i32, i32)> = Vec::new();
        let t = Instant::now();
        while t.elapsed() < dur {
            let c = pixel(cx, cy);
            n += 1;
            minimo = minimo.min(c.0.max(c.1).max(c.2));
            if negro(c) {
                negras += 1;
            }
            if !ok(c) {
                malas += 1;
                if ejemplos.len() < 9 {
                    ejemplos.push(c);
                }
            }
            let _ = p.recoger();
        }
        if !ejemplos.is_empty() {
            println!("   (muestras fuera de lo esperado: {ejemplos:?})");
        }
        (n, negras, malas, minimo)
    };
    let (a, b, c, d) = (COLORES[0], COLORES[2], COLORES[1], COLORES[3]);

    // Primero A, con corte.
    cargar(&mut p, 1, a)?;
    p.enviar(OrdenProyector::Mostrar { id: 1 });
    std::thread::sleep(Duration::from_millis(400));

    // 1. Fundido cruzado A → B en 1 s.
    cargar(&mut p, 2, b)?;
    p.enviar(OrdenProyector::MostrarCon { id: 2, transicion: Transicion { tipo: TipoTransicion::Fundido, ms: 1000 } });
    let (ra, rb) = (rgb(a), rgb(b));
    let ok_mezcla = |v: (i32, i32, i32)| (0..=20).any(|k| dist(v, mezcla(ra, rb, k as f64 / 20.0)) < 30);
    let (n, negras, malas, _) = mirar(&mut p, Duration::from_millis(500), &ok_mezcla);
    let mitad = pixel(cx, cy);
    let (n2, negras2, malas2, _) = mirar(&mut p, Duration::from_millis(700), &ok_mezcla);
    let fin = pixel(cx, cy);
    anotar(negras + negras2 == 0 && malas + malas2 == 0, format!("fundido 1 s: {} muestras, {} negras, {} fuera de la mezcla; a la mitad {mitad:?} (entre {ra:?} y {rb:?})", n + n2, negras + negras2, malas + malas2));
    anotar(dist(mitad, ra) > 30 && dist(mitad, rb) > 30, "a la mitad del fundido se ven los dos (mezcla)".into());
    anotar(dist(fin, rb) < 30, format!("al terminar el fundido se ve {fin:?} (esperado {rb:?})"));

    // 2. Blackout B → C en 1 s (medio segundo a negro, medio desde negro).
    cargar(&mut p, 3, c)?;
    p.enviar(OrdenProyector::MostrarCon { id: 3, transicion: Transicion { tipo: TipoTransicion::Negro, ms: 1000 } });
    let rc = rgb(c);
    // El brillo de mpv oscurece restando luz (los canales más débiles llegan
    // antes a 0): vale cualquier versión más oscura del color de antes o del nuevo.
    let mas_oscuro = |v: (i32, i32, i32), o: (i32, i32, i32)| v.0 <= o.0 + 15 && v.1 <= o.1 + 15 && v.2 <= o.2 + 15;
    let ok_negro = |v: (i32, i32, i32)| mas_oscuro(v, rb) || mas_oscuro(v, rc);
    let (n, _, malas, minimo) = mirar(&mut p, Duration::from_millis(1200), &ok_negro);
    let fin = pixel(cx, cy);
    anotar(minimo < 24, format!("blackout 1 s: pasa por negro (lo más oscuro: {minimo})"));
    anotar(malas == 0, format!("blackout: {n} muestras, {malas} que no son ni el de antes ni el nuevo oscurecidos"));
    anotar(dist(fin, rc) < 30, format!("al terminar el blackout se ve {fin:?} (esperado {rc:?})"));

    // 3. Cuchilla C → D (control): sin negros.
    cargar(&mut p, 4, d)?;
    let rd = rgb(d);
    p.enviar(OrdenProyector::Mostrar { id: 4 });
    let (n, negras, _, _) = mirar(&mut p, Duration::from_millis(400), &|_| true);
    anotar(negras == 0 && dist(pixel(cx, cy), rd) < 30, format!("cuchilla: {n} muestras, {negras} negras"));

    // 4. Parar todo a mitad de un fundido: negro enseguida; lo siguiente sale bien.
    cargar(&mut p, 5, a)?;
    p.enviar(OrdenProyector::MostrarCon { id: 5, transicion: Transicion { tipo: TipoTransicion::Fundido, ms: 2000 } });
    std::thread::sleep(Duration::from_millis(600));
    p.enviar(OrdenProyector::Negro);
    std::thread::sleep(Duration::from_millis(120));
    let en_negro = pixel(cx, cy);
    cargar(&mut p, 6, b)?;
    p.enviar(OrdenProyector::Mostrar { id: 6 });
    std::thread::sleep(Duration::from_millis(400));
    let despues = pixel(cx, cy);
    anotar(negro(en_negro), format!("Parar todo en pleno fundido: {en_negro:?} a los 120 ms"));
    anotar(dist(despues, rb) < 30, format!("el GO siguiente se ve entero: {despues:?} (esperado {rb:?})"));

    // 5. GO en pleno blackout: salta al final y sale el nuevo.
    cargar(&mut p, 7, c)?;
    p.enviar(OrdenProyector::MostrarCon { id: 7, transicion: Transicion { tipo: TipoTransicion::Negro, ms: 2000 } });
    std::thread::sleep(Duration::from_millis(300));
    cargar(&mut p, 8, d)?;
    p.enviar(OrdenProyector::Mostrar { id: 8 });
    std::thread::sleep(Duration::from_millis(400));
    let ahora = pixel(cx, cy);
    anotar(dist(ahora, rd) < 30, format!("GO en pleno blackout: se ve {ahora:?} (esperado {rd:?})"));

    // 6. Palanca T: D → A a mano. En 0 se ve D, a la mitad la mezcla; volver
    // atrás deja D sin negros ni destellos de A; empujarla hasta el final deja A.
    cargar(&mut p, 9, a)?;
    p.enviar(OrdenProyector::MostrarManual { id: 9 });
    p.enviar(OrdenProyector::Alfa(0.0));
    std::thread::sleep(Duration::from_millis(250));
    let en_cero = pixel(cx, cy);
    anotar(dist(en_cero, rd) < 30, format!("palanca T en 0: se ve {en_cero:?} (esperado lo de antes {rd:?})"));
    let ok_md = |v: (i32, i32, i32)| (0..=20).any(|k| dist(v, mezcla(rd, ra, k as f64 / 20.0)) < 30);
    let mut malas = 0;
    let mut negras = 0;
    for k in 0..=25 {
        p.enviar(OrdenProyector::Alfa(k as f32 / 50.0));
        std::thread::sleep(Duration::from_millis(20));
        let v = pixel(cx, cy);
        negras += negro(v) as i32;
        malas += !ok_md(v) as i32;
    }
    std::thread::sleep(Duration::from_millis(150));
    let mitad = pixel(cx, cy);
    anotar(negras == 0 && malas == 0, format!("palanca T de 0 a la mitad: {negras} negras, {malas} fuera de la mezcla"));
    anotar(dist(mitad, rd) > 30 && dist(mitad, ra) > 30, format!("palanca T a la mitad: {mitad:?} es mezcla de {rd:?} y {ra:?}"));
    // Volver atrás: lo de antes sigue como estaba, sin pasar por A entero ni por negro.
    p.enviar(OrdenProyector::ManualCancelar);
    let (n, negras, malas, _) = mirar(&mut p, Duration::from_millis(400), &|v| dist(v, rd) < 30 || ok_md(v) && dist(v, ra) > 30);
    let atras = pixel(cx, cy);
    anotar(negras == 0 && malas == 0 && dist(atras, rd) < 30, format!("palanca T de vuelta: {n} muestras, {negras} negras, {malas} raras; queda {atras:?}"));
    // Otra vez, ahora hasta el final (A sigue precargado tras volver atrás).
    p.enviar(OrdenProyector::MostrarManual { id: 9 });
    for k in 0..=10 {
        p.enviar(OrdenProyector::Alfa(k as f32 / 10.0));
        std::thread::sleep(Duration::from_millis(30));
    }
    p.enviar(OrdenProyector::ManualFin);
    let (n, negras, _, _) = mirar(&mut p, Duration::from_millis(400), &|_| true);
    let fin = pixel(cx, cy);
    anotar(negras == 0 && dist(fin, ra) < 30, format!("palanca T hasta el final: {n} muestras, {negras} negras; queda {fin:?} (esperado {ra:?})"));
    // Y un GO normal después sigue andando.
    cargar(&mut p, 10, b)?;
    p.enviar(OrdenProyector::MostrarCon { id: 10, transicion: Transicion { tipo: TipoTransicion::Fundido, ms: 500 } });
    std::thread::sleep(Duration::from_millis(800));
    let luego = pixel(cx, cy);
    anotar(dist(luego, rb) < 30, format!("fundido después de la palanca: se ve {luego:?} (esperado {rb:?})"));

    println!();
    for l in &informe {
        println!("{l}");
    }
    println!("RESULTADO: {}", if fallos == 0 { "BIEN" } else { "PROBLEMA" });
    Ok(fallos == 0)
}
