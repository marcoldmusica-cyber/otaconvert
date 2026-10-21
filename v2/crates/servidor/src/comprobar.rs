//! Luz verde (etapa 5C): la comprobación antes de la función. Solo **lee**:
//! no cambia nada del show ni de lo que suena. Revisa cue por cue que el
//! archivo esté y se pueda usar, y avisa de lo que conviene mirar antes de
//! abrir sala. Lo del equipo (protector de pantalla, batería, pendrive…) lo
//! agrega el motor con `ComprobarEquipo`.

use std::path::{Path, PathBuf};

use otaconvert_show::{rutas, Cue, TipoCue};
use serde_json::{json, Value};

use crate::texto::Texto;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Nivel {
    Bien,
    Aviso,
    Problema,
}

impl Nivel {
    pub fn nombre(self) -> &'static str {
        match self {
            Nivel::Bien => "bien",
            Nivel::Aviso => "aviso",
            Nivel::Problema => "problema",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Punto {
    pub nivel: Nivel,
    pub texto: Texto,
}

impl Punto {
    pub fn nuevo(nivel: Nivel, es: impl Into<String>, en: impl Into<String>) -> Self {
        Punto { nivel, texto: Texto::nuevo(es, en) }
    }
}

/// Lo que hace falta saber del show, copiado con la sesión tomada (después
/// se revisa sin tenerla: decodificar los audios tarda).
#[derive(Clone)]
pub struct Pedido {
    pub carpeta: PathBuf,
    /// (número para mostrar, cue)
    pub cues: Vec<(String, Cue)>,
    /// Frecuencia de la tarjeta (`None` = sin sonido).
    pub frecuencia: Option<u32>,
    pub rampa_inicio_ms: f32,
    pub con_proyeccion: bool,
    pub auriculares: bool,
    pub sucio: bool,
    /// Errores que la sesión ya conoce (p. ej. un vídeo que la proyección no pudo abrir).
    pub errores: Vec<(String, Texto)>,
}

fn nombre_cue(n: &str, c: &Cue) -> (String, String) {
    (format!("Cue {n} «{}»", c.nombre), format!("Cue {n} “{}”", c.nombre))
}

/// Revisa todo. `progreso(hechos, total)` se llama después de cada cue.
pub fn revisar(p: &Pedido, mut progreso: impl FnMut(usize, usize)) -> Vec<Punto> {
    let mut puntos = Vec::new();
    let total = p.cues.len();
    if total == 0 {
        puntos.push(Punto::nuevo(Nivel::Aviso, "El show no tiene cues todavía.", "The show has no cues yet."));
    }
    match p.frecuencia {
        None => puntos.push(Punto::nuevo(
            Nivel::Problema,
            "No hay salida de sonido: los audios no van a sonar. Revisa la tarjeta en Ajustes del show → Salidas.",
            "There is no sound output: audio will not play. Check the card in Show settings → Outputs.",
        )),
        Some(fs) => puntos.push(Punto::nuevo(Nivel::Bien, format!("Salida de sonido abierta a {fs} Hz."), format!("Sound output open at {fs} Hz."))),
    }
    let hay_visuales = p.cues.iter().any(|(_, c)| c.tipo != TipoCue::Audio);
    if hay_visuales && !p.con_proyeccion {
        puntos.push(Punto::nuevo(
            Nivel::Problema,
            "El show tiene vídeos o imágenes pero la proyección no está abierta.",
            "The show has videos or images but the projection is not open.",
        ));
    }
    if p.sucio {
        puntos.push(Punto::nuevo(Nivel::Aviso, "Hay cambios sin guardar (Ctrl+S).", "There are unsaved changes (Ctrl+S)."));
    }
    for (n, e) in &p.errores {
        puntos.push(Punto { nivel: Nivel::Problema, texto: Texto::nuevo(format!("Cue {n}: {}", e.es), format!("Cue {n}: {}", e.en)) });
    }
    let mut cues_bien = 0;
    for (k, (n, c)) in p.cues.iter().enumerate() {
        let antes = puntos.len();
        revisar_cue(p, n, c, &mut puntos);
        if !puntos[antes..].iter().any(|x| x.nivel != Nivel::Bien) {
            cues_bien += 1;
        }
        progreso(k + 1, total);
    }
    if total > 0 {
        puntos.push(Punto::nuevo(
            Nivel::Bien,
            format!("{cues_bien} de {total} cues sin nada que revisar."),
            format!("{cues_bien} of {total} cues with nothing to check."),
        ));
    }
    puntos
}

fn revisar_cue(p: &Pedido, n: &str, c: &Cue, puntos: &mut Vec<Punto>) {
    let (es, en) = nombre_cue(n, c);
    let ruta = match rutas::resolver(&p.carpeta, &c.archivo) {
        Ok(r) if r.is_file() => r,
        Ok(_) => {
            puntos.push(Punto::nuevo(
                Nivel::Problema,
                format!("{es}: falta el archivo {}.", c.archivo),
                format!("{en}: the file {} is missing.", c.archivo),
            ));
            return;
        }
        Err(e) => {
            puntos.push(Punto::nuevo(Nivel::Problema, format!("{es}: ruta no válida ({e})."), format!("{en}: invalid path ({e}).")));
            return;
        }
    };
    if c.salida == "Auriculares" && !p.auriculares && c.tipo != TipoCue::Imagen {
        puntos.push(Punto::nuevo(
            Nivel::Aviso,
            format!("{es} va a Auriculares, pero no hay auriculares elegidos: sonará por Principal (la sala)."),
            format!("{en} goes to Headphones, but no headphones are set up: it will play on Main (the house)."),
        ));
    }
    let (Some(fs), TipoCue::Audio | TipoCue::Video) = (p.frecuencia, c.tipo) else { return };
    let bucle = c.bucle && c.inicio_ms == 0 && c.fin_ms.is_none();
    match otaconvert_audio::preparar::preparar_con(&ruta, fs, bucle) {
        Err(otaconvert_audio::preparar::ErrorPreparar::SinAudio) if c.tipo == TipoCue::Video => {}
        Err(e) => {
            let nivel = if c.tipo == TipoCue::Audio { Nivel::Problema } else { Nivel::Aviso };
            let t = Texto::from(e);
            puntos.push(Punto::nuevo(nivel, format!("{es}: no se puede reproducir ({}).", t.es), format!("{en}: cannot be played ({}).", t.en)));
        }
        Ok(prep) => {
            // El volumen del cue también cuenta para el pico.
            let pico = prep.pico * 10f32.powf(c.volumen_db / 20.0);
            if pico > 1.0 {
                let db = 20.0 * pico.log10();
                puntos.push(Punto::nuevo(
                    Nivel::Aviso,
                    format!("{es} pasa de 0 dB (+{db:.1} dB con su volumen). El limitador lo controla, pero conviene bajarle el volumen."),
                    format!("{en} goes above 0 dB (+{db:.1} dB with its volume). The limiter handles it, but lowering its volume is better."),
                ));
            }
            if prep.paquetes_danados > 0 {
                puntos.push(Punto::nuevo(
                    Nivel::Aviso,
                    format!("{es}: el archivo tiene {} partes dañadas que se saltan (puede oírse un salto). Conviene volver a exportarlo.", prep.paquetes_danados),
                    format!("{en}: the file has {} damaged parts that are skipped (a jump may be heard). Re-exporting it is better.", prep.paquetes_danados),
                ));
            }
            // Arranque de golpe a mitad de onda: puede oírse un clic (CLAUDE.md, "Rampas cortas").
            if p.rampa_inicio_ms <= 0.0 && c.fundido_entrada_ms == 0 && empieza_a_mitad_de_onda(&prep.pista, fs, c.inicio_ms) {
                puntos.push(Punto::nuevo(
                    Nivel::Aviso,
                    format!("{es} empieza de golpe a mitad de onda: puede oírse un clic. Ponle un fundido inicial corto o activa la rampa al iniciar (Ajustes del show)."),
                    format!("{en} starts abruptly mid-wave: a click may be heard. Give it a short fade-in or turn on the start ramp (Show settings)."),
                ));
            }
        }
    }
}

/// ¿El primer cuadro que suena (en `inicio_ms`) es un salto desde el silencio?
/// Hay clic si el primer valor está lejos de 0 **y** es mucho más grande que
/// el paso al cuadro siguiente (una onda que arranca desde 0 sube de a poco).
fn empieza_a_mitad_de_onda(pista: &otaconvert_audio::Pista, fs: u32, inicio_ms: u64) -> bool {
    let canales = pista.canales() as usize;
    let cuadro = (inicio_ms as u128 * fs as u128 / 1000) as usize;
    let m = pista.muestras();
    let i = cuadro * canales;
    let (Some(a), Some(b)) = (m.get(i..i + canales), m.get(i + canales..i + 2 * canales)) else { return false };
    a.iter().zip(b).any(|(a, b)| a.abs() > 0.02 && a.abs() > 3.0 * (b - a).abs())
}

/// Resultado para el estado de la ventana: la luz y la lista.
pub fn a_json(puntos: &[Punto], idioma: crate::Idioma, hora: &str) -> Value {
    let luz = puntos.iter().map(|p| p.nivel).max().unwrap_or(Nivel::Bien);
    let mut ordenados: Vec<&Punto> = puntos.iter().collect();
    ordenados.sort_by_key(|p| std::cmp::Reverse(p.nivel));
    json!({
        "luz": luz.nombre(),
        "hora": hora,
        "puntos": ordenados.iter().map(|p| json!({ "nivel": p.nivel.nombre(), "texto": p.texto.en_idioma(idioma) })).collect::<Vec<_>>(),
    })
}

/// Para que el motor agregue lo del equipo (protector de pantalla, batería, pendrive…).
pub type ComprobarEquipo = std::sync::Arc<dyn Fn(&Path) -> Vec<Punto> + Send + Sync>;
