//! Mensajes del motor que ve la ventana, en los dos idiomas del programa.
//! La ventana muestra el del idioma elegido: nunca mezclados (CLAUDE.md,
//! regla de interfaz). El registro de la función se escribe en español.

pub use otaconvert_audio::consejos::Idioma;
use otaconvert_audio::preparar::ErrorPreparar;

#[derive(Clone, Debug, PartialEq)]
pub struct Texto {
    pub es: String,
    pub en: String,
}

impl Texto {
    pub fn nuevo(es: impl Into<String>, en: impl Into<String>) -> Self {
        Self { es: es.into(), en: en.into() }
    }

    pub fn en_idioma(&self, idioma: Idioma) -> &str {
        match idioma {
            Idioma::Es => &self.es,
            Idioma::En => &self.en,
        }
    }

    /// ¿Contiene `parte` en alguno de los dos idiomas? (para pruebas)
    pub fn contiene(&self, parte: &str) -> bool {
        self.es.contains(parte) || self.en.contains(parte)
    }
}

/// Un detalle técnico que no tiene traducción (viene de Windows o de una
/// librería): el mismo texto en los dos idiomas.
impl From<String> for Texto {
    fn from(s: String) -> Self {
        Self { en: s.clone(), es: s }
    }
}

impl From<&str> for Texto {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

impl From<ErrorPreparar> for Texto {
    fn from(e: ErrorPreparar) -> Self {
        Self { es: e.texto(Idioma::Es), en: e.texto(Idioma::En) }
    }
}

/// "es" o "en" → idioma.
pub fn idioma_de(codigo: &str) -> Option<Idioma> {
    match codigo {
        "es" => Some(Idioma::Es),
        "en" => Some(Idioma::En),
        _ => None,
    }
}
