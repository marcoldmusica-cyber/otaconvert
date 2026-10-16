//! Archivo del show de Otaconvert V2 (Fase 1, etapa 1).
//!
//! Un show es una carpeta con un archivo `.otashow` (JSON) y sus medios. El
//! archivo solo contiene datos; las rutas de medios son relativas y siempre
//! quedan dentro de la carpeta (regla de seguridad 3 de CLAUDE.md).

pub mod archivo;
pub mod modelo;
pub mod registro;
pub mod rutas;

pub use archivo::{abrir, desde_texto, guardar, validar, ErrorShow, EXTENSION};
pub use modelo::*;
pub use registro::{marca_de_tiempo, Registro};
