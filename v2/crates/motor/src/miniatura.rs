//! Foto chica de un vídeo o una imagen (para la lista y la Vista Estudio):
//! `otaconvert-motor --miniatura <archivo> <salida.jpg> [inicio_s]`.
//!
//! Corre en un proceso aparte (lo abre el motor, uno a la vez y con límite de
//! tiempo): un archivo dañado nunca puede afectar al motor ni a la función.
//! Usa libmpv sin ventana con su salida a imagen (`vo=image`): un solo cuadro,
//! escalado a 320 px de ancho, en JPEG.

use std::path::Path;
use std::time::{Duration, Instant};

use otaconvert_video::mpv::{ruta_junto_al_exe, Evento, Libmpv, Reproductor};

pub const ANCHO: u32 = 320;
const LIMITE: Duration = Duration::from_secs(15);

/// `inicio`: segundos («12.500») o porcentaje del vídeo («40.0%», lo acepta la opción `start` de mpv).
pub fn ejecutar(archivo: &Path, salida: &Path, inicio: &str) -> Result<bool, String> {
    if !archivo.is_absolute() || !archivo.is_file() {
        return Err("archivo no encontrado".into());
    }
    let dir = salida.parent().ok_or("salida sin carpeta")?;
    // Carpeta temporal propia: mpv escribe ahí "00000001.jpg".
    let tmp = dir.join(format!(".tmp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let r = (|| -> Result<(), String> {
        let lib = Libmpv::cargar(&ruta_junto_al_exe()?)?;
        let tmp_s = tmp.to_string_lossy().into_owned();

        let filtro = format!("scale=w={ANCHO}:h=-2");
        let rep = Reproductor::sin_ventana(
            lib,
            &[
                ("vo", "image"),
                ("vo-image-format", "jpg"),
                ("vo-image-jpeg-quality", "80"),
                ("vo-image-outdir", &tmp_s),
                ("frames", "1"),
                ("vf", &filtro),
                ("start", inicio),
                ("image-display-duration", "0"),
            ],
        )?;
        rep.comando(&["loadfile", &archivo.to_string_lossy()])?;
        let fin = Instant::now() + LIMITE;
        loop {
            let resta = fin.saturating_duration_since(Instant::now());
            if resta.is_zero() {
                return Err("tardó demasiado".into());
            }
            match rep.evento(resta.as_secs_f64()) {
                Evento::FinArchivo(true) => return Err("mpv no pudo abrir el archivo".into()),
                Evento::FinArchivo(false) | Evento::Apagado => return Ok(()),
                _ => {}
            }
        }
    })();
    let foto = std::fs::read_dir(&tmp)
        .ok()
        .and_then(|l| l.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "jpg")));
    let resultado = match (r, foto) {
        (Ok(()), Some(f)) => std::fs::rename(&f, salida).map(|_| true).map_err(|e| e.to_string()),
        (Ok(()), None) => Err("mpv no dejó ninguna foto".into()),
        (Err(e), _) => Err(e),
    };
    let _ = std::fs::remove_dir_all(&tmp);
    resultado
}
