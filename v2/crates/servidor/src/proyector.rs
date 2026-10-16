//! La proyección vista desde la sesión: órdenes y avisos. El motor la
//! implementa con un proceso aparte (`otaconvert-motor --proyector`) y un
//! vigilante; las pruebas, con un proyector falso.

use std::path::PathBuf;

use crate::texto::Texto;

#[derive(Debug, Clone, PartialEq)]
pub enum OrdenProyector {
    /// Cargar en pausa, debajo del telón, sin que se vea.
    /// `inicio_s`/`fin_s`: los del cue (el bucle vuelve a `inicio_s`).
    /// `desde_s`: dónde queda esperando (salir desde un punto, o tras una caída).
    Precargar { id: u64, ruta: PathBuf, bucle: bool, imagen: bool, inicio_s: f64, fin_s: Option<f64>, desde_s: Option<f64> },
    /// Subir encima de todo y arrancar; la anterior se retira sola.
    Mostrar { id: u64 },
    /// Parar todo: telón negro al instante.
    Negro,
    /// Fundir a negro en `ms`.
    Fundir { ms: u64 },
    Pausa(bool),
    /// Soltar un cue precargado que ya no hace falta.
    Quitar { id: u64 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AvisoProyector {
    /// Precargado: el primer cuadro ya está listo. `duracion` en segundos (vídeo).
    Listo { id: u64, duracion: Option<f64> },
    Error { id: u64, motivo: Texto },
    Mostrando { id: u64 },
    /// Un vídeo sin bucle llegó al final (se queda en su último cuadro).
    Fin { id: u64 },
    /// El proceso de proyección se cayó o se colgó y se volvió a abrir (en negro).
    Reiniciado { ms: u64 },
    /// No se pudo volver a abrir la proyección.
    Caido { motivo: Texto },
}

pub trait Proyector: Send {
    fn enviar(&mut self, orden: OrdenProyector);
    /// Avisos nuevos desde la última vez. También vigila el proceso.
    fn recoger(&mut self) -> Vec<AvisoProyector>;
}

/// Motivo de un error del proceso de proyección, en los dos idiomas.
pub fn motivo(codigo: &str) -> Texto {
    match codigo {
        "abrir" => Texto::nuevo("no se pudo abrir el archivo", "the file could not be opened"),
        "ruta" => Texto::nuevo("archivo no encontrado o ruta no permitida", "file not found or path not allowed"),
        "capas" => Texto::nuevo("no hay capas libres en la proyección", "no free layers in the projection"),
        "tiempo" => Texto::nuevo("tardó demasiado en cargar", "it took too long to load"),
        "no_listo" => Texto::nuevo("no estaba listo", "it was not ready"),
        otro => Texto::from(otro.to_string()),
    }
}
