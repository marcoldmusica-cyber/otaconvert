//! Abrir y guardar el archivo del show.
//!
//! - Abrir nunca cierra el programa: un archivo dañado, enorme, de una versión
//!   más nueva o con rutas peligrosas devuelve un error explicado.
//! - Guardar escribe primero a un temporal y lo renombra: un corte de luz deja
//!   el archivo anterior o el nuevo, nunca uno a medias. Antes, copia el
//!   anterior a `.otaconvert/copias/` y conserva las últimas `COPIAS`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::modelo::{Show, VERSION_FORMATO};
use crate::rutas;

pub const EXTENSION: &str = "otashow";
/// Más grande que esto no es un show razonable (evita colgarse con un archivo malicioso).
pub const TAMANO_MAXIMO: u64 = 20 * 1024 * 1024;
pub const CUES_MAXIMOS: usize = 10_000;
pub const COPIAS: usize = 20;
const CARPETA_COPIAS: &str = ".otaconvert/copias";

#[derive(Debug)]
pub enum ErrorShow {
    NoSePudoLeer(String),
    Demasiado(String),
    Danado { linea: usize, columna: usize, detalle: String },
    VersionMasNueva(u32),
    RutaMala { donde: String, motivo: rutas::RutaInvalida },
    ValorMalo(String),
    NoSePudoGuardar(String),
}

impl std::fmt::Display for ErrorShow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSePudoLeer(e) => write!(f, "no se pudo leer el show: {e}"),
            Self::Demasiado(e) => write!(f, "el archivo del show es demasiado grande: {e}"),
            Self::Danado { linea, columna, detalle } => {
                write!(f, "el archivo del show está dañado (línea {linea}, columna {columna}): {detalle}. Puedes abrir una copia de .otaconvert/copias")
            }
            Self::VersionMasNueva(v) => write!(
                f,
                "este show se guardó con una versión más nueva de Otaconvert (formato {v}; esta versión entiende hasta {VERSION_FORMATO}). Actualiza Otaconvert para abrirlo"
            ),
            Self::RutaMala { donde, motivo } => write!(f, "{donde}: {motivo}"),
            Self::ValorMalo(e) => write!(f, "valor no válido en el show: {e}"),
            Self::NoSePudoGuardar(e) => write!(f, "no se pudo guardar el show: {e}"),
        }
    }
}

impl std::error::Error for ErrorShow {}

/// Comprueba lo que serde no puede: rutas, números razonables, límites.
pub fn validar(show: &Show) -> Result<(), ErrorShow> {
    if show.version_formato > VERSION_FORMATO {
        return Err(ErrorShow::VersionMasNueva(show.version_formato));
    }
    if show.cues.len() > CUES_MAXIMOS {
        return Err(ErrorShow::Demasiado(format!("{} cues (máximo {CUES_MAXIMOS})", show.cues.len())));
    }
    let numero_ok = |v: f32| v.is_finite();
    for (i, c) in show.cues.iter().enumerate() {
        let donde = format!("cue {} ({})", if c.numero.is_empty() { (i + 1).to_string() } else { c.numero.clone() }, c.nombre);
        if !c.archivo.is_empty() {
            rutas::validar(&c.archivo).map_err(|motivo| ErrorShow::RutaMala { donde: donde.clone(), motivo })?;
        }
        if !numero_ok(c.volumen_db) || c.volumen_db > 24.0 {
            return Err(ErrorShow::ValorMalo(format!("{donde}: volumen {}", c.volumen_db)));
        }
        if let Some(fin) = c.fin_ms {
            if fin <= c.inicio_ms {
                return Err(ErrorShow::ValorMalo(format!("{donde}: el fin está antes del inicio")));
            }
        }
        if let Some(p) = c.encuadre.problema() {
            return Err(ErrorShow::ValorMalo(format!("{donde}: {p}")));
        }
    }
    if let Some(p) = show.superficie.problema() {
        return Err(ErrorShow::ValorMalo(format!("superficie del proyector: {p}")));
    }
    for p in &show.botonera {
        for b in &p.botones {
            if !b.archivo.is_empty() {
                rutas::validar(&b.archivo).map_err(|motivo| ErrorShow::RutaMala {
                    donde: format!("botón \"{}\" de la página \"{}\"", b.nombre, p.nombre),
                    motivo,
                })?;
            }
            if !numero_ok(b.volumen_db) || b.volumen_db > 24.0 {
                return Err(ErrorShow::ValorMalo(format!("botón \"{}\": volumen {}", b.nombre, b.volumen_db)));
            }
        }
    }
    let r = &show.ajustes.rampas_ms;
    for (nombre, v) in [
        ("inicio", r.inicio),
        ("reanudar", r.reanudar),
        ("pausa", r.pausa),
        ("parada", r.parada),
        ("salto", r.salto),
        ("volumen", r.volumen),
        ("fin", r.fin),
    ] {
        if !numero_ok(v) || !(0.0..=10_000.0).contains(&v) {
            return Err(ErrorShow::ValorMalo(format!("rampa de {nombre}: {v} ms")));
        }
    }
    if !numero_ok(show.ajustes.volumen_maestro_db) || show.ajustes.volumen_maestro_db > 24.0 {
        return Err(ErrorShow::ValorMalo(format!("volumen maestro {}", show.ajustes.volumen_maestro_db)));
    }
    if show.ajustes.tiempo_minimo_entre_go_ms > 5_000 {
        return Err(ErrorShow::ValorMalo(format!("protección contra doble GO {} ms", show.ajustes.tiempo_minimo_entre_go_ms)));
    }
    if show.ajustes.transicion.ms > 10_000 {
        return Err(ErrorShow::ValorMalo(format!("transición {} ms", show.ajustes.transicion.ms)));
    }
    if show.ajustes.fundir_todo_ms > 60_000 {
        return Err(ErrorShow::ValorMalo(format!("Fundir todo {} ms", show.ajustes.fundir_todo_ms)));
    }
    Ok(())
}

/// Abre un show desde un texto JSON (sin tocar el disco).
pub fn desde_texto(texto: &str) -> Result<Show, ErrorShow> {
    // Primero solo la versión, para avisar bien si es de un programa más nuevo
    // aunque tenga campos que este no entiende.
    #[derive(serde::Deserialize)]
    struct SoloVersion {
        #[serde(default)]
        version_formato: u32,
    }
    if let Ok(v) = serde_json::from_str::<SoloVersion>(texto) {
        if v.version_formato > VERSION_FORMATO {
            return Err(ErrorShow::VersionMasNueva(v.version_formato));
        }
    }
    let show: Show = serde_json::from_str(texto).map_err(|e| ErrorShow::Danado {
        linea: e.line(),
        columna: e.column(),
        detalle: e.to_string(),
    })?;
    validar(&show)?;
    Ok(migrar(show))
}

/// Lleva un show viejo al formato actual. Hoy solo existe el formato 1.
fn migrar(mut show: Show) -> Show {
    if show.version_formato < VERSION_FORMATO {
        show.version_formato = VERSION_FORMATO;
    }
    show
}

pub fn abrir(ruta: &Path) -> Result<Show, ErrorShow> {
    let meta = fs::metadata(ruta).map_err(|e| ErrorShow::NoSePudoLeer(e.to_string()))?;
    if meta.len() > TAMANO_MAXIMO {
        return Err(ErrorShow::Demasiado(format!("{} MB", meta.len() / (1024 * 1024))));
    }
    let bytes = fs::read(ruta).map_err(|e| ErrorShow::NoSePudoLeer(e.to_string()))?;
    let texto = String::from_utf8(bytes).map_err(|_| ErrorShow::Danado {
        linea: 0,
        columna: 0,
        detalle: "no es texto UTF-8".into(),
    })?;
    // El Bloc de notas puede agregar una marca BOM al principio.
    desde_texto(texto.trim_start_matches('\u{FEFF}'))
}

/// Guarda de forma segura y deja una copia del anterior.
pub fn guardar(show: &Show, ruta: &Path, marca_de_tiempo: &str) -> Result<(), ErrorShow> {
    validar(show)?;
    let texto = serde_json::to_string_pretty(show).map_err(|e| ErrorShow::NoSePudoGuardar(e.to_string()))?;
    let carpeta = ruta.parent().unwrap_or(Path::new("."));
    if ruta.exists() {
        copiar_anterior(ruta, carpeta, marca_de_tiempo)?;
    }
    let tmp = ruta.with_extension(format!("{EXTENSION}.tmp"));
    {
        let mut f = fs::File::create(&tmp).map_err(|e| ErrorShow::NoSePudoGuardar(e.to_string()))?;
        f.write_all(texto.as_bytes()).map_err(|e| ErrorShow::NoSePudoGuardar(e.to_string()))?;
        // Que llegue al disco antes de renombrar.
        f.sync_all().map_err(|e| ErrorShow::NoSePudoGuardar(e.to_string()))?;
    }
    // En Windows reemplaza el archivo existente (MoveFileExW con REPLACE_EXISTING).
    fs::rename(&tmp, ruta).map_err(|e| ErrorShow::NoSePudoGuardar(e.to_string()))?;
    Ok(())
}

fn copiar_anterior(ruta: &Path, carpeta: &Path, marca: &str) -> Result<(), ErrorShow> {
    let dir = carpeta.join(CARPETA_COPIAS);
    fs::create_dir_all(&dir).map_err(|e| ErrorShow::NoSePudoGuardar(format!("copias: {e}")))?;
    let base = ruta.file_stem().and_then(|s| s.to_str()).unwrap_or("show");
    let destino = dir.join(format!("{base}-{marca}.{EXTENSION}"));
    fs::copy(ruta, &destino).map_err(|e| ErrorShow::NoSePudoGuardar(format!("copia: {e}")))?;
    // Conservar solo las últimas COPIAS de este show (el nombre lleva la fecha: orden alfabético = cronológico).
    let prefijo = format!("{base}-");
    let mut copias: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|e| ErrorShow::NoSePudoGuardar(e.to_string()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&prefijo) && n.ends_with(EXTENSION))
        })
        .collect();
    copias.sort();
    while copias.len() > COPIAS {
        let vieja = copias.remove(0);
        let _ = fs::remove_file(vieja);
    }
    Ok(())
}
