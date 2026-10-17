//! La proyección vista desde la sesión: órdenes y avisos. El motor la
//! implementa con un proceso aparte (`otaconvert-motor --proyector`) y un
//! vigilante; las pruebas, con un proyector falso.

use std::path::PathBuf;

use otaconvert_show::Encuadre;

use crate::texto::Texto;

#[derive(Debug, Clone, PartialEq)]
pub enum OrdenProyector {
    /// Cargar en pausa, debajo del telón, sin que se vea.
    /// `inicio_s`/`fin_s`: los del cue (el bucle vuelve a `inicio_s`).
    /// `desde_s`: dónde queda esperando (salir desde un punto, o tras una caída).
    /// `encuadre`: cómo entra en la pantalla (zoom, mover, recortar).
    Precargar {
        id: u64,
        ruta: PathBuf,
        bucle: bool,
        imagen: bool,
        inicio_s: f64,
        fin_s: Option<f64>,
        desde_s: Option<f64>,
        encuadre: Encuadre,
    },
    /// Las 4 esquinas del Proyector: los 8 números de la homografía inversa
    /// (`Superficie::homografia_inversa`). Vale para todo lo que se proyecte.
    Esquinas([f64; 8]),
    /// Patrón de prueba encima de todo (para encajar las esquinas), o quitarlo.
    Patron(bool),
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
    /// Algo para avisar que no impide proyectar (p. ej. las esquinas no se pueden usar en esta tarjeta).
    Aviso { motivo: Texto },
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
        "esquinas" => Texto::nuevo(
            "La tarjeta gráfica de este equipo no pudo usar las 4 esquinas: la imagen sale sin deformar. Revisa el driver de la tarjeta gráfica o prueba con la Prueba de hardware.",
            "This computer's graphics card could not use the 4 corners: the image is shown without warping. Check the graphics driver or run the Hardware test.",
        ),
        otro => Texto::from(otro.to_string()),
    }
}
