//! Registro de la función: qué se disparó, cuándo y cualquier error, en
//! `registros/funcion-AAAA-MM-DD.txt` dentro de la carpeta del show
//! (regla de seguridad 7: nada personal ni rutas de fuera de la carpeta).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Fecha y hora local (año, mes, día, hora, minuto, segundo, milisegundo).
pub fn ahora() -> (u16, u16, u16, u16, u16, u16, u16) {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct SystemTime {
            anio: u16,
            mes: u16,
            dia_semana: u16,
            dia: u16,
            hora: u16,
            minuto: u16,
            segundo: u16,
            ms: u16,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GetLocalTime(t: *mut SystemTime);
        }
        let mut t = SystemTime { anio: 0, mes: 0, dia_semana: 0, dia: 0, hora: 0, minuto: 0, segundo: 0, ms: 0 };
        // SAFETY: GetLocalTime rellena una estructura SYSTEMTIME de 8 u16.
        unsafe { GetLocalTime(&mut t) };
        (t.anio, t.mes, t.dia, t.hora, t.minuto, t.segundo, t.ms)
    }
    #[cfg(not(windows))]
    {
        (2000, 1, 1, 0, 0, 0, 0)
    }
}

/// `AAAAMMDD-HHMMSS`, para nombres de copias.
pub fn marca_de_tiempo() -> String {
    let (a, mo, d, h, mi, s, _) = ahora();
    format!("{a:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

pub struct Registro {
    archivo: Option<File>,
    pub ruta: PathBuf,
}

impl Registro {
    /// Abre (o crea) el registro del día. Si no se puede escribir, sigue sin
    /// registro: nunca debe frenar la función.
    pub fn abrir(carpeta_show: &Path) -> Self {
        let (a, mo, d, ..) = ahora();
        let dir = carpeta_show.join("registros");
        let ruta = dir.join(format!("funcion-{a:04}-{mo:02}-{d:02}.txt"));
        let archivo = fs::create_dir_all(&dir).ok().and_then(|_| OpenOptions::new().create(true).append(true).open(&ruta).ok());
        Self { archivo, ruta }
    }

    /// Anota una línea con la hora: `21:03:12.345  GO  Cue 3 «Campanas»`.
    pub fn anotar(&mut self, tipo: &str, texto: &str) {
        let (_, _, _, h, mi, s, ms) = ahora();
        let limpio: String = texto.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
        if let Some(f) = &mut self.archivo {
            let _ = writeln!(f, "{h:02}:{mi:02}:{s:02}.{ms:03}  {tipo:<8} {limpio}");
            let _ = f.flush();
        }
    }
}
