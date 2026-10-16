//! Rutas de medios **dentro** de la carpeta del show (regla de seguridad 3):
//! se rechazan `..`, rutas absolutas, unidades (`C:`), rutas de red
//! (`\\servidor\…`, que pueden filtrar la contraseña de Windows), flujos
//! alternativos (`archivo:flujo`) y nombres reservados de Windows.

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RutaInvalida {
    Vacia,
    Absoluta,
    SaleDeLaCarpeta,
    CaracterNoPermitido(char),
    NombreReservado(String),
    FueraDeLaCarpeta,
}

impl std::fmt::Display for RutaInvalida {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vacia => write!(f, "la ruta está vacía"),
            Self::Absoluta => write!(f, "la ruta no puede ser de un disco o de la red; debe estar dentro de la carpeta del show"),
            Self::SaleDeLaCarpeta => write!(f, "la ruta sale de la carpeta del show (\"..\")"),
            Self::CaracterNoPermitido(c) => write!(f, "la ruta tiene un carácter no permitido ({c:?})"),
            Self::NombreReservado(n) => write!(f, "\"{n}\" es un nombre reservado de Windows"),
            Self::FueraDeLaCarpeta => write!(f, "el archivo está fuera de la carpeta del show (enlace o acceso directo)"),
        }
    }
}

const RESERVADOS: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Comprueba una ruta relativa del archivo del show y la devuelve normalizada
/// con `/`. No toca el disco.
pub fn validar(ruta: &str) -> Result<String, RutaInvalida> {
    let r = ruta.trim();
    if r.is_empty() {
        return Err(RutaInvalida::Vacia);
    }
    if let Some(c) = r.chars().find(|c| c.is_control() || matches!(c, '<' | '>' | '"' | '|' | '?' | '*')) {
        return Err(RutaInvalida::CaracterNoPermitido(c));
    }
    let r = r.replace('\\', "/");
    if r.starts_with('/') || r.contains(':') {
        // "/x", "//servidor", "C:..." o "archivo:flujo".
        return Err(if r.starts_with('/') || r.as_bytes().get(1) == Some(&b':') {
            RutaInvalida::Absoluta
        } else {
            RutaInvalida::CaracterNoPermitido(':')
        });
    }
    let mut partes = Vec::new();
    for p in r.split('/') {
        match p {
            "" | "." => continue,
            ".." => return Err(RutaInvalida::SaleDeLaCarpeta),
            _ => {
                let base = p.split('.').next().unwrap_or("").trim_end().to_ascii_uppercase();
                if RESERVADOS.contains(&base.as_str()) {
                    return Err(RutaInvalida::NombreReservado(p.to_string()));
                }
                if p.ends_with(' ') || p.ends_with('.') {
                    return Err(RutaInvalida::CaracterNoPermitido(p.chars().last().unwrap_or(' ')));
                }
                partes.push(p);
            }
        }
    }
    if partes.is_empty() {
        return Err(RutaInvalida::Vacia);
    }
    Ok(partes.join("/"))
}

/// Une la ruta relativa con la carpeta del show. Si el archivo existe, además
/// comprueba que, siguiendo enlaces y uniones, siga dentro de la carpeta.
pub fn resolver(carpeta: &Path, ruta: &str) -> Result<PathBuf, RutaInvalida> {
    let rel = validar(ruta)?;
    let completa = carpeta.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
    debug_assert!(completa.components().all(|c| !matches!(c, Component::ParentDir)));
    if completa.exists() {
        let base = std::fs::canonicalize(carpeta).map_err(|_| RutaInvalida::FueraDeLaCarpeta)?;
        let real = std::fs::canonicalize(&completa).map_err(|_| RutaInvalida::FueraDeLaCarpeta)?;
        if !real.starts_with(&base) {
            return Err(RutaInvalida::FueraDeLaCarpeta);
        }
    }
    Ok(completa)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn acepta_rutas_normales() {
        assert_eq!(validar("audio/lluvia.wav").unwrap(), "audio/lluvia.wav");
        assert_eq!(validar("audio\\Acto 1\\trueno.mp3").unwrap(), "audio/Acto 1/trueno.mp3");
        assert_eq!(validar("./vídeo/luna.mp4").unwrap(), "vídeo/luna.mp4");
        assert_eq!(validar("consola.wav").unwrap(), "consola.wav");
    }

    #[test]
    fn rechaza_rutas_peligrosas() {
        for (r, e) in [
            ("../secreto.txt", RutaInvalida::SaleDeLaCarpeta),
            ("audio/../../x.wav", RutaInvalida::SaleDeLaCarpeta),
            ("C:\\Windows\\win.ini", RutaInvalida::Absoluta),
            ("c:x.wav", RutaInvalida::Absoluta),
            ("\\\\servidor\\compartido\\a.wav", RutaInvalida::Absoluta),
            ("//servidor/compartido/a.wav", RutaInvalida::Absoluta),
            ("/etc/passwd", RutaInvalida::Absoluta),
            ("", RutaInvalida::Vacia),
            ("./", RutaInvalida::Vacia),
        ] {
            assert_eq!(validar(r), Err(e), "{r}");
        }
        assert!(matches!(validar("audio.wav:oculto"), Err(RutaInvalida::CaracterNoPermitido(':'))));
        assert!(matches!(validar("CON"), Err(RutaInvalida::NombreReservado(_))));
        assert!(matches!(validar("audio/nul.wav"), Err(RutaInvalida::NombreReservado(_))));
        assert!(matches!(validar("a\u{0}b.wav"), Err(RutaInvalida::CaracterNoPermitido(_))));
        assert!(matches!(validar("a*.wav"), Err(RutaInvalida::CaracterNoPermitido('*'))));
        assert!(matches!(validar("audio. "), Err(RutaInvalida::CaracterNoPermitido(_))));
    }
}
