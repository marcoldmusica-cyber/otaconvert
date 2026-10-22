//! Experimento antes de las transiciones de la Vista Estudio:
//! `otaconvert-motor --prueba-cruzado [--carpeta <vídeos RRGGBB.mp4>] [--pantalla N]`.
//!
//! ¿Se puede hacer un **fundido cruzado de imagen** (el vídeo nuevo aparece
//! encima mientras el de abajo todavía se ve) con las capas de siempre?
//! Idea: la capa de arriba como ventana hija «en capas» (`WS_EX_LAYERED`, que
//! Windows admite en ventanas hijas desde Windows 8) con su transparencia
//! (`SetLayeredWindowAttributes`, `LWA_ALPHA`) subiendo de 0 a 255.
//! Se prueba con la forma de dibujar de mpv por defecto (d3d11 «flip») y con
//! `d3d11-flip=no`, porque no está documentado cuál se lleva bien con eso.
//!
//! Mide en la pantalla de verdad (GetPixel): que a mitad del fundido se vea la
//! mezcla de los dos colores, que nunca se vea negro y, con vídeos reales,
//! que no se pierdan cuadros mientras funde ni después.

use std::path::Path;
use std::time::{Duration, Instant};

use otaconvert_video::mpv::{ruta_junto_al_exe, Libmpv, Reproductor};
use otaconvert_video::ventana::{Proyeccion, Rect};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED};

use crate::prueba_esquinas::{bmp, pixel};

fn transparencia(h: isize, alfa: Option<u8>) -> bool {
    // SAFETY: HWND de una capa viva de la proyección; solo cambia su estilo y su transparencia.
    unsafe {
        let hw = h as windows_sys::Win32::Foundation::HWND;
        let estilo = GetWindowLongPtrW(hw, GWL_EXSTYLE);
        match alfa {
            Some(a) => {
                SetWindowLongPtrW(hw, GWL_EXSTYLE, estilo | WS_EX_LAYERED as isize);
                SetLayeredWindowAttributes(hw, 0, a, LWA_ALPHA) != 0
            }
            None => {
                SetWindowLongPtrW(hw, GWL_EXSTYLE, estilo & !(WS_EX_LAYERED as isize));
                true
            }
        }
    }
}

fn mezcla(a: (i32, i32, i32), b: (i32, i32, i32), t: f64) -> (i32, i32, i32) {
    let m = |x: i32, y: i32| (x as f64 + (y - x) as f64 * t).round() as i32;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

fn distancia(a: (i32, i32, i32), b: (i32, i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs()).max((a.2 - b.2).abs())
}

fn es_negro(p: (i32, i32, i32)) -> bool {
    p.0 < 24 && p.1 < 24 && p.2 < 24
}

fn color_de(nombre: &str) -> Option<(i32, i32, i32)> {
    let s = Path::new(nombre).file_stem()?.to_str()?;
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(((v >> 16) as i32 & 0xFF, (v >> 8) as i32 & 0xFF, v as i32 & 0xFF))
}

fn perdidos(r: &Reproductor) -> i64 {
    let n = |p: &str| r.leer(p).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    n("frame-drop-count") + n("decoder-frame-drop-count")
}

pub fn ejecutar(carpeta: Option<&Path>, pantalla: Option<&str>) -> Result<bool, String> {
    let lista = crate::pantallas::listar();
    let p = match pantalla.and_then(|s| s.parse::<usize>().ok()) {
        Some(n) => lista.get(n.wrapping_sub(1)).cloned().ok_or("esa pantalla no existe")?,
        None => lista.iter().find(|p| !p.principal).or(lista.first()).cloned().ok_or("no hay pantallas")?,
    };
    let (x, y, w, h) = (p.x, p.y, p.ancho, p.alto);
    let (cx, cy) = (x + w / 2, y + h / 2);
    println!("Prueba de fundido cruzado en {} ({w} × {h}, pantalla completa). Se verán colores que se mezclan.", p.nombre);

    let dir = std::env::temp_dir().join("otaconvert-prueba-cruzado");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let (rojo, azul) = ((208, 72, 72), (72, 104, 208));
    let f_rojo = dir.join("rojo.bmp");
    let f_azul = dir.join("azul.bmp");
    bmp(&f_rojo, 1920, 1080, |_, _| (208, 72, 72))?;
    bmp(&f_azul, 1920, 1080, |_, _| (72, 104, 208))?;

    let lib = Libmpv::cargar(&ruta_junto_al_exe()?)?;
    let mut informe = Vec::new();
    let mut fallos = 0;
    let mut anotar = |ok: bool, t: String| {
        informe.push(format!("{}  {t}", if ok { "BIEN    " } else { "PROBLEMA" }));
        if !ok {
            fallos += 1;
        }
    };

    for flip in ["yes"] {
        let modo = if flip == "yes" { "d3d11 flip (por defecto)" } else { "d3d11-flip=no" };
        let proy = Proyeccion::abrir(Rect { x, y, ancho: w, alto: h }, 2)?;
        let opts: &[(&str, &str)] = &[("image-display-duration", "inf"), ("d3d11-flip", flip)];
        let abajo = Reproductor::nuevo(lib.clone(), proy.capa(0), opts)?;
        let arriba = Reproductor::nuevo(lib.clone(), proy.capa(1), opts)?;
        abajo.precargar(&f_rojo.to_string_lossy(), Duration::from_secs(10))?;
        arriba.precargar(&f_azul.to_string_lossy(), Duration::from_secs(10))?;
        proy.arriba(0);
        abajo.propiedad("pause", "no")?;
        std::thread::sleep(Duration::from_millis(500));

        // La capa de arriba entra transparente: tiene que seguir viéndose el rojo.
        let ok_alfa = transparencia(proy.capa(1), Some(0));
        proy.arriba(1);
        arriba.propiedad("pause", "no")?;
        std::thread::sleep(Duration::from_millis(500));
        let antes = pixel(cx, cy);
        anotar(ok_alfa && distancia(antes, rojo) < 30, format!("[{modo}] capa nueva transparente encima: se ve {antes:?} (rojo {rojo:?})"));

        // Fundido de 1 s en 40 pasos: cada muestra contra la mezcla esperada.
        let (mut negros, mut lejos, mut peor) = (0, 0, 0);
        let mut mitad = (0, 0, 0);
        for paso in 1..=40 {
            let t = paso as f64 / 40.0;
            transparencia(proy.capa(1), Some((t * 255.0).round() as u8));
            std::thread::sleep(Duration::from_millis(25));
            let v = pixel(cx, cy);
            // La pantalla va un cuadro atrás: vale la mezcla de este paso o la del anterior.
            let d = distancia(v, mezcla(rojo, azul, t)).min(distancia(v, mezcla(rojo, azul, (paso - 1) as f64 / 40.0)));
            peor = peor.max(d);
            if es_negro(v) {
                negros += 1;
            }
            if d > 30 {
                lejos += 1;
            }
            if paso == 20 {
                mitad = v;
            }
        }
        std::thread::sleep(Duration::from_millis(200));
        let fin = pixel(cx, cy);
        anotar(negros == 0 && lejos == 0, format!("[{modo}] fundido de 1 s en 40 pasos: a la mitad {mitad:?} (esperado ≈ {:?}); {negros} negros, {lejos} lejos de la mezcla (peor {peor})", mezcla(rojo, azul, 0.5)));
        anotar(distancia(fin, azul) < 30, format!("[{modo}] al terminar se ve {fin:?} (azul {azul:?})"));

        // Quitar la transparencia al terminar (como se haría en el programa): sin parpadeo.
        transparencia(proy.capa(1), None);
        let (mut malos, mut n) = (0, 0);
        let hasta = Instant::now() + Duration::from_millis(500);
        while Instant::now() < hasta {
            if distancia(pixel(cx, cy), azul) >= 30 {
                malos += 1;
            }
            n += 1;
        }
        anotar(malos == 0, format!("[{modo}] al quitar la transparencia: {malos} de {n} muestras distintas del azul"));
        drop(arriba);
        drop(abajo);
        drop(proy);
        std::thread::sleep(Duration::from_millis(300));
    }

    // Con vídeos reales: 6 fundidos de 1 s entre dos vídeos que se mueven.
    let videos: Vec<_> = carpeta
        .and_then(|c| std::fs::read_dir(c).ok())
        .map(|d| d.flatten().map(|e| e.path()).filter(|p| color_de(&p.to_string_lossy()).is_some()).collect())
        .unwrap_or_default();
    if videos.len() >= 2 {
        // Tres maneras, 6 cambios cada una, contando los cuadros perdidos en
        // cada momento: «corte» es como el programa hoy (control).
        for variante in ["corte (como hoy)", "fundido, quitando la transparencia al final", "fundido, dejando la capa transparente"] {
            let proy = Proyeccion::abrir(Rect { x, y, ancho: w, alto: h }, 2)?;
            let opts: &[(&str, &str)] = &[("loop-file", "inf")];
            let reps = [Reproductor::nuevo(lib.clone(), proy.capa(0), opts)?, Reproductor::nuevo(lib.clone(), proy.capa(1), opts)?];
            let fundir = !variante.starts_with("corte");
            let quitar = variante.contains("quitando");
            reps[0].precargar(&videos[0].to_string_lossy(), Duration::from_secs(10))?;
            proy.arriba(0);
            reps[0].propiedad("pause", "no")?;
            std::thread::sleep(Duration::from_millis(800));
            let (mut negros, mut muestras, mut lejos) = (0, 0, 0);
            // Cuadros perdidos por momento: al precargar, al arrancar/fundir, al terminar, después.
            let mut fases = [0i64; 4];
            let mut arriba_i = 0usize;
            let total = |r: &[Reproductor; 2]| perdidos(&r[0]) + perdidos(&r[1]);
            for k in 1..=6usize {
                let nuevo = 1 - arriba_i;
                let (c_viejo, c_nuevo) = (color_de(&videos[(k - 1) % videos.len()].to_string_lossy()).unwrap(), color_de(&videos[k % videos.len()].to_string_lossy()).unwrap());
                let a = total(&reps);
                reps[nuevo].precargar(&videos[k % videos.len()].to_string_lossy(), Duration::from_secs(10))?;
                std::thread::sleep(Duration::from_millis(300));
                let b = total(&reps);
                if fundir && (quitar || k <= 2) {
                    transparencia(proy.capa(nuevo), Some(0));
                }
                proy.arriba(nuevo);
                reps[nuevo].propiedad("pause", "no")?;
                let inicio = Instant::now();
                let dura = if fundir { 1000 } else { 300 };
                while inicio.elapsed() < Duration::from_millis(dura) {
                    if fundir {
                        let t = inicio.elapsed().as_secs_f64();
                        transparencia(proy.capa(nuevo), Some((t * 255.0).min(255.0) as u8));
                    }
                    let v = pixel(cx, cy);
                    muestras += 1;
                    if es_negro(v) {
                        negros += 1;
                    }
                    let d = (0..=10).map(|j| distancia(v, mezcla(c_viejo, c_nuevo, j as f64 / 10.0))).min().unwrap_or(999);
                    if d > 40 {
                        lejos += 1;
                    }
                }
                if fundir {
                    transparencia(proy.capa(nuevo), Some(255));
                }
                let c = total(&reps);
                if fundir && quitar {
                    transparencia(proy.capa(nuevo), None);
                }
                let _ = reps[arriba_i].propiedad("pause", "yes");
                std::thread::sleep(Duration::from_millis(300));
                let d = total(&reps);
                std::thread::sleep(Duration::from_millis(1200));
                let e = total(&reps);
                // Al cargar otro archivo mpv puede volver sus cuentas a 0: solo se suman subidas.
                for (i, (x0, x1)) in [(a, b), (b, c), (c, d), (d, e)].into_iter().enumerate() {
                    fases[i] += (x1 - x0).max(0);
                }
                arriba_i = nuevo;
            }
            let hw = |r: &Reproductor| r.leer("hwdec-current").unwrap_or_default();
            anotar(negros == 0 && lejos == 0, format!("[{variante}] vídeos reales, 6 cambios: {muestras} muestras, {negros} negras, {lejos} fuera de la mezcla"));
            anotar(fases.iter().sum::<i64>() == 0, format!("[{variante}] cuadros perdidos: al precargar {}, al arrancar o fundir {}, al terminar {}, después {} (decodificación: {} / {})", fases[0], fases[1], fases[2], fases[3], hw(&reps[0]), hw(&reps[1])));
            drop(reps);
            drop(proy);
            std::thread::sleep(Duration::from_millis(300));
        }
    } else {
        informe.push("NO MEDIDO  vídeos reales (falta --carpeta con vídeos RRGGBB.mp4)".into());
    }

    println!();
    for l in &informe {
        println!("{l}");
    }
    println!("RESULTADO: {}", if fallos == 0 { "BIEN" } else { "PROBLEMA" });
    Ok(fallos == 0)
}
