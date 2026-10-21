//! Etapa 5B: las salidas de sonido (Principal y Auriculares) y la pantalla
//! del proyector **de este equipo**. Se eligen desde la ventana (Edición →
//! Ajustes del show → Salidas) y se recuerdan en la carpeta de datos del
//! programa (`equipo.json`), no en el show: cada teatro tiene su tarjeta.
//!
//! Regla (CLAUDE.md, "Elegir la salida es FUNDAMENTAL"): si la elegida no está,
//! se avisa fuerte; nunca se cambia en silencio a otra.

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use serde_json::{json, Value};

/// Lo elegido en este equipo. `None` = la predeterminada de Windows (o, para
/// la pantalla, la primera que no es la principal).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Guardado {
    /// (id estable de cpal, nombre para avisar si falta)
    pub principal: Option<(String, String)>,
    pub auriculares: Option<(String, String)>,
    /// "vista" o (dispositivo de Windows, nombre del monitor o proyector)
    pub pantalla: Option<(String, String)>,
}

fn archivo() -> Option<PathBuf> {
    crate::proyector::carpeta_datos().map(|d| d.join("equipo.json"))
}

fn par(v: &Value) -> Option<(String, String)> {
    Some((v.get("id")?.as_str()?.to_string(), v.get("nombre").and_then(|n| n.as_str()).unwrap_or("").to_string()))
}

impl Guardado {
    /// Lee lo guardado; si no hay archivo o está dañado, nada (las predeterminadas).
    pub fn leer() -> Guardado {
        let Some(t) = archivo().and_then(|a| std::fs::read_to_string(a).ok()) else { return Guardado::default() };
        let Ok(v) = serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')) else { return Guardado::default() };
        Guardado { principal: par(&v["principal"]), auriculares: par(&v["auriculares"]), pantalla: par(&v["pantalla"]) }
    }

    /// Guarda (a un temporal y después renombra: nunca queda a medias).
    pub fn guardar(&self) -> Result<(), String> {
        let a = archivo().ok_or("no hay carpeta de datos")?;
        if let Some(d) = a.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let j = |p: &Option<(String, String)>| p.as_ref().map(|(id, n)| json!({ "id": id, "nombre": n })).unwrap_or(Value::Null);
        let v = json!({ "version": 1, "principal": j(&self.principal), "auriculares": j(&self.auriculares), "pantalla": j(&self.pantalla) });
        let tmp = a.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&v).unwrap_or_default()).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &a).map_err(|e| e.to_string())
    }
}

/// Las salidas de sonido y las pantallas que ve Windows ahora (para la ventana).
pub fn lista_json() -> Value {
    use cpal::traits::{DeviceTrait, HostTrait};
    let predeterminada = cpal::default_host().default_output_device().and_then(|d| d.id().ok()).map(|i| i.to_string());
    let salidas: Vec<Value> = crate::salidas()
        .unwrap_or_default()
        .into_iter()
        .map(|(d, id)| {
            let c = d.default_output_config().ok();
            json!({
                "id": id,
                "nombre": d.to_string(),
                "predeterminada": predeterminada.as_deref() == Some(id.as_str()),
                "hz": c.as_ref().map(|c| c.sample_rate()),
                "canales": c.as_ref().map(|c| c.channels()),
            })
        })
        .collect();
    let pantallas: Vec<Value> = crate::pantallas::listar()
        .into_iter()
        .map(|p| json!({ "id": p.dispositivo, "nombre": p.nombre, "ancho": p.ancho, "alto": p.alto, "hz": p.hz, "principal": p.principal, "gpu": p.gpu }))
        .collect();
    json!({ "salidas": salidas, "pantallas": pantallas })
}

/// Busca la pantalla guardada: por dispositivo y nombre; si Windows las
/// reordenó (cambia `\\.\DISPLAYn`), por el nombre del proyector si es único.
pub fn buscar_pantalla(dispositivo: &str, nombre: &str) -> Option<crate::pantallas::Pantalla> {
    let lista = crate::pantallas::listar();
    if let Some(p) = lista.iter().find(|p| p.dispositivo.eq_ignore_ascii_case(dispositivo) && p.nombre == nombre) {
        return Some(p.clone());
    }
    let mismos: Vec<_> = lista.iter().filter(|p| !nombre.is_empty() && p.nombre == nombre).collect();
    match mismos.as_slice() {
        [p] => Some((*p).clone()),
        _ => None,
    }
}

/// Una salida de sonido abierta y lo que el programa vigila de ella.
pub struct Abierta {
    /// Se guarda solo para que siga sonando: al soltarla, la tarjeta se cierra.
    #[allow(dead_code)]
    pub stream: cpal::Stream,
    pub id: String,
    pub nombre: String,
    pub errores: Arc<AtomicU64>,
    pub tardias: Arc<AtomicU64>,
    pub intervalo_max_us: Arc<AtomicU64>,
}

/// Principal y (si hay) Auriculares, abiertas juntas, y el sonido para la sesión.
pub struct JuegoSonido {
    pub principal: Abierta,
    pub auriculares: Option<Abierta>,
    pub sonido: crate::sonido_cpal::SonidoCpal,
    pub frecuencia: u32,
}

/// Abre Principal y Auriculares. Si Auriculares no está o trabaja a otra
/// frecuencia, queda sin usar y se devuelve el motivo para avisarlo.
pub fn abrir_sonido(principal: Option<&str>, auriculares: Option<&str>, margen_ms: u64) -> Result<(JuegoSonido, Option<String>), String> {
    let p = crate::sonido_cpal::abrir_salida(principal, margen_ms)?;
    let frecuencia = p.frecuencia;
    let mut aviso = None;
    let mut aur = None;
    let mut control_aur = None;
    if let Some(a) = auriculares {
        match crate::sonido_cpal::abrir_salida(Some(a), margen_ms) {
            Ok(s) if s.frecuencia == frecuencia => {
                control_aur = Some(s.control);
                aur = Some(Abierta { stream: s.stream, id: s.id, nombre: s.nombre, errores: s.errores, tardias: s.tardias, intervalo_max_us: s.intervalo_max_us });
            }
            Ok(s) => {
                aviso = Some(format!(
                    "Auriculares ({}) trabaja a {} Hz y Principal a {} Hz; por ahora deben ser iguales. Auriculares queda sin usar (cámbialo en Configuración de sonido de Windows).",
                    s.nombre, s.frecuencia, frecuencia
                ))
            }
            Err(e) => aviso = Some(format!("No se pudo abrir Auriculares: {e}. Queda sin usar.")),
        }
    }
    let sonido = crate::sonido_cpal::SonidoCpal { principal: p.control, frecuencia, auriculares: control_aur };
    let principal = Abierta { stream: p.stream, id: p.id, nombre: p.nombre, errores: p.errores, tardias: p.tardias, intervalo_max_us: p.intervalo_max_us };
    Ok((JuegoSonido { principal, auriculares: aur, sonido, frecuencia }, aviso))
}

/// Lo que la ventana muestra como "elegido ahora".
pub fn elegido_json(principal: &Abierta, auriculares: Option<&Abierta>, hz: u32, pantalla: &Value) -> Value {
    json!({
        "principal": { "id": principal.id, "nombre": principal.nombre, "hz": hz },
        "auriculares": auriculares.map(|a| json!({ "id": a.id, "nombre": a.nombre })),
        "pantalla": pantalla,
    })
}
