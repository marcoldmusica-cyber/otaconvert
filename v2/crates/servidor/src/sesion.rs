//! Un show abierto: cue en espera, qué suena, audios preparados, cambios sin
//! guardar. El GO dispara el audio del cue (etapa 2); vídeo e imagen llegan
//! en la etapa 4.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use otaconvert_audio::{Comando, Pista};
use otaconvert_show::{abrir, guardar, marca_de_tiempo, rutas, ErrorShow, Registro, Show, TipoCue, EXTENSION, SALIDA_AURICULARES};
use serde_json::json;

use crate::sonido::{Destino, Sonido};
use crate::texto::{Idioma, Texto};

/// Cuántos cues desde el que está en espera se preparan de antemano.
pub const PREPARAR_ADELANTE: usize = 3;

struct Sonando {
    id: u32,
    cue: usize,
    destino: Destino,
    inicio: Instant,
    /// `None` = bucle (no termina solo).
    duracion: Option<Duration>,
    /// Duración de una vuelta del cue (para la barra de progreso).
    largo: Duration,
    /// Se está fundiendo (F o Mayús+F).
    fundiendo: bool,
}

/// Edición prepara el show; Función es para la función de verdad: el motor
/// rechaza ahí cualquier cambio del show (no solo la ventana).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Modo {
    Edicion,
    Funcion,
}

/// Límites de lo que se puede escribir desde la ventana.
const MAX_NOMBRE: usize = 200;
const MAX_NOTA: usize = 2000;
const MAX_FUNDIDO_MS: u32 = 10 * 60 * 1000;

pub struct Sesion {
    pub show: Show,
    pub ruta: PathBuf,
    pub en_espera: usize,
    pub ultimo_disparado: Option<usize>,
    /// Hay cambios sin guardar.
    pub sucio: bool,
    /// Aviso para la ventana, en los dos idiomas.
    pub aviso: Option<Texto>,
    /// Idioma del programa (lo elige la ventana): avisos y errores salen en él.
    pub idioma: Idioma,
    pub pausado: bool,
    pub modo: Modo,
    registro: Registro,
    ultimo_go: Option<Instant>,
    sonido: Option<Box<dyn Sonido>>,
    preparados: HashMap<usize, Arc<Pista>>,
    errores: HashMap<usize, Texto>,
    preparando: Option<usize>,
    sonando: Vec<Sonando>,
    siguiente_id: u32,
    pausa_desde: Option<Instant>,
}

/// Busca el archivo del show: la ruta misma, o el único `.otashow` de la carpeta.
pub fn buscar_archivo(ruta: &Path) -> Result<PathBuf, ErrorShow> {
    if ruta.is_file() {
        return Ok(ruta.to_path_buf());
    }
    let leidos = std::fs::read_dir(ruta).map_err(|e| ErrorShow::NoSePudoLeer(format!("{}: {e}", ruta.display())))?;
    let shows: Vec<PathBuf> = leidos
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some(EXTENSION))
        .collect();
    match shows.as_slice() {
        [uno] => Ok(uno.clone()),
        [] => Err(ErrorShow::NoSePudoLeer(format!("no hay ningún archivo .{EXTENSION} en {}", ruta.display()))),
        _ => Err(ErrorShow::NoSePudoLeer(format!("hay más de un archivo .{EXTENSION} en {}; elige uno", ruta.display()))),
    }
}

/// Atajo: un mensaje en español y en inglés.
fn t(es: impl Into<String>, en: impl Into<String>) -> Texto {
    Texto::nuevo(es, en)
}

fn db_a_lineal(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

impl Sesion {
    pub fn abrir(ruta: &Path) -> Result<Self, ErrorShow> {
        let archivo = buscar_archivo(ruta)?;
        let show = abrir(&archivo)?;
        let carpeta = archivo.parent().unwrap_or(Path::new(".")).to_path_buf();
        let mut registro = Registro::abrir(&carpeta);
        registro.anotar("ABRIR", &format!("Show «{}», {} cues", show.nombre, show.cues.len()));
        Ok(Self {
            show,
            ruta: archivo,
            en_espera: 0,
            ultimo_disparado: None,
            sucio: false,
            aviso: None,
            idioma: Idioma::Es,
            pausado: false,
            modo: Modo::Edicion,
            registro,
            ultimo_go: None,
            sonido: None,
            preparados: HashMap::new(),
            errores: HashMap::new(),
            preparando: None,
            sonando: Vec::new(),
            siguiente_id: 1,
            pausa_desde: None,
        })
    }

    pub fn carpeta(&self) -> PathBuf {
        self.ruta.parent().unwrap_or(Path::new(".")).to_path_buf()
    }

    /// Conecta el sonido y le pasa las rampas y el volumen maestro del show.
    pub fn conectar_sonido(&mut self, mut s: Box<dyn Sonido>) {
        let fs = s.frecuencia();
        let r = &self.show.ajustes.rampas_ms;
        let cuadros = |ms: f32| otaconvert_audio::ms(fs, ms);
        let ajustes = otaconvert_audio::Ajustes {
            rampa_inicio: cuadros(r.inicio),
            rampa_reanudar: cuadros(r.reanudar),
            rampa_pausa: cuadros(r.pausa),
            rampa_parada: cuadros(r.parada),
            rampa_salto: cuadros(r.salto),
            rampa_volumen: cuadros(r.volumen),
            rampa_fin: cuadros(r.fin),
            ..otaconvert_audio::Ajustes::para(fs)
        };
        let maestro = db_a_lineal(self.show.ajustes.volumen_maestro_db);
        for d in [Destino::Principal, Destino::Auriculares] {
            if d == Destino::Auriculares && !s.tiene_auriculares() {
                continue;
            }
            let _ = s.enviar(d, Comando::CambiarAjustes(ajustes));
            let _ = s.enviar(d, Comando::VolumenMaestro { volumen: maestro, cuadros: 0 });
        }
        self.registro.anotar("SONIDO", &format!("salida conectada a {fs} Hz"));
        self.sonido = Some(s);
        self.preparados.clear();
        self.errores.clear();
    }

    fn describir(&self, i: usize) -> String {
        let c = &self.show.cues[i];
        let n = if c.numero.is_empty() { (i + 1).to_string() } else { c.numero.clone() };
        format!("Cue {n} «{}»", c.nombre)
    }

    /// Lo mismo que `describir`, en inglés.
    fn describir_en(&self, i: usize) -> String {
        let c = &self.show.cues[i];
        let n = if c.numero.is_empty() { (i + 1).to_string() } else { c.numero.clone() };
        format!("Cue {n} “{}”", c.nombre)
    }

    pub fn cambiar_idioma(&mut self, idioma: Idioma) {
        self.idioma = idioma;
    }

    fn cuadros(&self, ms: u64) -> usize {
        let fs = self.sonido.as_ref().map(|s| s.frecuencia()).unwrap_or(48_000) as u64;
        (ms * fs / 1000) as usize
    }

    fn destino_de(&mut self, i: usize) -> Destino {
        let quiere_auriculares = self.show.cues[i].salida == SALIDA_AURICULARES;
        let hay = self.sonido.as_ref().is_some_and(|s| s.tiene_auriculares());
        if quiere_auriculares && !hay {
            self.aviso = Some(Texto::nuevo(
                format!("{}: la salida Auriculares no está configurada; suena por Principal.", self.describir(i)),
                format!("{}: the Headphones output is not set up; it plays through Main.", self.describir_en(i)),
            ));
            Destino::Principal
        } else if quiere_auriculares {
            Destino::Auriculares
        } else {
            Destino::Principal
        }
    }

    /// Envía a una salida o a las dos.
    fn a_todas(&mut self, c: impl Fn() -> Comando) {
        if let Some(s) = self.sonido.as_mut() {
            let _ = s.enviar(Destino::Principal, c());
            if s.tiene_auriculares() {
                let _ = s.enviar(Destino::Auriculares, c());
            }
        }
    }

    /// GO: dispara el cue en espera y deja en espera el siguiente.
    /// Devuelve `false` si se ignoró.
    pub fn go(&mut self) -> bool {
        if self.show.cues.is_empty() || self.en_espera >= self.show.cues.len() {
            self.aviso = Some(Texto::nuevo("No hay ningún cue en espera.", "There is no cue on standby."));
            return false;
        }
        let minimo = Duration::from_millis(self.show.ajustes.tiempo_minimo_entre_go_ms as u64);
        if let Some(t) = self.ultimo_go {
            if t.elapsed() < minimo {
                self.aviso = Some(Texto::nuevo(
                    "GO ignorado: llegó demasiado rápido después del anterior (protección contra doble GO).",
                    "GO ignored: it came too soon after the previous one (double-GO protection).",
                ));
                let d = self.describir(self.en_espera);
                self.registro.anotar("IGNORADO", &format!("doble GO; seguía en espera {d}"));
                return false;
            }
        }
        let i = self.en_espera;
        let d = self.describir(i);
        self.aviso = None;
        if self.show.cues[i].tipo == TipoCue::Audio && self.sonido.is_some() {
            if let Some(e) = self.errores.get(&i).cloned() {
                // No se puede sonar, pero la función sigue: se avanza y se avisa.
                let den = self.describir_en(i);
                self.aviso = Some(Texto::nuevo(format!("{d} no sonó: {}", e.es), format!("{den} did not play: {}", e.en)));
                self.registro.anotar("ERROR", &format!("{d}: {}", e.es));
            } else if let Some(p) = self.preparados.get(&i).cloned() {
                self.disparar(i, p);
            } else {
                self.aviso = Some(Texto::nuevo(
                    format!("{d} todavía se está preparando; vuelve a dar GO en un momento."),
                    format!("{} is still being prepared; press GO again in a moment.", self.describir_en(i)),
                ));
                self.registro.anotar("IGNORADO", &format!("{d} sin preparar"));
                return false;
            }
        } else if self.show.cues[i].tipo != TipoCue::Audio {
            self.aviso = Some(Texto::nuevo(
                "Los cues de vídeo e imagen llegan en la etapa 4; se pasa al siguiente.",
                "Video and image cues arrive in stage 4; moving to the next one.",
            ));
        }
        self.ultimo_go = Some(Instant::now());
        self.registro.anotar("GO", &d);
        self.ultimo_disparado = Some(i);
        self.en_espera = i + 1;
        true
    }

    fn disparar(&mut self, i: usize, pista: Arc<Pista>) {
        let c = self.show.cues[i].clone();
        let destino = self.destino_de(i);
        let id = self.siguiente_id;
        self.siguiente_id = self.siguiente_id.wrapping_add(1).max(1);
        let total = pista.cuadros();
        let inicio = self.cuadros(c.inicio_ms).min(total.saturating_sub(1));
        let hasta = c.fin_ms.map(|f| self.cuadros(f).min(total));
        let fin = hasta.unwrap_or(total);
        let fs = pista.frecuencia() as f64;
        let comando = Comando::IniciarCue {
            id,
            pista,
            volumen: db_a_lineal(c.volumen_db),
            inicio,
            desde: inicio,
            hasta,
            bucle: c.bucle,
            fundido_entrada: self.cuadros(c.fundido_entrada_ms as u64) as u32,
            fundido_salida: self.cuadros(c.fundido_salida_ms as u64) as u32,
        };
        let enviado = self.sonido.as_mut().map(|s| s.enviar(destino, comando));
        if let Some(Err(e)) = enviado {
            self.aviso = Some(Texto::nuevo(format!("No se pudo disparar: {e}"), format!("Could not fire the cue: {e}")));
            return;
        }
        let largo = Duration::from_secs_f64(fin.saturating_sub(inicio) as f64 / fs);
        self.sonando.push(Sonando {
            id,
            cue: i,
            destino,
            inicio: Instant::now(),
            duracion: (!c.bucle).then_some(largo),
            largo,
            fundiendo: false,
        });
    }

    /// Cambia entre Edición y Función (Tab). Nunca toca lo que suena.
    pub fn cambiar_modo(&mut self, modo: Modo) {
        if self.modo != modo {
            self.modo = modo;
            self.registro.anotar("MODO", if modo == Modo::Funcion { "Función" } else { "Edición" });
        }
    }

    /// En Edición, clic en un cue: queda en espera.
    pub fn ir_a(&mut self, i: usize) {
        if self.modo == Modo::Funcion {
            self.aviso = Some(Texto::nuevo("En Función no se elige el cue con el ratón: usa ↑ y ↓.", "In Live mode the cue is not chosen with the mouse: use ↑ and ↓."));
            return;
        }
        if i < self.show.cues.len() {
            self.en_espera = i;
        }
    }

    /// En Edición, cambia campos de un cue. Se valida todo antes de aplicar;
    /// si algo no vale, no se cambia nada y se avisa. Un cue que suena solo
    /// se edita con `forzar` (la ventana lo pide confirmando). Los cambios se
    /// usan desde el próximo GO: lo que ya suena no se toca.
    pub fn editar_cue(&mut self, i: usize, cambios: &serde_json::Value, forzar: bool) -> Result<(), Texto> {
        if self.modo == Modo::Funcion {
            return Err(t(
                "En Función no se puede cambiar el show. Pasa a Edición con Tab.",
                "The show can't be changed in Live mode. Switch to Edit with Tab.",
            ));
        }
        let Some(original) = self.show.cues.get(i) else {
            return Err(t("Ese cue no existe.", "That cue does not exist."));
        };
        if !forzar && self.sonando.iter().any(|s| s.cue == i) {
            return Err(t(
                "Este cue está sonando: páralo o confirma para editarlo.",
                "This cue is playing: stop it or confirm to edit it.",
            ));
        }
        let Some(campos) = cambios.as_object() else {
            return Err(t("Cambios no válidos.", "Invalid changes."));
        };
        let mut c = original.clone();
        for (k, v) in campos {
            match k.as_str() {
                "nombre" => {
                    let s = v.as_str().ok_or_else(|| t("El nombre debe ser texto.", "The name must be text."))?;
                    if s.chars().count() > MAX_NOMBRE {
                        return Err(t(
                            format!("El nombre puede tener hasta {MAX_NOMBRE} letras."),
                            format!("The name can have up to {MAX_NOMBRE} characters."),
                        ));
                    }
                    c.nombre = s.replace(['\r', '\n'], " ");
                }
                "nota" => {
                    let s = v.as_str().ok_or_else(|| t("La nota debe ser texto.", "The note must be text."))?;
                    if s.chars().count() > MAX_NOTA {
                        return Err(t(
                            format!("La nota puede tener hasta {MAX_NOTA} letras."),
                            format!("The note can have up to {MAX_NOTA} characters."),
                        ));
                    }
                    c.nota = s.to_string();
                }
                "volumen_db" => {
                    let rango = || t("El volumen va de −60 dB a +24 dB.", "Volume goes from −60 dB to +24 dB.");
                    let x = v.as_f64().ok_or_else(rango)? as f32;
                    if !x.is_finite() || !(-60.0..=24.0).contains(&x) {
                        return Err(rango());
                    }
                    c.volumen_db = x;
                }
                "fundido_entrada_ms" | "fundido_salida_ms" => {
                    let x = v
                        .as_u64()
                        .filter(|&x| x <= MAX_FUNDIDO_MS as u64)
                        .ok_or_else(|| t("El fundido va de 0 a 600 segundos.", "A fade goes from 0 to 600 seconds."))? as u32;
                    if k == "fundido_entrada_ms" {
                        c.fundido_entrada_ms = x;
                    } else {
                        c.fundido_salida_ms = x;
                    }
                }
                "bucle" => c.bucle = v.as_bool().ok_or_else(|| t("Bucle debe ser sí o no.", "Loop must be yes or no."))?,
                "salida" => {
                    let s = v.as_str().unwrap_or("");
                    if s != otaconvert_show::SALIDA_PRINCIPAL && s != SALIDA_AURICULARES {
                        return Err(t("La salida debe ser Principal o Auriculares.", "The output must be Main or Headphones."));
                    }
                    c.salida = s.to_string();
                }
                _ => {
                    return Err(t(format!("No se puede cambiar «{k}» desde aquí."), format!("“{k}” can't be changed from here.")));
                }
            }
        }
        let mut prueba = self.show.clone();
        prueba.cues[i] = c;
        // El detalle de `validar` está en español (viene del archivo del show).
        otaconvert_show::validar(&prueba).map_err(|e| t(format!("Valor no válido: {e}"), format!("Invalid value: {e}")))?;
        if prueba.cues[i] != self.show.cues[i] {
            self.show = prueba;
            self.sucio = true;
            let d = self.describir(i);
            self.registro.anotar("EDITAR", &d);
        }
        Ok(())
    }

    pub fn mover_espera(&mut self, delta: i64) {
        if self.show.cues.is_empty() {
            return;
        }
        let max = self.show.cues.len() as i64;
        self.en_espera = (self.en_espera as i64 + delta).clamp(0, max) as usize;
    }

    pub fn parar_todo(&mut self) {
        self.a_todas(|| Comando::PararTodo { fundido: 0 });
        self.sonando.clear();
        self.pausado = false;
        self.pausa_desde = None;
        self.registro.anotar("PARAR", "Parar todo");
        self.aviso = None;
    }

    /// Fundir todo (F): baja todo en `fundir_todo_ms`.
    pub fn fundir_todo(&mut self) {
        let f = self.cuadros(self.show.ajustes.fundir_todo_ms as u64) as u32;
        self.a_todas(|| Comando::PararTodo { fundido: f });
        self.registro.anotar("FUNDIR", "Fundir todo");
        let fin = Instant::now() + Duration::from_millis(self.show.ajustes.fundir_todo_ms as u64);
        for s in &mut self.sonando {
            s.duracion = Some(fin.saturating_duration_since(s.inicio).min(s.duracion.unwrap_or(Duration::MAX)));
            s.fundiendo = true;
        }
    }

    /// Mayús+F: funde solo el último cue que se disparó (pedido del usuario).
    pub fn fundir_ultimo(&mut self) {
        // El último que todavía no se está fundiendo: un segundo Mayús+F
        // funde el anterior.
        let Some(s) = self.sonando.iter().rev().find(|s| !s.fundiendo) else {
            self.aviso = Some(Texto::nuevo("No hay nada sonando para fundir.", "Nothing is playing to fade out."));
            return;
        };
        let (id, destino, cue) = (s.id, s.destino, s.cue);
        let f = self.cuadros(self.show.ajustes.fundir_todo_ms as u64) as u32;
        if let Some(son) = self.sonido.as_mut() {
            let _ = son.enviar(destino, Comando::Parar { id, fundido: f });
        }
        let d = self.describir(cue);
        self.registro.anotar("FUNDIR", &format!("solo {d}"));
        let fin = Instant::now() + Duration::from_millis(self.show.ajustes.fundir_todo_ms as u64);
        if let Some(s) = self.sonando.iter_mut().find(|s| s.id == id) {
            s.duracion = Some(fin.saturating_duration_since(s.inicio).min(s.duracion.unwrap_or(Duration::MAX)));
            s.fundiendo = true;
        }
    }

    /// Pausa general (P): la primera vez pausa todo, la segunda lo reanuda.
    pub fn pausa(&mut self) {
        if self.pausado {
            self.a_todas(|| Comando::ReanudarTodo);
            if let Some(t) = self.pausa_desde.take() {
                let pausa = t.elapsed();
                for s in &mut self.sonando {
                    s.inicio += pausa;
                }
            }
            self.pausado = false;
            self.registro.anotar("SIGUE", "Fin de la pausa");
        } else {
            self.a_todas(|| Comando::PausarTodo);
            self.pausado = true;
            self.pausa_desde = Some(Instant::now());
            self.registro.anotar("PAUSA", "Pausa general");
        }
    }

    pub fn guardar(&mut self) -> Result<(), ErrorShow> {
        guardar(&self.show, &self.ruta, &marca_de_tiempo())?;
        self.sucio = false;
        self.registro.anotar("GUARDAR", "Show guardado");
        Ok(())
    }

    /// Guardado automático: guarda solo si hay cambios.
    pub fn guardar_si_hace_falta(&mut self) {
        if self.sucio {
            if let Err(e) = self.guardar() {
                self.aviso = Some(Texto::nuevo(format!("No se pudo guardar solo: {e}"), format!("Autosave failed: {e}")));
                self.registro.anotar("ERROR", &e.to_string());
            }
        }
    }

    pub fn anotar_error(&mut self, texto: &str) {
        self.registro.anotar("ERROR", texto);
    }

    /// Anota un evento cualquiera en el registro de la función.
    pub fn anotar(&mut self, tipo: &str, texto: &str) {
        self.registro.anotar(tipo, texto);
    }

    /// Quita de "sonando" lo que ya terminó y libera memoria del mezclador.
    pub fn limpiar(&mut self) {
        let ahora = Instant::now();
        let pausado = self.pausado;
        self.sonando.retain(|s| pausado || s.duracion.is_none_or(|d| ahora.duration_since(s.inicio) < d + Duration::from_millis(200)));
        if let Some(s) = self.sonido.as_mut() {
            s.recoger_basura();
        }
    }

    /// Qué audio preparar ahora: el cue en espera y los siguientes. Además
    /// suelta los preparados que ya no hacen falta. Devuelve (cue, ruta, frecuencia).
    pub fn siguiente_a_preparar(&mut self) -> Option<(usize, PathBuf, u32)> {
        let fs = self.sonido.as_ref()?.frecuencia();
        let quiero: Vec<usize> = (self.en_espera..self.show.cues.len())
            .filter(|&i| self.show.cues[i].tipo == TipoCue::Audio)
            .take(PREPARAR_ADELANTE)
            .collect();
        let sonando: Vec<usize> = self.sonando.iter().map(|s| s.cue).collect();
        self.preparados.retain(|i, _| quiero.contains(i) || sonando.contains(i));
        let carpeta = self.carpeta();
        for i in quiero {
            if self.preparados.contains_key(&i) || self.errores.contains_key(&i) || self.preparando == Some(i) {
                continue;
            }
            match rutas::resolver(&carpeta, &self.show.cues[i].archivo) {
                Ok(r) if r.is_file() => {
                    self.preparando = Some(i);
                    return Some((i, r, fs));
                }
                Ok(_) => {
                    let a = &self.show.cues[i].archivo;
                    let e = Texto::nuevo(format!("no se encontró el archivo {a}"), format!("file not found: {a}"));
                    self.registro.anotar("ERROR", &format!("{}: {}", self.describir(i), e.es));
                    self.errores.insert(i, e);
                }
                Err(e) => {
                    // Ruta no válida (el detalle viene del archivo del show, en español).
                    self.errores.insert(i, Texto::nuevo(e.to_string(), format!("invalid file path: {e}")));
                }
            }
        }
        None
    }

    /// Guarda el resultado de preparar el cue `i`.
    pub fn preparado<E: Into<Texto>>(&mut self, i: usize, resultado: Result<otaconvert_audio::preparar::Preparado, E>) {
        if self.preparando == Some(i) {
            self.preparando = None;
        }
        match resultado {
            Ok(p) => {
                if p.pico > 1.0 {
                    let d = self.describir(i);
                    self.registro.anotar("AVISO", &format!("{d}: el audio pasa de 0 dB (pico {:.2}); el limitador lo controla", p.pico));
                }
                self.preparados.insert(i, Arc::new(p.pista));
            }
            Err(e) => {
                let e: Texto = e.into();
                let d = self.describir(i);
                self.registro.anotar("ERROR", &format!("{d}: {}", e.es));
                self.errores.insert(i, e);
            }
        }
    }

    pub fn esta_preparado(&self, i: usize) -> bool {
        self.preparados.contains_key(&i)
    }

    /// Lo que la ventana necesita para dibujarse.
    pub fn estado_json(&self) -> String {
        let cues: Vec<_> = self
            .show
            .cues
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let preparacion = if c.tipo != TipoCue::Audio || self.sonido.is_none() {
                    ""
                } else if self.errores.contains_key(&i) {
                    "error"
                } else if self.preparados.contains_key(&i) {
                    "listo"
                } else if self.preparando == Some(i) {
                    "preparando"
                } else {
                    ""
                };
                // Duración de una vuelta, cuando el audio ya está preparado.
                let duracion_s = self.preparados.get(&i).map(|p| {
                    let fs = p.frecuencia() as f64;
                    let total = p.cuadros() as f64 / fs;
                    let ini = (c.inicio_ms as f64 / 1000.0).min(total);
                    let fin = c.fin_ms.map(|f| (f as f64 / 1000.0).min(total)).unwrap_or(total);
                    (fin - ini).max(0.0)
                });
                json!({
                    "numero": if c.numero.is_empty() { (i + 1).to_string() } else { c.numero.clone() },
                    "nombre": c.nombre,
                    "tipo": match c.tipo { TipoCue::Audio => "audio", TipoCue::Video => "video", TipoCue::Imagen => "imagen" },
                    "archivo": c.archivo,
                    "nota": c.nota,
                    "bucle": c.bucle,
                    "salida": c.salida,
                    "volumen_db": c.volumen_db,
                    "fundido_entrada_ms": c.fundido_entrada_ms,
                    "fundido_salida_ms": c.fundido_salida_ms,
                    "duracion_s": duracion_s,
                    "preparacion": preparacion,
                    "error": self.errores.get(&i).map(|e| e.en_idioma(self.idioma)),
                })
            })
            .collect();
        let ahora = Instant::now();
        let sonando: Vec<_> = self
            .sonando
            .iter()
            .map(|s| {
                let transcurrido = self.pausa_desde.unwrap_or(ahora).saturating_duration_since(s.inicio);
                let largo = s.largo.as_secs_f64();
                let pasado = transcurrido.as_secs_f64();
                json!({
                    "cue": s.cue,
                    "nombre": self.show.cues.get(s.cue).map(|c| c.nombre.clone()).unwrap_or_default(),
                    "restante_s": s.duracion.map(|d| d.saturating_sub(transcurrido).as_secs_f64()),
                    // Progreso de la vuelta actual, de 0 a 1 (en bucle vuelve a 0).
                    "progreso": if largo > 0.0 { ((pasado % largo) / largo).min(1.0) } else { 0.0 },
                    "fundiendo": s.fundiendo,
                    "salida": if s.destino == Destino::Auriculares { "Auriculares" } else { "Principal" },
                })
            })
            .collect();
        json!({
            "tipo": "estado",
            "show": self.show.nombre,
            "cues": cues,
            "en_espera": self.en_espera,
            "ultimo": self.ultimo_disparado,
            "sucio": self.sucio,
            "aviso": self.aviso.as_ref().map(|a| a.en_idioma(self.idioma)),
            "idioma": if self.idioma == Idioma::En { "en" } else { "es" },
            "pausado": self.pausado,
            "sonando": sonando,
            "con_sonido": self.sonido.is_some(),
            "auriculares": self.sonido.as_ref().is_some_and(|s| s.tiene_auriculares()),
            "modo": if self.modo == Modo::Funcion { "funcion" } else { "edicion" },
        })
        .to_string()
    }
}
