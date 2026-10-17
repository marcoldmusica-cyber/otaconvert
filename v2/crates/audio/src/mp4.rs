//! Lista de edición de los MP4 / M4A / MOV (cajas `edts`/`elst`).
//!
//! El sonido AAC empieza con unos cuadros de "arranque" del codificador
//! (normalmente 1024, unos 21 ms) y termina con relleno. El archivo dice
//! cuánto saltear y cuánto dura de verdad en su lista de edición; FFmpeg y
//! mpv la respetan (por eso la imagen del vídeo empieza en el cuadro justo).
//! Symphonia 0.6.1 la lee pero no la aplica (`ElstAtom` queda sin usar), así
//! que sin esto el sonido de un vídeo quedaba 21 ms atrasado respecto de la
//! imagen y un bucle saltaba a ese arranque en cada vuelta (clic).
//!
//! Solo lee cabeceras, con límites; si algo no cuadra, no se recorta nada.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// `moov` más grande que esto no se lee (un vídeo de horas tiene unos pocos MB).
const MOOV_MAXIMO: u64 = 64 * 1024 * 1024;

/// Qué hacer con el sonido decodificado, en cuadros de la escala del sonido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recorte {
    /// Silencio que va antes (ediciones vacías: el sonido empieza más tarde que la imagen).
    pub silencio: u64,
    /// Cuadros del principio que no se tocan (arranque del codificador).
    pub saltar: u64,
    /// Cuadros que se tocan después de saltar (`None` = hasta el final).
    pub largo: Option<u64>,
    /// Escala de tiempo del sonido (cuadros por segundo según el archivo).
    pub escala: u32,
}

fn u32_be(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

fn u64_be(b: &[u8], i: usize) -> Option<u64> {
    Some(u64::from_be_bytes(b.get(i..i + 8)?.try_into().ok()?))
}

/// Cajas hijas dentro de `b`: (tipo, contenido).
fn cajas(b: &[u8]) -> Vec<([u8; 4], &[u8])> {
    let mut v = Vec::new();
    let mut i = 0usize;
    while i + 8 <= b.len() {
        let Some(t32) = u32_be(b, i) else { break };
        let tipo: [u8; 4] = match b[i + 4..i + 8].try_into() {
            Ok(t) => t,
            Err(_) => break,
        };
        let (cabecera, tam) = match t32 {
            1 => match u64_be(b, i + 8) {
                Some(t) => (16usize, t as usize),
                None => break,
            },
            0 => (8, b.len() - i),
            t => (8, t as usize),
        };
        if tam < cabecera || i + tam > b.len() {
            break;
        }
        v.push((tipo, &b[i + cabecera..i + tam]));
        i += tam;
    }
    v
}

fn hija<'a>(b: &'a [u8], tipo: &[u8; 4]) -> Option<&'a [u8]> {
    cajas(b).into_iter().find(|(t, _)| t == tipo).map(|(_, c)| c)
}

/// Busca la caja `moov` del archivo (saltando `mdat` sin leerla).
fn leer_moov(ruta: &Path) -> Option<Vec<u8>> {
    let mut f = File::open(ruta).ok()?;
    let largo = f.metadata().ok()?.len();
    let mut pos = 0u64;
    let mut primera = true;
    while pos + 8 <= largo {
        f.seek(SeekFrom::Start(pos)).ok()?;
        let mut cab = [0u8; 16];
        f.read_exact(&mut cab[..8]).ok()?;
        let t32 = u32::from_be_bytes(cab[0..4].try_into().ok()?);
        let tipo: [u8; 4] = cab[4..8].try_into().ok()?;
        // Solo archivos de la familia MP4 (la primera caja es ftyp, o moov/wide/free en MOV viejos).
        if primera && !matches!(&tipo, b"ftyp" | b"moov" | b"wide" | b"free" | b"mdat" | b"skip") {
            return None;
        }
        primera = false;
        let (cabecera, tam) = match t32 {
            1 => {
                f.read_exact(&mut cab[8..16]).ok()?;
                (16u64, u64::from_be_bytes(cab[8..16].try_into().ok()?))
            }
            0 => (8, largo - pos),
            t => (8, t as u64),
        };
        if tam < cabecera {
            return None;
        }
        if &tipo == b"moov" {
            let n = tam - cabecera;
            if n > MOOV_MAXIMO {
                return None;
            }
            let mut v = vec![0u8; n as usize];
            f.read_exact(&mut v).ok()?;
            return Some(v);
        }
        pos = pos.checked_add(tam)?;
    }
    None
}

/// Escala de tiempo de `mvhd` o `mdhd` (versión 0 o 1).
fn escala(b: &[u8]) -> Option<u32> {
    match *b.first()? {
        0 => u32_be(b, 12),
        1 => u32_be(b, 20),
        _ => None,
    }
}

/// Recorte del primer sonido del archivo según su lista de edición.
pub fn recorte(ruta: &Path) -> Option<Recorte> {
    let moov = leer_moov(ruta)?;
    let escala_pelicula = escala(hija(&moov, b"mvhd")?)?;
    for (t, trak) in cajas(&moov) {
        if &t != b"trak" {
            continue;
        }
        let Some(mdia) = hija(trak, b"mdia") else { continue };
        if hija(mdia, b"hdlr").and_then(|h| h.get(8..12)) != Some(b"soun") {
            continue;
        }
        let escala_sonido = escala(hija(mdia, b"mdhd")?)?;
        let elst = hija(hija(trak, b"edts")?, b"elst")?;
        let version = *elst.first()?;
        let n = u32_be(elst, 4)? as usize;
        let mut silencio = 0u64;
        let mut i = 8usize;
        for _ in 0..n.min(64) {
            let (dur, tiempo, paso) = if version == 1 {
                (u64_be(elst, i)?, u64_be(elst, i + 8)? as i64, 20usize)
            } else {
                (u32_be(elst, i)? as u64, u32_be(elst, i + 4)? as i32 as i64, 12usize)
            };
            i += paso;
            // Pasar de la escala de la película a la del sonido.
            let en_sonido = |d: u64| (d as u128 * escala_sonido as u128 / escala_pelicula.max(1) as u128) as u64;
            if tiempo == -1 {
                // Edición vacía: el sonido empieza más tarde.
                silencio += en_sonido(dur);
                continue;
            }
            if tiempo < 0 {
                return None;
            }
            // Primera edición con sonido: desde dónde y cuánto (0 = hasta el final).
            return Some(Recorte {
                silencio,
                saltar: tiempo as u64,
                largo: (dur > 0).then(|| en_sonido(dur)),
                escala: escala_sonido,
            });
        }
        return None;
    }
    None
}

/// Aplica el recorte a muestras intercaladas decodificadas a `frecuencia`.
pub fn aplicar(r: Recorte, muestras: Vec<f32>, canales: usize, frecuencia: u32) -> Vec<f32> {
    if canales == 0 || r.escala == 0 {
        return muestras;
    }
    let a_cuadros = |x: u64| (x as u128 * frecuencia as u128 / r.escala as u128).min(usize::MAX as u128 / 4) as usize;
    let total = muestras.len() / canales;
    let saltar = a_cuadros(r.saltar).min(total);
    let hasta = match r.largo {
        Some(l) => saltar.saturating_add(a_cuadros(l)).min(total),
        None => total,
    };
    let silencio = a_cuadros(r.silencio).min(frecuencia as usize * 60);
    let mut v = Vec::with_capacity((silencio + hasta - saltar) * canales);
    v.resize(silencio * canales, 0.0);
    v.extend_from_slice(&muestras[saltar * canales..hasta * canales]);
    v
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn archivos_que_no_son_mp4_no_se_tocan() {
        let d = std::env::temp_dir().join(format!("otaconvert-mp4-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        for (n, b) in [("vacio.mp4", &b""[..]), ("basura.mp4", b"hola mundo, esto no es un mp4 ni nada parecido"), ("raro.mp4", &[0, 0, 0, 1, b'm', b'o', b'o', b'v'])] {
            let r = d.join(n);
            std::fs::write(&r, b).unwrap();
            assert_eq!(recorte(&r), None, "{n}");
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn aplicar_salta_el_arranque_y_corta_el_relleno() {
        let m: Vec<f32> = (0..20).map(|x| x as f32).collect();
        let r = Recorte { silencio: 0, saltar: 2, largo: Some(5), escala: 48_000 };
        assert_eq!(aplicar(r, m.clone(), 2, 48_000), vec![4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0]);
        let r = Recorte { silencio: 1, saltar: 0, largo: None, escala: 48_000 };
        assert_eq!(aplicar(r, m.clone(), 2, 48_000).len(), 22);
        // Valores imposibles no rompen nada.
        let r = Recorte { silencio: 0, saltar: 1_000_000, largo: Some(u64::MAX / 2), escala: 48_000 };
        assert!(aplicar(r, m, 2, 48_000).is_empty());
    }
}
