//! Experimento de la etapa 4C (antes de decidir cómo hacerla):
//! `otaconvert-motor --prueba-esquinas [--carpeta <vídeos RRGGBB.mp4>] [--pantalla N]`.
//!
//! 1. **4 esquinas** con un shader propio de mpv/libplacebo en la etapa final
//!    (`OUTPUT`): para cada píxel de la salida calcula de qué punto de la imagen
//!    viene (homografía inversa, 8 números) y fuera de la forma pinta negro.
//!    Los 8 números se pasan con `glsl-shader-opts` (documentado en el manual
//!    de mpv; que se pueda cambiar en vivo no está documentado: se mide aquí).
//! 2. **Encuadre** con opciones de mpv documentadas: `panscan` (llenar
//!    recortando), `keepaspect` (estirar), `video-zoom`, `video-pan-x`,
//!    `video-crop` (el manual dice que funciona con decodificación por GPU).
//!
//! Mide en la pantalla de verdad (GetPixel) que la imagen quede donde tiene
//! que quedar, que mover las esquinas en vivo no dé negros, y con un vídeo real
//! que la tarjeta gráfica siga decodificando y no se pierdan cuadros.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use otaconvert_video::mpv::{ruta_junto_al_exe, Libmpv, Reproductor};
use otaconvert_video::ventana::{Proyeccion, Rect};

const SHADER: &str = r#"//!PARAM h0
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

/// Homografía del cuadrado unidad a un cuadrilátero (esquinas en orden:
/// arriba-izquierda, arriba-derecha, abajo-derecha, abajo-izquierda), y su
/// inversa normalizada (lo que usa el shader: de la salida a la imagen).
pub fn inversa_para(esq: [(f64, f64); 4]) -> Option<[f64; 8]> {
    let [(x0, y0), (x1, y1), (x2, y2), (x3, y3)] = esq;
    let (dx1, dx2, dx3) = (x1 - x2, x3 - x2, x0 - x1 + x2 - x3);
    let (dy1, dy2, dy3) = (y1 - y2, y3 - y2, y0 - y1 + y2 - y3);
    let (g, h) = if dx3.abs() < 1e-12 && dy3.abs() < 1e-12 {
        (0.0, 0.0)
    } else {
        let den = dx1 * dy2 - dx2 * dy1;
        if den.abs() < 1e-12 {
            return None;
        }
        ((dx3 * dy2 - dx2 * dy3) / den, (dx1 * dy3 - dx3 * dy1) / den)
    };
    let m = [
        [x1 - x0 + g * x1, x3 - x0 + h * x3, x0],
        [y1 - y0 + g * y1, y3 - y0 + h * y3, y0],
        [g, h, 1.0],
    ];
    // Inversa por adjunta.
    let c = |r: usize, k: usize| m[r][k];
    let det = c(0, 0) * (c(1, 1) * c(2, 2) - c(1, 2) * c(2, 1)) - c(0, 1) * (c(1, 0) * c(2, 2) - c(1, 2) * c(2, 0))
        + c(0, 2) * (c(1, 0) * c(2, 1) - c(1, 1) * c(2, 0));
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = [
        [c(1, 1) * c(2, 2) - c(1, 2) * c(2, 1), c(0, 2) * c(2, 1) - c(0, 1) * c(2, 2), c(0, 1) * c(1, 2) - c(0, 2) * c(1, 1)],
        [c(1, 2) * c(2, 0) - c(1, 0) * c(2, 2), c(0, 0) * c(2, 2) - c(0, 2) * c(2, 0), c(0, 2) * c(1, 0) - c(0, 0) * c(1, 2)],
        [c(1, 0) * c(2, 1) - c(1, 1) * c(2, 0), c(0, 1) * c(2, 0) - c(0, 0) * c(2, 1), c(0, 0) * c(1, 1) - c(0, 1) * c(1, 0)],
    ];
    let n = inv[2][2];
    if n.abs() < 1e-12 {
        return None;
    }
    Some([inv[0][0] / n, inv[0][1] / n, inv[0][2] / n, inv[1][0] / n, inv[1][1] / n, inv[1][2] / n, inv[2][0] / n, inv[2][1] / n])
}

/// ¿El punto (x, y) está dentro del cuadrilátero, con un margen?
fn dentro(esq: [(f64, f64); 4], x: f64, y: f64, margen: f64) -> Option<bool> {
    let mut signo = 0.0;
    let mut min_dist = f64::MAX;
    for k in 0..4 {
        let (ax, ay) = esq[k];
        let (bx, by) = esq[(k + 1) % 4];
        let cruz = (bx - ax) * (y - ay) - (by - ay) * (x - ax);
        let largo = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
        min_dist = min_dist.min((cruz / largo).abs());
        if signo == 0.0 {
            signo = cruz.signum();
        } else if cruz.signum() != signo {
            return if min_dist < margen { None } else { Some(false) };
        }
    }
    if min_dist < margen {
        None
    } else {
        Some(true)
    }
}

fn opts(h: [f64; 8]) -> String {
    (0..8).map(|k| format!("h{k}={:.9}", h[k])).collect::<Vec<_>>().join(",")
}

fn bmp(ruta: &Path, w: u32, h: u32, color: impl Fn(u32, u32) -> (u8, u8, u8)) -> Result<(), String> {
    let fila = w * 3;
    let tam = fila * h;
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
    // BMP va de abajo hacia arriba.
    for fy in (0..h).rev() {
        for x in 0..w {
            let (r, g, b) = color(x, fy);
            d.extend_from_slice(&[b, g, r]);
        }
    }
    std::fs::write(ruta, d).map_err(|e| format!("{}: {e}", ruta.display()))
}

fn pixel(x: i32, y: i32) -> (i32, i32, i32) {
    use windows_sys::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
    // SAFETY: DC de la pantalla, liberado enseguida.
    let c = unsafe {
        let dc = GetDC(std::ptr::null_mut());
        let c = GetPixel(dc, x, y);
        ReleaseDC(std::ptr::null_mut(), dc);
        c
    };
    ((c & 0xFF) as i32, ((c >> 8) & 0xFF) as i32, ((c >> 16) & 0xFF) as i32)
}

fn es_negro(p: (i32, i32, i32)) -> bool {
    p.0 < 40 && p.1 < 40 && p.2 < 40
}
fn es_claro(p: (i32, i32, i32)) -> bool {
    p.0 > 200 && p.1 > 200 && p.2 > 200
}
fn parecido(p: (i32, i32, i32), c: (i32, i32, i32)) -> bool {
    (p.0 - c.0).abs() < 48 && (p.1 - c.1).abs() < 48 && (p.2 - c.2).abs() < 48
}

pub fn ejecutar(carpeta: Option<&Path>, pantalla: Option<&str>) -> Result<bool, String> {
    let lista = crate::pantallas::listar();
    let p = match pantalla.and_then(|s| s.parse::<usize>().ok()) {
        Some(n) => lista.get(n.wrapping_sub(1)).cloned().ok_or("esa pantalla no existe")?,
        None => lista.iter().find(|p| !p.principal).or(lista.first()).cloned().ok_or("no hay pantallas")?,
    };
    let (x, y, w, h) = (p.x, p.y, p.ancho, p.alto);
    let en = |u: f64, v: f64| (x + (u * w as f64) as i32, y + (v * h as f64) as i32);
    println!("Prueba de esquinas y encuadre en {} ({w} × {h}, pantalla completa). Se verán formas blancas y colores.", p.nombre);

    let dir = std::env::temp_dir().join("otaconvert-prueba-esquinas");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let shader = dir.join("esquinas.glsl");
    std::fs::write(&shader, SHADER).map_err(|e| e.to_string())?;
    let blanco = dir.join("blanco.bmp");
    bmp(&blanco, 1920, 1080, |_, _| (255, 255, 255))?;
    // 4:3, mitad izquierda roja y mitad derecha azul (para ver bandas, zoom, mover y recortar).
    let mitades = dir.join("mitades-4x3.bmp");
    bmp(&mitades, 1440, 1080, |px, _| if px < 720 { (208, 72, 72) } else { (72, 104, 208) })?;
    let (rojo, azul) = ((208, 72, 72), (72, 104, 208));

    let lib = Libmpv::cargar(&ruta_junto_al_exe()?)?;
    let proy = Proyeccion::abrir(Rect { x, y, ancho: w, alto: h }, 1)?;
    let shader_s = shader.to_string_lossy().into_owned();
    let rep = Reproductor::nuevo(lib.clone(), proy.capa(0), &[("image-display-duration", "inf"), ("loop-file", "inf"), ("glsl-shaders", &shader_s)])?;
    rep.precargar(&blanco.to_string_lossy(), Duration::from_secs(10))?;
    proy.arriba(0);
    rep.propiedad("pause", "no")?;
    std::thread::sleep(Duration::from_millis(600));

    let mut informe = Vec::new();
    let mut fallos = 0;
    let mut anotar = |ok: bool, t: String| {
        informe.push(format!("{}  {t}", if ok { "BIEN    " } else { "PROBLEMA" }));
        if !ok {
            fallos += 1;
        }
    };

    // 1. Sin deformar: toda la pantalla blanca.
    let identidad = inversa_para([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]).ok_or("identidad")?;
    rep.propiedad("glsl-shader-opts", &opts(identidad))?;
    std::thread::sleep(Duration::from_millis(400));
    let puntos9: Vec<(f64, f64)> = [0.05, 0.5, 0.95].iter().flat_map(|&u| [0.05, 0.5, 0.95].map(move |v| (u, v))).collect();
    let mal = puntos9.iter().filter(|&&(u, v)| !es_claro({ let (a, b) = en(u, v); pixel(a, b) })).count();
    anotar(mal == 0, format!("Sin deformar (shader cargado): pantalla entera blanca · puntos mal {mal} de 9"));

    // 2. Una forma torcida: dentro blanco, fuera negro (grilla de 11 × 11 puntos).
    let forma = [(0.12, 0.08), (0.80, 0.18), (0.92, 0.88), (0.20, 0.95)];
    let hforma = inversa_para(forma).ok_or("forma imposible")?;
    rep.propiedad("glsl-shader-opts", &opts(hforma))?;
    std::thread::sleep(Duration::from_millis(400));
    let (mut revisados, mut mal) = (0, Vec::new());
    for i in 0..11 {
        for j in 0..11 {
            let (u, v) = (0.03 + 0.094 * i as f64, 0.03 + 0.094 * j as f64);
            let Some(esperado) = dentro(forma, u, v, 0.012) else { continue };
            let (a, b) = en(u, v);
            let c = pixel(a, b);
            revisados += 1;
            if (esperado && !es_claro(c)) || (!esperado && !es_negro(c)) {
                mal.push(format!("({u:.2},{v:.2}) {c:?} tocaba {}", if esperado { "blanco" } else { "negro" }));
            }
        }
    }
    anotar(
        mal.is_empty(),
        format!("Forma de 4 esquinas torcida: {revisados} puntos revisados · mal {}{}", mal.len(), mal.first().map(|m| format!(" (ej.: {m})")).unwrap_or_default()),
    );

    // 3. Mover las esquinas en vivo (como al arrastrarlas): 90 cambios en 3 s, el centro nunca negro.
    let t0 = Instant::now();
    let (mut muestras, mut negros) = (0, 0);
    let mut k = 0;
    while t0.elapsed() < Duration::from_secs(3) {
        let a = (k as f64 * 0.21).sin() * 0.06;
        let e = [(0.10 + a, 0.10), (0.90, 0.10 - a / 2.0), (0.90 - a, 0.90), (0.10, 0.90 + a / 2.0)];
        if let Some(hh) = inversa_para(e) {
            rep.propiedad("glsl-shader-opts", &opts(hh))?;
        }
        k += 1;
        let fin = Instant::now() + Duration::from_millis(33);
        while Instant::now() < fin {
            let (cx, cy) = en(0.5, 0.5);
            muestras += 1;
            if !es_claro(pixel(cx, cy)) {
                negros += 1;
            }
        }
    }
    anotar(negros == 0, format!("Mover las esquinas en vivo: {k} cambios en 3 s · centro mirado {muestras} veces · no blanco {negros}"));

    // 4. Encuadre (sin deformar), con una imagen 4:3 en pantalla ancha.
    rep.propiedad("glsl-shader-opts", &opts(identidad))?;
    rep.comando(&["loadfile", &mitades.to_string_lossy(), "replace"])?;
    rep.esperar_listo(Duration::from_secs(10))?;
    rep.propiedad("pause", "no")?;
    let mirar = |pasos: &[(&str, &str)], ms: u64| -> Result<(), String> {
        for (n, v) in pasos {
            rep.propiedad(n, v)?;
        }
        std::thread::sleep(Duration::from_millis(ms));
        Ok(())
    };
    let col = |u: f64, v: f64| {
        let (a, b) = en(u, v);
        pixel(a, b)
    };
    mirar(&[], 500)?;
    let ajustar = es_negro(col(0.03, 0.5)) && es_negro(col(0.97, 0.5)) && parecido(col(0.3, 0.5), rojo) && parecido(col(0.7, 0.5), azul);
    anotar(ajustar, format!("Encuadre «Ajustar» (normal): bandas negras a los lados · borde {:?}, centro-izq {:?}", col(0.03, 0.5), col(0.3, 0.5)));
    mirar(&[("panscan", "1.0")], 400)?;
    let llenar = parecido(col(0.03, 0.5), rojo) && parecido(col(0.97, 0.5), azul);
    anotar(llenar, format!("Encuadre «Llenar» (panscan=1, cambiado en vivo): sin bandas · borde {:?}", col(0.03, 0.5)));
    mirar(&[("panscan", "0.0"), ("keepaspect", "no")], 400)?;
    let estirar = parecido(col(0.03, 0.5), rojo) && parecido(col(0.97, 0.5), azul);
    anotar(estirar, format!("Encuadre «Estirar» (keepaspect=no, en vivo): sin bandas · borde {:?}", col(0.03, 0.5)));
    mirar(&[("keepaspect", "yes"), ("video-zoom", "-1")], 400)?;
    // Mitad de tamaño: 4:3 en 1080 de alto = 1440 de ancho → 720 → de 600 a 1320 en 1920.
    let zoom = es_negro(col(0.25, 0.5)) && es_negro(col(0.5, 0.15)) && parecido(col(0.45, 0.5), rojo) && parecido(col(0.55, 0.5), azul);
    anotar(zoom, format!("Zoom (video-zoom=-1, la mitad): rodeado de negro · {:?} / {:?}", col(0.25, 0.5), col(0.45, 0.5)));
    mirar(&[("video-pan-x", "0.5")], 400)?;
    // Movido media imagen a la derecha: de 960 a 1680.
    let mover = es_negro(col(0.45, 0.5)) && parecido(col(0.6, 0.5), rojo) && parecido(col(0.8, 0.5), azul);
    anotar(mover, format!("Mover (video-pan-x=0,5): la imagen se corre a la derecha · {:?} / {:?}", col(0.45, 0.5), col(0.6, 0.5)));
    mirar(&[("video-zoom", "0"), ("video-pan-x", "0"), ("video-crop", "720x1080+720+0")], 600)?;
    let recorte = parecido(col(0.5, 0.5), azul) && !parecido(col(0.5, 0.5), rojo);
    anotar(recorte, format!("Recorte (video-crop: solo la mitad derecha, en vivo): se ve azul · centro {:?}", col(0.5, 0.5)));
    mirar(&[("video-crop", "")], 300)?;

    // 5. Vídeo real con la forma torcida: decodificación por la tarjeta y cuadros perdidos.
    let video: Option<PathBuf> = carpeta.and_then(|c| {
        std::fs::read_dir(c).ok()?.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("mp4")))
    });
    match video {
        Some(v) => {
            rep.comando(&["loadfile", &v.to_string_lossy(), "replace"])?;
            rep.esperar_listo(Duration::from_secs(10))?;
            rep.propiedad("pause", "no")?;
            rep.propiedad("glsl-shader-opts", &opts(hforma))?;
            std::thread::sleep(Duration::from_secs(1));
            let num = |n: &str| rep.leer(n).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
            let (d0, t0v) = (num("frame-drop-count"), num("vo-delayed-frame-count"));
            // 8 s quieto y 4 s moviendo las esquinas.
            std::thread::sleep(Duration::from_secs(8));
            for k in 0..120 {
                let a = (k as f64 * 0.21).sin() * 0.06;
                if let Some(hh) = inversa_para([(0.12 + a, 0.08), (0.80, 0.18), (0.92 - a, 0.88), (0.20, 0.95)]) {
                    rep.propiedad("glsl-shader-opts", &opts(hh))?;
                }
                std::thread::sleep(Duration::from_millis(33));
            }
            let (d1, t1v) = (num("frame-drop-count"), num("vo-delayed-frame-count"));
            let hw = rep.leer("hwdec-current").filter(|s| !s.is_empty() && s != "no").unwrap_or_else(|| "CPU".into());
            anotar(
                d1 == d0 && hw.contains("d3d11"),
                format!(
                    "Vídeo {} 12 s con la forma torcida (4 s moviéndola): decodificación {hw} · cuadros perdidos {} · tarde {}",
                    v.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                    d1 - d0,
                    t1v - t0v
                ),
            );
        }
        None => informe.push("NO MEDIDO  Vídeo real (falta --carpeta con vídeos .mp4)".into()),
    }

    drop(rep);
    drop(proy);
    println!("— Informe —");
    for l in &informe {
        println!("{l}");
    }
    println!("{}", if fallos == 0 { "RESULTADO: BIEN" } else { "RESULTADO: HAY PROBLEMAS" });
    Ok(fallos == 0)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_homografia_lleva_las_esquinas_a_su_lugar() {
        let forma = [(0.12, 0.08), (0.80, 0.18), (0.92, 0.88), (0.20, 0.95)];
        let h = inversa_para(forma).unwrap();
        let ir = |x: f64, y: f64| {
            let z = h[6] * x + h[7] * y + 1.0;
            ((h[0] * x + h[1] * y + h[2]) / z, (h[3] * x + h[4] * y + h[5]) / z)
        };
        for (k, esperado) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].into_iter().enumerate() {
            let (s, t) = ir(forma[k].0, forma[k].1);
            assert!((s - esperado.0).abs() < 1e-9 && (t - esperado.1).abs() < 1e-9, "esquina {k}: {s},{t}");
        }
        assert!(inversa_para([(0.0, 0.0), (0.5, 0.0), (1.0, 0.0), (0.2, 0.0)]).is_none(), "forma aplastada");
    }
}
