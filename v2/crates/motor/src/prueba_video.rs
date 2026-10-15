//! Prueba automática de vídeo sin negros (Fase 0).
//!
//! Dos capas con un reproductor mpv cada una. Cada "cue" es un color liso
//! generado por mpv (sin archivos). En cada GO la capa de abajo, ya cargada
//! en pausa, sube y arranca; después la de arriba anterior carga el color
//! siguiente en pausa. Mientras tanto se mira el píxel central de la pantalla
//! todo el tiempo: cualquier negro, o un color que no sea el anterior ni el
//! nuevo, cuenta como fallo.

use std::path::Path;
use std::time::{Duration, Instant};

use otaconvert_video::mpv::{ruta_junto_al_exe, Evento, Libmpv, Reproductor};
use otaconvert_video::ventana::{Proyeccion, Rect};

use crate::pantallas::{self, Pantalla};

/// Colores de prueba (0xRRGGBB), ninguno oscuro, cada uno distinto del siguiente.
const COLORES: [u32; 6] = [0xD04848, 0x48B060, 0x4868D0, 0xD0B848, 0xB848B8, 0x48B8B8];
const TOLERANCIA: i32 = 48;
const LIMITE_CARGA: Duration = Duration::from_secs(5);
const LIMITE_CAMBIO: Duration = Duration::from_secs(1);

fn canales(rgb: u32) -> (i32, i32, i32) {
    (((rgb >> 16) & 0xFF) as i32, ((rgb >> 8) & 0xFF) as i32, (rgb & 0xFF) as i32)
}

/// COLORREF de Windows (0x00BBGGRR) a (r, g, b).
fn canales_pantalla(c: u32) -> (i32, i32, i32) {
    ((c & 0xFF) as i32, ((c >> 8) & 0xFF) as i32, ((c >> 16) & 0xFF) as i32)
}

fn parecido(pantalla: u32, rgb: u32) -> bool {
    let (a, b) = (canales_pantalla(pantalla), canales(rgb));
    (a.0 - b.0).abs() <= TOLERANCIA && (a.1 - b.1).abs() <= TOLERANCIA && (a.2 - b.2).abs() <= TOLERANCIA
}

fn es_negro(pantalla: u32) -> bool {
    let (r, g, b) = canales_pantalla(pantalla);
    r < 32 && g < 32 && b < 32
}

#[derive(Default)]
struct Cuenta {
    muestras: u64,
    negros: u64,
    raros: u64,
    sin_cambio: u64,
    latencias_ms: Vec<f64>,
    /// Los primeros colores raros vistos (r, g, b), para entender qué pasó.
    ejemplos_raros: Vec<(i32, i32, i32)>,
}

impl Cuenta {
    /// Mira la pantalla una vez. `valido` son los colores aceptables ahora.
    fn mirar(&mut self, p: &Proyeccion, valido: &[u32]) -> u32 {
        let c = p.color_centro();
        self.muestras += 1;
        if es_negro(c) {
            self.negros += 1;
        } else if !valido.iter().any(|&v| parecido(c, v)) {
            self.raros += 1;
            if self.ejemplos_raros.len() < 5 {
                self.ejemplos_raros.push(canales_pantalla(c));
            }
        }
        c
    }
}

fn elegir_pantalla(pedida: Option<&str>) -> Result<Pantalla, String> {
    let lista = pantallas::listar();
    if lista.is_empty() {
        return Err("no hay pantallas".into());
    }
    let nombres = || lista.iter().enumerate().map(|(n, p)| format!("{}. {}", n + 1, p.nombre)).collect::<Vec<_>>().join("\n  ");
    let Some(p) = pedida else {
        // Sin elegir: la primera que no es la principal (el proyector), o la principal si es la única.
        return Ok(lista.iter().find(|p| !p.principal).unwrap_or(&lista[0]).clone());
    };
    if let Ok(n) = p.parse::<usize>() {
        return lista.get(n.wrapping_sub(1)).cloned().ok_or_else(|| format!("no existe la pantalla {n}. Pantallas:\n  {}", nombres()));
    }
    let p_min = p.to_lowercase();
    let coinciden: Vec<&Pantalla> = lista
        .iter()
        .filter(|x| x.dispositivo.eq_ignore_ascii_case(p) || x.nombre.to_lowercase().contains(&p_min))
        .collect();
    match coinciden.as_slice() {
        [x] => Ok((*x).clone()),
        [] => Err(format!("no se encontró la pantalla \"{p}\". Pantallas:\n  {}", nombres())),
        _ => Err(format!("\"{p}\" coincide con varias pantallas; usa el número. Pantallas:\n  {}", nombres())),
    }
}

/// Cue de prueba de color `rgb`: un archivo `RRGGBB.*` de `carpeta` (vídeo real,
/// con el centro de ese color) o, sin carpeta, un color liso generado por mpv.
fn fuente(rgb: u32, r: &Rect, carpeta: Option<&Path>) -> Result<String, String> {
    let Some(dir) = carpeta else {
        return Ok(format!("av://lavfi:color=c=0x{rgb:06X}:s={}x{}:r=60", r.ancho, r.alto));
    };
    let nombre = format!("{rgb:06X}");
    let leidos = std::fs::read_dir(dir).map_err(|e| format!("no se pudo leer {}: {e}", dir.display()))?;
    for e in leidos.flatten() {
        let p = e.path();
        if p.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case(&nombre)) {
            return Ok(p.to_string_lossy().into_owned());
        }
    }
    Err(format!("falta un vídeo llamado {nombre}.* en {}", dir.display()))
}

pub fn ejecutar(cambios: u64, pantalla: Option<&str>, carpeta: Option<&Path>) -> Result<bool, String> {
    let p = elegir_pantalla(pantalla)?;
    let rect = Rect { x: p.x, y: p.y, ancho: p.ancho, alto: p.alto };
    let lib = Libmpv::cargar(&ruta_junto_al_exe()?)?;
    println!("libmpv: {} (API {}.{})", lib.ruta.display(), lib.version.0, lib.version.1);
    println!("Pantalla: {} · {} × {} a {} Hz · {}", p.nombre, p.ancho, p.alto, p.hz, p.gpu);
    match carpeta {
        Some(d) => println!("Vídeos de prueba: {}", d.display()),
        None => println!("Vídeos de prueba: colores lisos generados por mpv"),
    }
    println!("Prueba de {cambios} cambios. La pantalla se llenará de colores; no hace falta tocar nada.");

    let proy = Proyeccion::abrir(rect, 2)?;
    let reps = [
        Reproductor::nuevo(lib.clone(), proy.capa(0), &[])?,
        Reproductor::nuevo(lib.clone(), proy.capa(1), &[])?,
    ];

    // Primer cue en la capa 0, a la vista; el segundo precargado en la capa 1.
    reps[0].precargar(&fuente(COLORES[0], &rect, carpeta)?, LIMITE_CARGA)?;
    proy.arriba(0);
    reps[0].propiedad("pause", "no")?;
    reps[1].precargar(&fuente(COLORES[1], &rect, carpeta)?, LIMITE_CARGA)?;
    std::thread::sleep(Duration::from_millis(400));
    let visto = proy.color_centro();
    if !parecido(visto, COLORES[0]) {
        return Err(format!(
            "no se ve el primer color en la pantalla (se ve {:?}); no se puede medir",
            canales_pantalla(visto)
        ));
    }

    let mut cuenta = Cuenta::default();
    let mut arriba = 0usize;
    let inicio = Instant::now();
    for i in 1..=cambios as usize {
        let nueva = 1 - arriba;
        let anterior = COLORES[(i - 1) % COLORES.len()];
        let esperado = COLORES[i % COLORES.len()];

        // GO: la capa precargada sube y arranca.
        let t0 = Instant::now();
        proy.arriba(nueva);
        reps[nueva].propiedad("pause", "no")?;
        let mut visto: Option<Duration> = None;
        loop {
            let c = cuenta.mirar(&proy, &[anterior, esperado]);
            let t = t0.elapsed();
            if visto.is_none() && parecido(c, esperado) {
                visto = Some(t);
            }
            match visto {
                Some(v) if t > v + Duration::from_millis(80) => break,
                None if t > LIMITE_CAMBIO => {
                    cuenta.sin_cambio += 1;
                    break;
                }
                _ => {}
            }
        }
        if let Some(v) = visto {
            cuenta.latencias_ms.push(v.as_secs_f64() * 1000.0);
        }

        // La capa que quedó abajo carga el cue siguiente en pausa. Se sigue
        // mirando: si se viera negro aquí, las capas no se tapan bien.
        let siguiente = COLORES[(i + 1) % COLORES.len()];
        let vieja = &reps[arriba];
        vieja.vaciar_eventos();
        vieja.propiedad("pause", "yes")?;
        vieja.comando(&["loadfile", &fuente(siguiente, &rect, carpeta)?, "replace"])?;
        let t1 = Instant::now();
        loop {
            cuenta.mirar(&proy, &[esperado]);
            match vieja.evento(0.0) {
                Evento::Listo => break,
                Evento::FinArchivo(true) | Evento::Apagado => return Err("mpv falló al precargar".into()),
                _ => {}
            }
            if t1.elapsed() > LIMITE_CARGA {
                return Err("la precarga no terminó a tiempo".into());
            }
        }
        for _ in 0..5 {
            cuenta.mirar(&proy, &[esperado]);
        }
        arriba = nueva;
    }
    let duracion = inicio.elapsed();

    let lat = &cuenta.latencias_ms;
    let media = if lat.is_empty() { 0.0 } else { lat.iter().sum::<f64>() / lat.len() as f64 };
    let maxima = lat.iter().cloned().fold(0.0, f64::max);
    println!("— Informe —");
    println!("Cambios: {cambios} en {:.1} s", duracion.as_secs_f64());
    println!(
        "Muestras de pantalla: {} ({:.0} por segundo)",
        cuenta.muestras,
        cuenta.muestras as f64 / duracion.as_secs_f64()
    );
    println!("Muestras negras: {}", cuenta.negros);
    println!("Muestras de otro color: {}", cuenta.raros);
    if !cuenta.ejemplos_raros.is_empty() {
        println!("  Ejemplos (r, g, b): {:?}", cuenta.ejemplos_raros);
    }
    println!("Cambios que no se vieron en 1 s: {}", cuenta.sin_cambio);
    println!("Tiempo del GO a la imagen nueva: media {media:.1} ms · máxima {maxima:.1} ms");
    for (n, r) in reps.iter().enumerate() {
        println!(
            "Capa {n}: salida {} · cuadros perdidos {} · decodificación {}",
            r.leer("current-vo").unwrap_or_else(|| "?".into()),
            r.leer("frame-drop-count").unwrap_or_else(|| "?".into()),
            r.leer("hwdec-current").filter(|s| !s.is_empty() && s != "no").unwrap_or_else(|| "CPU".into()),
        );
    }
    let ok = cuenta.negros == 0 && cuenta.raros == 0 && cuenta.sin_cambio == 0;
    println!("{}", if ok { "RESULTADO: BIEN" } else { "RESULTADO: HAY PROBLEMAS" });
    drop(reps);
    drop(proy);
    Ok(ok)
}

/// Reproducción continua de un vídeo en bucle durante `segundos`, contando
/// cuadros perdidos. Mide si el equipo aguanta ese vídeo (p. ej. 4K60) sin cortes.
pub fn reproduccion(segundos: u64, archivo: &Path, pantalla: Option<&str>) -> Result<bool, String> {
    if !archivo.is_file() {
        return Err(format!("no existe el vídeo {}", archivo.display()));
    }
    let p = elegir_pantalla(pantalla)?;
    let rect = Rect { x: p.x, y: p.y, ancho: p.ancho, alto: p.alto };
    let lib = Libmpv::cargar(&ruta_junto_al_exe()?)?;
    println!("Pantalla: {} · {} × {} a {} Hz · {}", p.nombre, p.ancho, p.alto, p.hz, p.gpu);
    println!("Reproduciendo {} durante {segundos} s…", archivo.display());
    let proy = Proyeccion::abrir(rect, 1)?;
    let rep = Reproductor::nuevo(lib, proy.capa(0), &[("loop-file", "inf")])?;
    rep.precargar(&archivo.to_string_lossy(), LIMITE_CARGA)?;
    rep.propiedad("pause", "no")?;
    std::thread::sleep(Duration::from_secs(segundos));
    let leer = |n: &str| rep.leer(n).unwrap_or_else(|| "?".into());
    let num = |n: &str| rep.leer(n).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    let perdidos = num("frame-drop-count");
    let perdidos_dec = num("decoder-frame-drop-count");
    let atrasados = num("vo-delayed-frame-count");
    println!("— Informe —");
    println!("Vídeo: {} × {} a {} cuadros/s · {}", leer("video-params/w"), leer("video-params/h"), leer("container-fps"), leer("video-codec"));
    println!("Decodificación: {}", rep.leer("hwdec-current").filter(|s| !s.is_empty() && s != "no").unwrap_or_else(|| "CPU".into()));
    println!("Cuadros por segundo medidos: {}", leer("estimated-vf-fps"));
    println!("Cuadros perdidos al mostrar: {perdidos}");
    println!("Cuadros perdidos al decodificar: {perdidos_dec}");
    println!("Cuadros mostrados tarde: {atrasados}");
    let ok = perdidos == 0 && perdidos_dec == 0;
    println!("{}", if ok { "RESULTADO: BIEN" } else { "RESULTADO: HAY PROBLEMAS" });
    drop(rep);
    drop(proy);
    Ok(ok)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn colores_validos_y_distintos() {
        for (i, &c) in COLORES.iter().enumerate() {
            let (r, g, b) = canales(c);
            assert!(r.max(g).max(b) > 150, "color {i} demasiado oscuro");
            let sig = COLORES[(i + 1) % COLORES.len()];
            // Pantalla con el color c en formato COLORREF.
            let pantalla = ((b as u32) << 16) | ((g as u32) << 8) | r as u32;
            assert!(parecido(pantalla, c));
            assert!(!parecido(pantalla, sig), "colores {i} y siguiente se confunden");
            assert!(!es_negro(pantalla));
        }
        assert!(es_negro(0));
    }
}
