//! Traer archivos al show (etapa 5A): arrastrados a la ventana o elegidos
//! con "Buscar archivos…". El show es una carpeta que se lleva entera: lo que
//! ya está dentro se usa tal cual; lo de afuera se copia a `Medios/` (esto es
//! "Reunir medios" para lo que se agrega).
//!
//! Seguridad (CLAUDE.md, sección Seguridad): solo archivos locales (nada de
//! rutas de red `\\servidor\…`), solo audios, vídeos e imágenes, y la ruta que
//! queda en el show siempre es relativa y dentro de la carpeta.

use std::path::{Path, PathBuf};

use otaconvert_show::{rutas, tipo_por_extension};

use crate::texto::Texto;

/// Carpeta de la carpeta del show donde se copian los medios que vienen de afuera.
pub const CARPETA_MEDIOS: &str = "Medios";

/// Qué hacer con un archivo: copiarlo (de `origen` a `destino`) o no, y la ruta que queda en el show.
#[derive(Debug, Clone, PartialEq)]
pub struct Paso {
    pub origen: PathBuf,
    pub copiar_a: Option<PathBuf>,
    pub relativa: String,
}

fn t(es: impl Into<String>, en: impl Into<String>) -> Texto {
    Texto::nuevo(es.into(), en.into())
}

fn es_de_red(p: &Path) -> bool {
    let s = p.to_string_lossy();
    (s.starts_with("\\\\") || s.starts_with("//")) && !s.starts_with("\\\\?\\") || s.starts_with("\\\\?\\UNC")
}

/// Decide, sin copiar nada todavía, qué pasa con cada archivo.
pub fn planear(carpeta: &Path, rutas_abs: &[PathBuf]) -> Result<Vec<Paso>, Texto> {
    let base = carpeta.canonicalize().map_err(|e| t(format!("carpeta del show: {e}"), format!("show folder: {e}")))?;
    let mut pasos: Vec<Paso> = Vec::new();
    for p in rutas_abs {
        let nombre = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !p.is_absolute() || es_de_red(p) {
            return Err(t(
                format!("{nombre}: solo se pueden agregar archivos de este equipo (no de la red). Cópialo primero al disco."),
                format!("{nombre}: only files on this computer can be added (not from the network). Copy it to the disk first."),
            ));
        }
        if tipo_por_extension(p).is_none() {
            return Err(t(format!("{nombre}: no es un audio, vídeo o imagen que se pueda usar."), format!("{nombre}: not a usable audio, video or image file.")));
        }
        let real = p.canonicalize().map_err(|e| t(format!("{nombre}: {e}"), format!("{nombre}: {e}")))?;
        if !real.is_file() {
            return Err(t(format!("{nombre}: no es un archivo."), format!("{nombre}: not a file.")));
        }
        // Ya está dentro de la carpeta del show: se usa tal cual.
        if let Ok(rel) = real.strip_prefix(&base) {
            let r = rel.to_string_lossy().replace('\\', "/");
            rutas::validar(&r).map_err(|e| t(format!("{nombre}: {e}. Cámbiale el nombre."), format!("{nombre}: invalid name ({e}). Rename it.")))?;
            pasos.push(Paso { origen: real.clone(), copiar_a: None, relativa: r });
            continue;
        }
        // De afuera: se copia a Medios/. Si ya hay uno con ese nombre y el
        // mismo tamaño, es el mismo (se agregó antes): no se vuelve a copiar.
        let largo = real.metadata().map(|m| m.len()).unwrap_or(0);
        let tallo = Path::new(&nombre).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let ext = Path::new(&nombre).extension().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let mut elegido = None;
        for k in 0..1000 {
            let n = if k == 0 { nombre.clone() } else { format!("{tallo} ({k}).{ext}") };
            let destino = base.join(CARPETA_MEDIOS).join(&n);
            let ya_pedido = pasos.iter().any(|x| x.copiar_a.as_deref() == Some(destino.as_path()));
            match destino.metadata() {
                Ok(m) if m.len() == largo && !ya_pedido => {
                    elegido = Some((n, None));
                    break;
                }
                Ok(_) => continue,
                Err(_) if ya_pedido => continue,
                Err(_) => {
                    elegido = Some((n, Some(destino)));
                    break;
                }
            }
        }
        let (n, copiar_a) = elegido.ok_or_else(|| t(format!("{nombre}: demasiados archivos con ese nombre."), format!("{nombre}: too many files with that name.")))?;
        let r = format!("{CARPETA_MEDIOS}/{n}");
        rutas::validar(&r).map_err(|e| t(format!("{nombre}: {e}. Cámbiale el nombre."), format!("{nombre}: invalid name ({e}). Rename it.")))?;
        pasos.push(Paso { origen: real, copiar_a, relativa: r });
    }
    Ok(pasos)
}

/// Copia lo que haga falta (a un temporal y después se renombra: nunca queda
/// un archivo a medias con el nombre bueno). Devuelve las rutas para el show.
pub fn copiar(pasos: &[Paso]) -> Result<Vec<String>, Texto> {
    for p in pasos {
        if let Some(destino) = &p.copiar_a {
            if let Some(d) = destino.parent() {
                std::fs::create_dir_all(d).map_err(|e| t(format!("no se pudo crear {}: {e}", d.display()), format!("could not create {}: {e}", d.display())))?;
            }
            let tmp = destino.with_extension(format!("{}.copiando", destino.extension().map(|e| e.to_string_lossy()).unwrap_or_default()));
            std::fs::copy(&p.origen, &tmp).map_err(|e| {
                let _ = std::fs::remove_file(&tmp);
                t(format!("no se pudo copiar {}: {e}", p.origen.display()), format!("could not copy {}: {e}", p.origen.display()))
            })?;
            std::fs::rename(&tmp, destino).map_err(|e| {
                let _ = std::fs::remove_file(&tmp);
                t(format!("no se pudo copiar {}: {e}", p.origen.display()), format!("could not copy {}: {e}", p.origen.display()))
            })?;
        }
    }
    Ok(pasos.iter().map(|p| p.relativa.clone()).collect())
}

/// Audios, vídeos e imágenes que hay en la carpeta del show (hasta 3 niveles,
/// sin carpetas ocultas ni `registros`), en orden por nombre: para elegir
/// desde la ventana sin buscar en el disco.
pub fn medios_de_la_carpeta(carpeta: &Path) -> Vec<String> {
    fn recorrer(base: &Path, dir: &Path, nivel: usize, salida: &mut Vec<String>) {
        let Ok(leidos) = std::fs::read_dir(dir) else { return };
        for e in leidos.flatten() {
            let p = e.path();
            let nombre = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if nombre.starts_with('.') || nombre == "registros" {
                continue;
            }
            if p.is_dir() && nivel < 3 {
                recorrer(base, &p, nivel + 1, salida);
            } else if tipo_por_extension(&p).is_some() {
                if let Ok(rel) = p.strip_prefix(base) {
                    let r = rel.to_string_lossy().replace('\\', "/");
                    if rutas::validar(&r).is_ok() && salida.len() < 10_000 {
                        salida.push(r);
                    }
                }
            }
        }
    }
    let mut v = Vec::new();
    recorrer(carpeta, carpeta, 0, &mut v);
    v.sort_by_key(|a| a.to_lowercase());
    v
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn de_afuera_se_copia_a_medios_y_lo_de_adentro_no() {
        let raiz = std::env::temp_dir().join(format!("otaconvert-importar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let (show, afuera) = (raiz.join("Mi show"), raiz.join("Descargas"));
        std::fs::create_dir_all(show.join("audio")).unwrap();
        std::fs::create_dir_all(&afuera).unwrap();
        std::fs::write(show.join("audio/sala.wav"), b"sala").unwrap();
        std::fs::write(afuera.join("trueno.wav"), b"trueno").unwrap();
        std::fs::write(afuera.join("notas.txt"), b"x").unwrap();
        let pasos = planear(&show, &[show.join("audio/sala.wav"), afuera.join("trueno.wav")]).unwrap();
        assert_eq!(pasos[0].relativa, "audio/sala.wav");
        assert!(pasos[0].copiar_a.is_none(), "lo de adentro no se copia");
        assert_eq!(pasos[1].relativa, "Medios/trueno.wav");
        let rel = copiar(&pasos).unwrap();
        assert_eq!(rel, vec!["audio/sala.wav", "Medios/trueno.wav"]);
        assert_eq!(std::fs::read(show.join("Medios/trueno.wav")).unwrap(), b"trueno");
        // El mismo otra vez: no se vuelve a copiar.
        assert!(planear(&show, &[afuera.join("trueno.wav")]).unwrap()[0].copiar_a.is_none());
        // Otro distinto con el mismo nombre: "trueno (1).wav".
        std::fs::create_dir_all(afuera.join("otro")).unwrap();
        std::fs::write(afuera.join("otro/trueno.wav"), b"otro trueno distinto").unwrap();
        assert_eq!(planear(&show, &[afuera.join("otro/trueno.wav")]).unwrap()[0].relativa, "Medios/trueno (1).wav");
        // Lo que no se puede.
        assert!(planear(&show, &[afuera.join("notas.txt")]).is_err());
        assert!(planear(&show, &[PathBuf::from(r"\\servidor\carpeta\a.wav")]).is_err());
        assert!(planear(&show, &[PathBuf::from("relativa.wav")]).is_err());
        assert!(planear(&show, &[afuera.join("no-existe.wav")]).is_err());
        // Medios de la carpeta del show.
        assert_eq!(medios_de_la_carpeta(&show), vec!["audio/sala.wav", "Medios/trueno.wav"]);
        let _ = std::fs::remove_dir_all(&raiz);
    }
}
