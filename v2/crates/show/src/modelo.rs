//! Lo que se guarda en el archivo del show. Todo con valores por defecto
//! (`#[serde(default)]`) para que un archivo con campos que falten se abra
//! igual. Solo datos: nada de esto se ejecuta.

use serde::{Deserialize, Serialize};

pub use crate::superficie::{Encuadre, ModoEncuadre, Superficie};

/// Versión del formato del archivo. Subirla cuando cambie algo que un
/// programa viejo no entendería, y agregar la migración en `archivo.rs`.
pub const VERSION_FORMATO: u32 = 1;

pub const SALIDA_PRINCIPAL: &str = "Principal";
pub const SALIDA_AURICULARES: &str = "Auriculares";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Show {
    pub version_formato: u32,
    pub nombre: String,
    pub ajustes: AjustesShow,
    pub cues: Vec<Cue>,
    pub botonera: Vec<PaginaBotonera>,
    /// Las 4 esquinas del Proyector (etapa 4C). Más adelante, por teatro.
    #[serde(skip_serializing_if = "Superficie::es_normal")]
    pub superficie: Superficie,
}

impl Default for Show {
    fn default() -> Self {
        Self {
            version_formato: VERSION_FORMATO,
            nombre: String::new(),
            ajustes: AjustesShow::default(),
            cues: Vec::new(),
            botonera: Vec::new(),
            superficie: Superficie::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TipoCue {
    Audio,
    Video,
    Imagen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Cue {
    /// Identificador interno único dentro del show (no se muestra).
    pub id: u64,
    /// Número libre para coincidir con el libreto: "1", "1.5", "2A".
    pub numero: String,
    pub nombre: String,
    pub tipo: TipoCue,
    /// Ruta relativa dentro de la carpeta del show, con `/`.
    pub archivo: String,
    /// Volumen en dB (0 = como el archivo).
    pub volumen_db: f32,
    pub fundido_entrada_ms: u32,
    pub fundido_salida_ms: u32,
    pub bucle: bool,
    /// Dónde empieza y dónde termina dentro del archivo.
    pub inicio_ms: u64,
    pub fin_ms: Option<u64>,
    /// Lo que ve el operador antes del GO.
    pub nota: String,
    /// Salida lógica ("Principal", "Auriculares"…).
    pub salida: String,
    /// Vídeo o imagen: cómo entra en la pantalla (zoom, mover, recortar).
    #[serde(skip_serializing_if = "Encuadre::es_normal")]
    pub encuadre: Encuadre,
}

impl Default for Cue {
    fn default() -> Self {
        Self {
            id: 0,
            numero: String::new(),
            nombre: String::new(),
            tipo: TipoCue::Audio,
            archivo: String::new(),
            volumen_db: 0.0,
            fundido_entrada_ms: 0,
            fundido_salida_ms: 0,
            bucle: false,
            inicio_ms: 0,
            fin_ms: None,
            nota: String::new(),
            salida: SALIDA_PRINCIPAL.into(),
            encuadre: Encuadre::default(),
        }
    }
}

/// Rampas cortas en milisegundos (CLAUDE.md, "Rampas cortas"). 0 = de golpe.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RampasMs {
    pub inicio: f32,
    pub reanudar: f32,
    pub pausa: f32,
    pub parada: f32,
    pub salto: f32,
    pub volumen: f32,
    pub fin: f32,
}

impl Default for RampasMs {
    fn default() -> Self {
        Self { inicio: 0.0, reanudar: 5.0, pausa: 10.0, parada: 10.0, salto: 5.0, volumen: 5.0, fin: 10.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AjustesShow {
    pub rampas_ms: RampasMs,
    /// Protección contra doble GO: tiempo mínimo entre dos GO.
    pub tiempo_minimo_entre_go_ms: u32,
    /// Fundir todo (F) tarda esto.
    pub fundir_todo_ms: u32,
    /// Volumen maestro en dB.
    pub volumen_maestro_db: f32,
}

impl Default for AjustesShow {
    fn default() -> Self {
        Self { rampas_ms: RampasMs::default(), tiempo_minimo_entre_go_ms: 300, fundir_todo_ms: 3000, volumen_maestro_db: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlRepetir {
    /// Tocarlo otra vez mientras suena lo para.
    Parar,
    /// Vuelve a empezar.
    Reiniciar,
    /// Suena otra copia encima.
    Superponer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BotonSonido {
    pub nombre: String,
    /// Color en formato `#RRGGBB`.
    pub color: String,
    pub archivo: String,
    pub volumen_db: f32,
    pub fundido_entrada_ms: u32,
    pub fundido_salida_ms: u32,
    pub bucle: bool,
    pub salida: String,
    pub tecla: Option<String>,
    pub al_repetir: AlRepetir,
}

impl Default for BotonSonido {
    fn default() -> Self {
        Self {
            nombre: String::new(),
            color: "#6F80FF".into(),
            archivo: String::new(),
            volumen_db: 0.0,
            fundido_entrada_ms: 0,
            fundido_salida_ms: 0,
            bucle: false,
            salida: SALIDA_PRINCIPAL.into(),
            tecla: None,
            al_repetir: AlRepetir::Parar,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PaginaBotonera {
    pub nombre: String,
    pub botones: Vec<BotonSonido>,
}
