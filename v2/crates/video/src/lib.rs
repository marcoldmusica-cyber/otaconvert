//! Salida de vídeo de Otaconvert V2 (Fase 0).
//!
//! - `mpv`: libmpv cargada con ruta completa desde la carpeta del programa
//!   (nunca por nombre), con opciones que solo permiten archivos locales.
//! - `ventana`: ventana de proyección negra y sin cursor en la pantalla
//!   elegida, con una ventana hija por capa.
//!
//! Sin huecos negros: el cue siguiente se carga en pausa en una capa de abajo
//! con su primer cuadro ya dibujado; en el GO esa capa sube y arranca, y solo
//! después se suelta la anterior.

#[cfg(windows)]
pub mod mpv;
#[cfg(windows)]
pub mod ventana;
