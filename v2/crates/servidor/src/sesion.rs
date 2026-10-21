//! Un show abierto: cue en espera, qué suena, audios preparados, cambios sin
//! guardar. El GO dispara el audio del cue (etapa 2); vídeo e imagen llegan
//! en la etapa 4.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use otaconvert_audio::{Comando, Pista};
use otaconvert_show::{abrir, guardar, marca_de_tiempo, rutas, Cue, ErrorShow, Registro, Show, TipoCue, EXTENSION, SALIDA_AURICULARES};
use serde_json::json;

use crate::sonido::{Destino, Sonido};
use crate::proyector::{AvisoProyector, OrdenProyector, Proyector};
use crate::texto::{Idioma, Texto};

/// Cuántos cues de vídeo o imagen se precargan desde el que está en espera.
pub const PRECARGAR_VISUALES: usize = 2;

/// Estado de un vídeo o una imagen en la proyección.
#[derive(Debug, Clone, PartialEq)]
enum Visual {
    Pidiendo,
    Listo { duracion: Option<f64> },
    Error(Texto),
}

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
    /// Es un vídeo o una imagen (va por la proyección, no por el mezclador).
    visual: bool,
    /// Tiene una voz en el mezclador con el `id` de arriba (un audio, o el sonido de un vídeo).
    voz: bool,
    /// Vídeo o imagen al aire: la duración que dio la proyección al precargarlo
    /// (para volver a darle GO mientras se ve: vuelve a empezar).
    duracion_visual: Option<f64>,
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
    /// Salidas elegidas en este equipo (las pone el motor; la ventana las muestra en
    /// Ajustes del show → Salidas).
    pub equipo: serde_json::Value,
    /// Cuántos pedidos de cambio de salida terminó el motor (bien o mal): la
    /// ventana sabe así que el suyo ya tuvo respuesta.
    pub salidas_respondidas: u64,
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
    /// Cómo se preparó cada audio: en bucle de punta a punta o suelto (cambia
    /// el principio y el final al adaptar la frecuencia; ver `preparar_con`).
    preparado_en_bucle: HashMap<usize, bool>,
    sonando: Vec<Sonando>,
    siguiente_id: u32,
    pausa_desde: Option<Instant>,
    proyector: Option<Box<dyn Proyector>>,
    visuales: HashMap<usize, Visual>,
    /// Tras reiniciarse la proyección: el cue que se veía, para volver a mostrarlo.
    remostrar: Option<usize>,
    /// Vídeos sin sonido (no es un error).
    mudos: std::collections::HashSet<usize>,
    /// Fotos que no se pudieron hacer (no se reintentan en esta sesión).
    miniaturas_fallidas: std::collections::HashSet<usize>,
    /// Fotos ya hechas: cue → versión (nombre del archivo de la foto).
    miniaturas_hechas: HashMap<usize, String>,
    /// "Salir desde aquí" (Vista Estudio): cue en espera y segundo del archivo
    /// desde el que sale con el próximo GO. `None` = desde su inicio.
    desde_elegido: Option<(usize, f64)>,
    /// Patrón de prueba a la vista en el proyector (para encajar las esquinas).
    patron: bool,
    /// Deshacer y rehacer (etapa 5A): cómo era el show antes de cada cambio.
    historial: Vec<Show>,
    rehechos: Vec<Show>,
    /// Último cambio (tipo y cuándo): varios movimientos seguidos de las
    /// esquinas cuentan como uno solo para deshacer.
    ultimo_cambio: Option<(&'static str, Instant)>,
    /// La preparación en curso quedó vieja (cambió el orden de los cues):
    /// su resultado se descarta y se vuelve a pedir.
    descartar_preparacion: bool,
}

/// Cuántos pasos se pueden deshacer.
const MAX_DESHACER: usize = 100;
/// Largo máximo del número de un cue ("1", "1.5", "2A"…).
const MAX_NUMERO: usize = 20;

/// Un audio para preparar en segundo plano (sin tener la sesión bloqueada).
#[derive(Debug, Clone)]
pub struct TareaPreparar {
    pub cue: usize,
    pub ruta: PathBuf,
    pub frecuencia: u32,
    /// Se repite de punta a punta del archivo.
    pub bucle: bool,
}

impl TareaPreparar {
    pub fn hacer(&self) -> Result<otaconvert_audio::preparar::Preparado, otaconvert_audio::preparar::ErrorPreparar> {
        otaconvert_audio::preparar::preparar_con(&self.ruta, self.frecuencia, self.bucle)
    }
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
        let mut show = abrir(&archivo)?;
        // Cada cue con un número interno distinto (los shows viejos o hechos
        // a mano pueden traer repetidos): si hay repetidos, se numeran en orden.
        let mut vistos = std::collections::HashSet::new();
        if !show.cues.iter().all(|c| vistos.insert(c.id)) {
            for (i, c) in show.cues.iter_mut().enumerate() {
                c.id = i as u64;
            }
        }
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
            preparado_en_bucle: HashMap::new(),
            errores: HashMap::new(),
            preparando: None,
            sonando: Vec::new(),
            siguiente_id: 1,
            pausa_desde: None,
            proyector: None,
            visuales: HashMap::new(),
            remostrar: None,
            mudos: std::collections::HashSet::new(),
            miniaturas_fallidas: std::collections::HashSet::new(),
            miniaturas_hechas: HashMap::new(),
            desde_elegido: None,
            patron: false,
            equipo: serde_json::Value::Null,
            salidas_respondidas: 0,
            historial: Vec::new(),
            rehechos: Vec::new(),
            ultimo_cambio: None,
            descartar_preparacion: false,
        })
    }

    /// Lugar en la lista del cue con este número interno.
    pub fn indice_de_id(&self, id: u64) -> Option<usize> {
        self.show.cues.iter().position(|c| c.id == id)
    }

    fn nuevo_id(&self) -> u64 {
        self.show.cues.iter().map(|c| c.id).max().map_or(0, |m| m + 1)
    }

    /// Solo en Edición se cambia el show.
    /// Para cambiar la tarjeta o la pantalla (etapa 5B): solo en Edición y con
    /// nada sonando ni a la vista, así el cambio no corta nada de la función.
    pub fn se_pueden_cambiar_salidas(&self) -> Result<(), Texto> {
        self.solo_en_edicion()?;
        if !self.sonando.is_empty() {
            return Err(t("Para cambiar la salida, primero para todo (Esc): no se cambia con algo sonando o a la vista.", "To change the output, stop everything first (Esc): it is never changed while something plays or is on screen."));
        }
        Ok(())
    }

    fn solo_en_edicion(&self) -> Result<(), Texto> {
        if self.modo == Modo::Funcion {
            return Err(t("En Función no se puede cambiar el show. Pasa a Edición con Tab.", "The show can't be changed in Live mode. Switch to Edit with Tab."));
        }
        Ok(())
    }

    /// Guarda cómo era el show antes de un cambio (para deshacer). Varios
    /// cambios seguidos del mismo `grupo` en poco tiempo cuentan como uno.
    fn antes_de_cambiar(&mut self, grupo: Option<&'static str>) {
        let junto = matches!((grupo, self.ultimo_cambio), (Some(g), Some((u, cuando))) if g == u && cuando.elapsed() < Duration::from_millis(1500));
        if !junto {
            self.historial.push(self.show.clone());
            if self.historial.len() > MAX_DESHACER {
                self.historial.remove(0);
            }
        }
        self.rehechos.clear();
        self.ultimo_cambio = grupo.map(|g| (g, Instant::now()));
        self.sucio = true;
    }

    /// Olvida lo preparado y precargado de un cue (cambió algo que lo afecta):
    /// se vuelve a preparar. Lo que suena sigue igual hasta el próximo GO.
    fn invalidar_cue(&mut self, i: usize) {
        if self.sonando.iter().any(|s| s.cue == i) {
            return;
        }
        self.preparados.remove(&i);
        self.preparado_en_bucle.remove(&i);
        self.errores.remove(&i);
        self.mudos.remove(&i);
        if self.preparando == Some(i) {
            self.preparando = None;
            self.descartar_preparacion = true;
        }
        if self.visuales.remove(&i).is_some() {
            if let Some(id) = self.show.cues.get(i).map(|c| c.id) {
                self.al_proyector(OrdenProyector::Quitar { id });
            }
        }
    }

    /// Tras cambiar el orden o la cantidad de cues, todo lo que se recuerda por
    /// lugar en la lista pasa al lugar nuevo de su cue (por su número interno).
    /// Lo de cues que ya no están se suelta, y si sonaban se paran.
    fn reindexar(&mut self, ids_antes: &[u64], espera_antes: Option<u64>) {
        let mapa: HashMap<usize, usize> =
            ids_antes.iter().enumerate().filter_map(|(viejo, id)| self.indice_de_id(*id).map(|nuevo| (viejo, nuevo))).collect();
        let m = |i: usize| mapa.get(&i).copied();
        // Precargados de cues que ya no están: se sueltan en la proyección.
        for (&k, _) in self.visuales.iter() {
            if m(k).is_none() {
                if let Some(&id) = ids_antes.get(k) {
                    if let Some(p) = self.proyector.as_mut() {
                        p.enviar(OrdenProyector::Quitar { id });
                    }
                }
            }
        }
        fn mover<V>(h: HashMap<usize, V>, m: &impl Fn(usize) -> Option<usize>) -> HashMap<usize, V> {
            h.into_iter().filter_map(|(k, v)| m(k).map(|n| (n, v))).collect()
        }
        self.preparados = mover(std::mem::take(&mut self.preparados), &m);
        self.preparado_en_bucle = mover(std::mem::take(&mut self.preparado_en_bucle), &m);
        self.errores = mover(std::mem::take(&mut self.errores), &m);
        self.visuales = mover(std::mem::take(&mut self.visuales), &m);
        self.miniaturas_hechas = mover(std::mem::take(&mut self.miniaturas_hechas), &m);
        self.mudos = self.mudos.iter().filter_map(|&k| m(k)).collect();
        self.miniaturas_fallidas = self.miniaturas_fallidas.iter().filter_map(|&k| m(k)).collect();
        if self.preparando.take().is_some() {
            self.descartar_preparacion = true;
        }
        self.remostrar = self.remostrar.and_then(m);
        self.ultimo_disparado = self.ultimo_disparado.and_then(m);
        self.desde_elegido = self.desde_elegido.and_then(|(k, s)| m(k).map(|n| (n, s)));
        // Lo que suena de cues que ya no están se para.
        let mut parar_voces = Vec::new();
        let mut negro = false;
        for s in &mut self.sonando {
            match m(s.cue) {
                Some(n) => s.cue = n,
                None => {
                    if s.voz {
                        parar_voces.push((s.id, s.destino));
                    }
                    negro |= s.visual;
                    s.cue = usize::MAX;
                }
            }
        }
        self.sonando.retain(|s| s.cue != usize::MAX);
        let parada = self.cuadros(self.show.ajustes.rampas_ms.parada.max(0.0) as u64) as u32;
        for (id, destino) in parar_voces {
            if let Some(son) = self.sonido.as_mut() {
                let _ = son.enviar(destino, Comando::Parar { id, fundido: parada });
            }
        }
        if negro {
            self.al_proyector(OrdenProyector::Negro);
        }
        // El cue en espera sigue siendo el mismo (si todavía está).
        self.en_espera = match espera_antes.and_then(|id| self.indice_de_id(id)) {
            Some(i) => i,
            None => self.en_espera.min(self.show.cues.len()),
        };
    }

    /// Pone otro show en lugar del actual (deshacer y rehacer): reacomoda lo
    /// recordado y olvida lo preparado de los cues que cambiaron.
    fn poner_show(&mut self, nuevo: Show) {
        let ids_antes: Vec<u64> = self.show.cues.iter().map(|c| c.id).collect();
        let espera = self.show.cues.get(self.en_espera).map(|c| c.id);
        let viejo = std::mem::replace(&mut self.show, nuevo);
        self.reindexar(&ids_antes, espera);
        let cambiados: Vec<usize> = (0..self.show.cues.len())
            .filter(|&i| {
                let c = &self.show.cues[i];
                viejo.cues.iter().find(|v| v.id == c.id).is_none_or(|v| {
                    v.archivo != c.archivo || v.tipo != c.tipo || v.inicio_ms != c.inicio_ms || v.fin_ms != c.fin_ms || v.bucle != c.bucle || v.encuadre != c.encuadre
                })
            })
            .collect();
        for i in cambiados {
            self.invalidar_cue(i);
        }
        if viejo.superficie != self.show.superficie {
            self.enviar_esquinas();
        }
        if viejo.ajustes != self.show.ajustes {
            self.enviar_ajustes(0);
        }
        self.sucio = true;
    }

    pub fn deshacer(&mut self) -> Result<(), Texto> {
        self.solo_en_edicion()?;
        let Some(anterior) = self.historial.pop() else {
            return Err(t("No hay nada para deshacer.", "There is nothing to undo."));
        };
        self.rehechos.push(self.show.clone());
        self.poner_show(anterior);
        self.ultimo_cambio = None;
        self.registro.anotar("DESHACER", "último cambio deshecho");
        Ok(())
    }

    pub fn rehacer(&mut self) -> Result<(), Texto> {
        self.solo_en_edicion()?;
        let Some(siguiente) = self.rehechos.pop() else {
            return Err(t("No hay nada para rehacer.", "There is nothing to redo."));
        };
        self.historial.push(self.show.clone());
        self.poner_show(siguiente);
        self.ultimo_cambio = None;
        self.registro.anotar("REHACER", "cambio vuelto a hacer");
        Ok(())
    }

    /// Siguiente número libre para un cue nuevo: uno más que el mayor número entero.
    fn numero_libre(&self) -> String {
        let mayor = self
            .show
            .cues
            .iter()
            .filter_map(|c| c.numero.chars().take_while(|ch| ch.is_ascii_digit()).collect::<String>().parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        (mayor + 1).to_string()
    }

    /// Agrega cues con archivos que ya están dentro de la carpeta del show
    /// (rutas relativas con `/`), después del cue `despues_de` (o al final).
    /// Devuelve dónde quedaron.
    pub fn agregar_cues(&mut self, archivos: &[String], despues_de: Option<usize>) -> Result<Vec<usize>, Texto> {
        self.solo_en_edicion()?;
        let carpeta = self.carpeta();
        let mut nuevos = Vec::new();
        for a in archivos {
            let ruta = rutas::resolver(&carpeta, a).map_err(|e| t(format!("{a}: {e}"), format!("{a}: invalid path ({e})")))?;
            let tipo = otaconvert_show::tipo_por_extension(&ruta)
                .ok_or_else(|| t(format!("{a}: no es un audio, vídeo o imagen que se pueda usar."), format!("{a}: not a usable audio, video or image file.")))?;
            if !ruta.is_file() {
                return Err(t(format!("No se encontró el archivo {a}."), format!("File not found: {a}.")));
            }
            let nombre = std::path::Path::new(a).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            nuevos.push((a.clone(), tipo, nombre));
        }
        if nuevos.is_empty() {
            return Ok(Vec::new());
        }
        let mut prueba = self.show.clone();
        let mut lugar = despues_de.map(|d| (d + 1).min(prueba.cues.len())).unwrap_or(prueba.cues.len());
        let primero = lugar;
        let mut id = self.nuevo_id();
        let mut numero = self.numero_libre().parse::<u64>().unwrap_or(1);
        for (archivo, tipo, nombre) in &nuevos {
            prueba.cues.insert(lugar, Cue { id, numero: numero.to_string(), nombre: nombre.chars().take(MAX_NOMBRE).collect(), tipo: *tipo, archivo: archivo.clone(), ..Default::default() });
            lugar += 1;
            id += 1;
            numero += 1;
        }
        otaconvert_show::validar(&prueba).map_err(|e| t(format!("No se pudo agregar: {e}"), format!("Could not add: {e}")))?;
        let ids_antes: Vec<u64> = self.show.cues.iter().map(|c| c.id).collect();
        let espera = self.show.cues.get(self.en_espera).map(|c| c.id);
        self.antes_de_cambiar(None);
        self.show = prueba;
        self.reindexar(&ids_antes, espera);
        // Si no había nada en espera (lista vacía o al final), queda en espera el primero nuevo.
        if espera.is_none() {
            self.en_espera = primero;
        }
        self.registro.anotar("AGREGAR", &format!("{} cue(s) desde el {}", nuevos.len(), primero + 1));
        Ok((primero..lugar).collect())
    }

    /// Quita un cue de la lista (el archivo no se borra). No se puede si está sonando.
    pub fn quitar_cue(&mut self, i: usize) -> Result<(), Texto> {
        self.solo_en_edicion()?;
        if i >= self.show.cues.len() {
            return Err(t("Ese cue no existe.", "That cue does not exist."));
        }
        if self.sonando.iter().any(|s| s.cue == i) {
            return Err(t("Ese cue está sonando: páralo antes de quitarlo.", "That cue is playing: stop it before removing it."));
        }
        let d = self.describir(i);
        let ids_antes: Vec<u64> = self.show.cues.iter().map(|c| c.id).collect();
        let espera = self.show.cues.get(self.en_espera).map(|c| c.id).filter(|_| self.en_espera != i);
        self.antes_de_cambiar(None);
        self.show.cues.remove(i);
        self.reindexar(&ids_antes, espera);
        if espera.is_none() {
            // Se quitó el que estaba en espera: queda en espera el que vino a su lugar.
            self.en_espera = i.min(self.show.cues.len());
        }
        self.registro.anotar("QUITAR", &d);
        Ok(())
    }

    /// Mueve el cue `desde` al lugar `hasta` (reordenar). Lo que suena sigue sonando.
    pub fn mover_cue(&mut self, desde: usize, hasta: usize) -> Result<(), Texto> {
        self.solo_en_edicion()?;
        let n = self.show.cues.len();
        if desde >= n || hasta >= n {
            return Err(t("Ese cue no existe.", "That cue does not exist."));
        }
        if desde == hasta {
            return Ok(());
        }
        let ids_antes: Vec<u64> = self.show.cues.iter().map(|c| c.id).collect();
        let espera = self.show.cues.get(self.en_espera).map(|c| c.id);
        self.antes_de_cambiar(None);
        let c = self.show.cues.remove(desde);
        self.show.cues.insert(hasta, c);
        self.reindexar(&ids_antes, espera);
        let d = self.describir(hasta);
        self.registro.anotar("MOVER", &format!("{d} al lugar {}", hasta + 1));
        Ok(())
    }

    pub fn carpeta(&self) -> PathBuf {
        self.ruta.parent().unwrap_or(Path::new(".")).to_path_buf()
    }

    /// Conecta el sonido y le pasa las rampas y el volumen maestro del show.
    pub fn conectar_sonido(&mut self, s: Box<dyn Sonido>) {
        let fs = s.frecuencia();
        self.sonido = Some(s);
        self.enviar_ajustes(0);
        self.registro.anotar("SONIDO", &format!("salida conectada a {fs} Hz"));
        self.preparados.clear();
        self.errores.clear();
        self.mudos.clear();
    }

    /// Manda al mezclador las rampas y el volumen maestro del show (sin cortar
    /// lo que suena). `cuadros_maestro`: cuánto tarda el cambio de volumen.
    fn enviar_ajustes(&mut self, cuadros_maestro: u32) {
        let Some(s) = self.sonido.as_mut() else { return };
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
            let _ = s.enviar(d, Comando::VolumenMaestro { volumen: maestro, cuadros: cuadros_maestro });
        }
    }

    /// Edición → Ajustes del show (etapa 5B): rampas cortas, protección contra
    /// doble GO, duración de Fundir todo y volumen maestro. Valida todo antes
    /// de cambiar nada; se aplica en vivo y se puede deshacer.
    pub fn cambiar_ajustes(&mut self, cambios: &serde_json::Value) -> Result<(), Texto> {
        self.solo_en_edicion()?;
        let obj = cambios.as_object().ok_or_else(|| t("Ajustes no válidos.", "Invalid settings."))?;
        let mut a = self.show.ajustes.clone();
        for (k, v) in obj {
            let n = v.as_f64().filter(|x| x.is_finite());
            let rango = |min: f64, max: f64, es: &str, en: &str| {
                n.filter(|x| (min..=max).contains(x)).ok_or_else(|| t(format!("{es}: debe ir de {min} a {max}."), format!("{en}: must be between {min} and {max}.")))
            };
            match k.as_str() {
                "rampa_inicio" | "rampa_reanudar" | "rampa_pausa" | "rampa_parada" | "rampa_salto" | "rampa_volumen" | "rampa_fin" => {
                    let ms = rango(0.0, 1000.0, "Rampa (ms)", "Ramp (ms)")? as f32;
                    let r = &mut a.rampas_ms;
                    match k.as_str() {
                        "rampa_inicio" => r.inicio = ms,
                        "rampa_reanudar" => r.reanudar = ms,
                        "rampa_pausa" => r.pausa = ms,
                        "rampa_parada" => r.parada = ms,
                        "rampa_salto" => r.salto = ms,
                        "rampa_volumen" => r.volumen = ms,
                        _ => r.fin = ms,
                    }
                }
                "doble_go_ms" => a.tiempo_minimo_entre_go_ms = rango(0.0, 5000.0, "Protección contra doble GO (ms)", "Double-GO protection (ms)")? as u32,
                "fundir_todo_ms" => a.fundir_todo_ms = rango(0.0, 60_000.0, "Fundir todo (ms)", "Fade all (ms)")? as u32,
                "volumen_maestro_db" => a.volumen_maestro_db = rango(-60.0, 12.0, "Volumen maestro (dB)", "Master volume (dB)")? as f32,
                _ => return Err(t(format!("Ajuste desconocido: {k}"), format!("Unknown setting: {k}"))),
            }
        }
        if a == self.show.ajustes {
            return Ok(());
        }
        let mut prueba = self.show.clone();
        prueba.ajustes = a.clone();
        otaconvert_show::validar(&prueba).map_err(|e| t(format!("Ajuste no válido: {e}"), format!("Invalid setting: {e}")))?;
        self.antes_de_cambiar(Some("ajustes"));
        self.show.ajustes = a;
        let fs = self.sonido.as_ref().map(|s| s.frecuencia()).unwrap_or(48_000);
        self.enviar_ajustes(otaconvert_audio::ms(fs, 50.0));
        let r = &self.show.ajustes.rampas_ms;
        self.registro.anotar(
            "AJUSTES",
            &format!(
                "rampas {}/{}/{}/{}/{}/{}/{} ms, doble GO {} ms, fundir todo {} ms, maestro {:.1} dB",
                r.inicio, r.reanudar, r.pausa, r.parada, r.salto, r.volumen, r.fin,
                self.show.ajustes.tiempo_minimo_entre_go_ms, self.show.ajustes.fundir_todo_ms, self.show.ajustes.volumen_maestro_db
            ),
        );
        Ok(())
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
            let den = self.describir_en(i);
            // El mismo vídeo o imagen que está al aire (p. ej. elegido otra vez en
            // la Vista Estudio): no hace falta precargarlo, vuelve a empezar.
            // Hallado en la función simulada: antes el GO esperaba una precarga
            // que nunca llegaba ("todavía se está preparando") hasta Parar todo.
            let al_aire = self.sonando.iter().rev().find(|s| s.visual && s.cue == i).map(|s| s.duracion_visual);
            if let (true, Some(duracion)) = (self.proyector.is_some(), al_aire) {
                // Vuelve a su inicio: "salir desde aquí" no se aplica a lo que ya se ve.
                if self.desde_elegido.is_some_and(|(k, _)| k == i) {
                    self.desde_elegido = None;
                }
                self.mostrar_visual(i, duracion);
                self.ultimo_disparado = Some(i);
                self.ultimo_go = Some(Instant::now());
                self.en_espera = i + 1;
                self.registro.anotar("GO", &format!("{d} (otra vez, desde su inicio)"));
                return true;
            }
            match (self.proyector.is_some(), self.visuales.get(&i).cloned()) {
                (false, _) => {
                    // Sin proyección (p. ej. falta libmpv): se avanza y se avisa.
                    self.aviso = Some(t(
                        format!("{d} no se vio: no hay proyección abierta."),
                        format!("{den} was not shown: no projection is open."),
                    ));
                    self.registro.anotar("ERROR", &format!("{d}: no hay proyección"));
                }
                (true, Some(Visual::Listo { .. })) if !self.sonido_del_video_listo(i) => {
                    // La imagen está, pero el sonido del vídeo todavía se prepara.
                    self.aviso = Some(t(
                        format!("{d} todavía se está preparando; vuelve a dar GO en un momento."),
                        format!("{den} is still being prepared; press GO again in a moment."),
                    ));
                    self.registro.anotar("IGNORADO", &format!("{d} sin preparar (sonido)"));
                    return false;
                }
                (true, Some(Visual::Listo { duracion })) => {
                    self.mostrar_visual(i, duracion);
                    if let Some(e) = self.errores.get(&i).cloned().filter(|_| self.show.cues[i].tipo == TipoCue::Video) {
                        // El vídeo se ve igual; solo su sonido no se puede reproducir.
                        self.aviso = Some(t(
                            format!("{d}: el vídeo se ve, pero su sonido no se puede reproducir: {}", e.es),
                            format!("{den}: the video is shown, but its sound can't be played: {}", e.en),
                        ));
                    }
                }
                (true, Some(Visual::Error(e))) => {
                    self.aviso = Some(t(format!("{d} no se vio: {}", e.es), format!("{den} was not shown: {}", e.en)));
                    self.registro.anotar("ERROR", &format!("{d}: {}", e.es));
                }
                (true, _) => {
                    self.aviso = Some(t(
                        format!("{d} todavía se está preparando; vuelve a dar GO en un momento."),
                        format!("{den} is still being prepared; press GO again in a moment."),
                    ));
                    self.registro.anotar("IGNORADO", &format!("{d} sin preparar"));
                    return false;
                }
            }
        }
        self.ultimo_go = Some(Instant::now());
        self.registro.anotar("GO", &d);
        self.ultimo_disparado = Some(i);
        self.en_espera = i + 1;
        true
    }

    /// Conecta la proyección (vídeo e imagen).
    pub fn conectar_proyector(&mut self, p: Box<dyn Proyector>) {
        self.registro.anotar("PROYECCION", "proyección conectada");
        self.proyector = Some(p);
        self.visuales.clear();
        self.enviar_esquinas();
    }

    fn al_proyector(&mut self, o: OrdenProyector) {
        if let Some(p) = self.proyector.as_mut() {
            p.enviar(o);
        }
    }

    /// ¿El sonido de un vídeo está listo? (o el vídeo es mudo, o su sonido
    /// falló, o no hay salida de sonido: en esos casos no se espera).
    fn sonido_del_video_listo(&self, i: usize) -> bool {
        self.show.cues[i].tipo != TipoCue::Video
            || self.sonido.is_none()
            || self.preparados.contains_key(&i)
            || self.mudos.contains(&i)
            || self.errores.contains_key(&i)
    }

    /// Resultado de preparar el sonido del cue `i`: un vídeo sin sonido no es
    /// un error (queda "mudo"); lo demás va a `preparado`.
    pub fn resultado_preparar(&mut self, i: usize, r: Result<otaconvert_audio::preparar::Preparado, otaconvert_audio::preparar::ErrorPreparar>) {
        if std::mem::take(&mut self.descartar_preparacion) {
            return;
        }
        match r {
            Err(otaconvert_audio::preparar::ErrorPreparar::SinAudio) if self.show.cues.get(i).is_some_and(|c| c.tipo == TipoCue::Video) => {
                if self.preparando == Some(i) {
                    self.preparando = None;
                }
                self.mudos.insert(i);
            }
            r => self.preparado(i, r.map_err(Texto::from)),
        }
    }

    /// GO de un vídeo o imagen ya precargado: sube encima y el anterior se retira.
    /// Si es un vídeo con sonido, el sonido arranca a la vez por el mezclador.
    fn mostrar_visual(&mut self, i: usize, duracion: Option<f64>) {
        self.al_proyector(OrdenProyector::Mostrar { id: self.show.cues[i].id });
        // Una sola superficie (Proyector) por ahora: lo que se veía deja de verse,
        // y si tenía sonido, su sonido se para (con la rampa de parada).
        let viejas: Vec<(u32, Destino)> = self.sonando.iter().filter(|s| s.visual && s.voz).map(|s| (s.id, s.destino)).collect();
        for (id, destino) in viejas {
            if let Some(son) = self.sonido.as_mut() {
                let _ = son.enviar(destino, Comando::Parar { id, fundido: 0 });
            }
        }
        self.sonando.retain(|s| !s.visual);
        // Ya no está precargado: está a la vista (si se vuelve a él, se precarga de nuevo).
        self.visuales.remove(&i);
        // Lo que dura una vuelta del vídeo, con su inicio y su fin.
        let c = self.show.cues[i].clone();
        let largo = match duracion {
            Some(d) if c.tipo == TipoCue::Video => {
                let fin = c.fin_ms.map(|f| (f as f64 / 1000.0).min(d)).unwrap_or(d);
                Duration::from_secs_f64((fin - c.inicio_ms as f64 / 1000.0).max(0.0))
            }
            _ => Duration::ZERO,
        };
        // El sonido del vídeo, a la vez que la imagen. En bucle, vuelve a
        // empezar donde vuelve la imagen (fin del vídeo según mpv).
        let fin_video = match (c.tipo, duracion) {
            (TipoCue::Video, Some(d)) => Some(c.fin_ms.map(|f| (f as f64 / 1000.0).min(d)).unwrap_or(d)),
            _ => None,
        };
        // "Salir desde aquí": la imagen ya está precargada en ese punto; el
        // sonido sale desde el mismo segundo. Se usa una sola vez.
        let desde = self.desde_elegido.filter(|(k, _)| *k == i).map(|(_, s)| s);
        if desde.is_some() {
            self.desde_elegido = None;
        }
        let voz = match self.preparados.get(&i).cloned() {
            Some(p) if c.tipo == TipoCue::Video => self.iniciar_voz(i, p, fin_video, desde),
            _ => None,
        };
        // El reloj de "lo que falta" cuenta como si hubiera empezado antes.
        let adelanto = desde.map(|s| Duration::from_secs_f64((s - c.inicio_ms as f64 / 1000.0).max(0.0))).unwrap_or(Duration::ZERO);
        let (id, destino) = match voz {
            Some((id, destino, _)) => (id, destino),
            None => {
                let id = self.siguiente_id;
                self.siguiente_id = self.siguiente_id.wrapping_add(1).max(1);
                (id, Destino::Principal)
            }
        };
        self.sonando.push(Sonando {
            id,
            cue: i,
            destino,
            inicio: Instant::now().checked_sub(adelanto).unwrap_or_else(Instant::now),
            // Termina con el aviso Fin del proyector (vídeo sin bucle) o al parar.
            duracion: None,
            largo,
            fundiendo: false,
            visual: true,
            voz: voz.is_some(),
            duracion_visual: duracion,
        });
    }

    /// Lo que cuenta la proyección. Se llama seguido (hilo de refresco).
    pub fn atender_proyector(&mut self) {
        let avisos = match self.proyector.as_mut() {
            Some(p) => p.recoger(),
            None => return,
        };
        for a in avisos {
            match a {
                AvisoProyector::Listo { id, duracion } => {
                    let Some(i) = self.indice_de_id(id) else { continue };
                    if self.remostrar == Some(i) {
                        // Tras reiniciarse la proyección: vuelve lo que se veía.
                        self.remostrar = None;
                        self.al_proyector(OrdenProyector::Mostrar { id });
                        if let Some(s) = self.sonando.iter_mut().find(|s| s.visual && s.cue == i) {
                            s.largo = duracion.map(Duration::from_secs_f64).unwrap_or(s.largo);
                        }
                    } else if self.visuales.contains_key(&i) {
                        self.visuales.insert(i, Visual::Listo { duracion });
                    }
                }
                AvisoProyector::Error { id, motivo } => {
                    let Some(i) = self.indice_de_id(id) else { continue };
                    if i < self.show.cues.len() {
                        let d = self.describir(i);
                        self.registro.anotar("ERROR", &format!("{d}: {}", motivo.es));
                    }
                    if self.remostrar == Some(i) {
                        self.remostrar = None;
                    }
                    if self.visuales.contains_key(&i) {
                        self.visuales.insert(i, Visual::Error(motivo));
                    }
                }
                AvisoProyector::Mostrando { .. } => {}
                AvisoProyector::Fin { id } => {
                    let Some(i) = self.indice_de_id(id) else { continue };
                    self.sonando.retain(|s| !(s.visual && s.cue == i));
                }
                AvisoProyector::Reiniciado { ms } => {
                    self.registro.anotar("PROYECCION", &format!("la proyección se cerró sola y se volvió a abrir en {ms} ms"));
                    self.aviso = Some(t(
                        "La proyección se cerró sola y se volvió a abrir. El sonido no se cortó.",
                        "The projection closed by itself and was reopened. The sound was not interrupted.",
                    ));
                    // La proyección nueva no sabe las esquinas ni si estaba el patrón.
                    self.enviar_esquinas();
                    if self.patron {
                        self.al_proyector(OrdenProyector::Patron(true));
                    }
                    // Lo precargado se perdió: se vuelve a pedir.
                    self.visuales.clear();
                    // Lo que se veía vuelve desde donde iba.
                    if let Some(s) = self.sonando.iter().find(|s| s.visual) {
                        let (i, pasado) = (s.cue, s.inicio.elapsed().as_secs_f64());
                        let largo = s.largo.as_secs_f64();
                        let c = &self.show.cues[i];
                        let base = c.inicio_ms as f64 / 1000.0;
                        let desde = if c.tipo == TipoCue::Imagen {
                            0.0
                        } else if c.bucle && largo > 0.0 {
                            base + pasado % largo
                        } else {
                            base + pasado
                        };
                        if let Ok(ruta) = rutas::resolver(&self.carpeta(), &c.archivo) {
                            let o = OrdenProyector::Precargar {
                                id: c.id,
                                ruta,
                                bucle: c.bucle,
                                imagen: c.tipo == TipoCue::Imagen,
                                inicio_s: base,
                                fin_s: c.fin_ms.map(|f| f as f64 / 1000.0),
                                desde_s: (c.tipo == TipoCue::Video).then_some(desde),
                                encuadre: c.encuadre,
                            };
                            self.remostrar = Some(i);
                            self.al_proyector(o);
                        }
                    }
                }
                AvisoProyector::Aviso { motivo } => {
                    self.registro.anotar("AVISO", &format!("proyección: {}", motivo.es));
                    self.aviso = Some(motivo);
                }
                AvisoProyector::Caido { motivo } => {
                    self.registro.anotar("ERROR", &format!("proyección: {}", motivo.es));
                    self.aviso = Some(t(
                        format!("¡Atención! La proyección no funciona: {}", motivo.es),
                        format!("Warning! The projection is not working: {}", motivo.en),
                    ));
                    self.visuales.clear();
                    self.sonando.retain(|s| !s.visual);
                }
            }
        }
    }

    fn disparar(&mut self, i: usize, pista: Arc<Pista>) {
        let c = self.show.cues[i].clone();
        if let Some((id, destino, largo)) = self.iniciar_voz(i, pista, None, None) {
            self.sonando.push(Sonando {
                id,
                cue: i,
                destino,
                inicio: Instant::now(),
                duracion: (!c.bucle).then_some(largo),
                largo,
                fundiendo: false,
                visual: false,
                voz: true,
                duracion_visual: None,
            });
        }
    }

    /// Manda al mezclador el sonido del cue `i` (un audio o el sonido de un
    /// vídeo) con su volumen, salida, inicio, fin, fundidos y bucle.
    /// `fin_s`: fin forzado en segundos (el del vídeo, para que en bucle el
    /// sonido vuelva a empezar junto con la imagen). Devuelve (voz, salida, largo).
    fn iniciar_voz(&mut self, i: usize, pista: Arc<Pista>, fin_s: Option<f64>, desde_s: Option<f64>) -> Option<(u32, Destino, Duration)> {
        let c = self.show.cues[i].clone();
        let destino = self.destino_de(i);
        let id = self.siguiente_id;
        self.siguiente_id = self.siguiente_id.wrapping_add(1).max(1);
        let total = pista.cuadros();
        let inicio = self.cuadros(c.inicio_ms).min(total.saturating_sub(1));
        let pista_fs = pista.frecuencia() as f64;
        let hasta = match fin_s {
            Some(f) => Some(((f * pista.frecuencia() as f64) as usize).min(total)),
            None => c.fin_ms.map(|f| self.cuadros(f).min(total)),
        }
        .filter(|&h| h > inicio);
        let fin = hasta.unwrap_or(total);
        let fs = pista.frecuencia() as f64;
        let comando = Comando::IniciarCue {
            id,
            pista,
            volumen: db_a_lineal(c.volumen_db),
            inicio,
            // Salir desde un punto (dentro del cue): el bucle vuelve igual a `inicio`.
            desde: desde_s.map(|s| ((s * pista_fs) as usize).clamp(inicio, fin.saturating_sub(1))).unwrap_or(inicio),
            hasta,
            bucle: c.bucle,
            fundido_entrada: self.cuadros(c.fundido_entrada_ms as u64) as u32,
            fundido_salida: self.cuadros(c.fundido_salida_ms as u64) as u32,
        };
        let enviado = self.sonido.as_mut().map(|s| s.enviar(destino, comando));
        match enviado {
            Some(Ok(())) => Some((id, destino, Duration::from_secs_f64(fin.saturating_sub(inicio) as f64 / fs))),
            Some(Err(e)) => {
                self.aviso = Some(Texto::nuevo(format!("No se pudo disparar: {e}"), format!("Could not fire the cue: {e}")));
                None
            }
            None => None,
        }
    }

    /// Cambia entre Edición y Función (Tab). Nunca toca lo que suena.
    pub fn cambiar_modo(&mut self, modo: Modo) {
        if self.modo != modo {
            self.modo = modo;
            self.registro.anotar("MODO", if modo == Modo::Funcion { "Función" } else { "Edición" });
            // El patrón de prueba nunca queda a la vista en Función.
            if modo == Modo::Funcion && self.patron {
                let _ = self.mostrar_patron(false);
            }
        }
    }

    /// En Edición, clic en un cue: queda en espera.
    /// "Salir desde aquí" (Vista Estudio): el vídeo en espera saldrá con el
    /// próximo GO desde el segundo `s` del archivo (imagen y sonido juntos).
    /// `None` = desde su inicio. Se vuelve a precargar en ese punto; hasta que
    /// esté listo, el GO avisa "todavía se está preparando".
    pub fn elegir_desde(&mut self, i: usize, s: Option<f64>) -> Result<(), Texto> {
        if i != self.en_espera {
            return Err(t("Solo se elige desde dónde sale el cue en espera.", "You can only choose where the standby cue starts."));
        }
        let Some(c) = self.show.cues.get(i).cloned() else {
            return Err(t("Ese cue no existe.", "That cue does not exist."));
        };
        if c.tipo != TipoCue::Video {
            return Err(t("Solo los vídeos pueden salir desde un punto.", "Only videos can start from a point."));
        }
        let inicio = c.inicio_ms as f64 / 1000.0;
        let duracion = match self.visuales.get(&i) {
            Some(Visual::Listo { duracion }) => *duracion,
            _ => None,
        };
        let fin = c.fin_ms.map(|f| f as f64 / 1000.0).or(duracion);
        if let Some(x) = s {
            if !x.is_finite() || x < inicio - 0.05 || fin.is_some_and(|f| x >= f) {
                return Err(t("Ese punto está fuera del vídeo.", "That point is outside the video."));
            }
        }
        // Muy cerca del inicio = desde el inicio.
        let nuevo = s.filter(|x| *x - inicio > 0.05).map(|x| (i, x));
        if nuevo == self.desde_elegido {
            return Ok(());
        }
        self.desde_elegido = nuevo;
        let d = self.describir(i);
        match nuevo {
            Some((_, x)) => self.registro.anotar("DESDE", &format!("{d} saldrá desde {x:.1} s")),
            None => self.registro.anotar("DESDE", &format!("{d} saldrá desde su inicio")),
        }
        // Se vuelve a precargar en el punto nuevo.
        if self.visuales.remove(&i).is_some() {
            self.al_proyector(OrdenProyector::Quitar { id: self.show.cues[i].id });
        }
        self.precargar_visuales();
        Ok(())
    }

    /// Si el cue en espera cambió, el "salir desde aquí" del anterior se
    /// olvida, y ese vídeo se vuelve a precargar desde su inicio.
    fn soltar_desde_si_cambio(&mut self) {
        if let Some((k, _)) = self.desde_elegido {
            if k != self.en_espera {
                self.desde_elegido = None;
                if self.visuales.remove(&k).is_some() {
                    if let Some(id) = self.show.cues.get(k).map(|c| c.id) {
                        self.al_proyector(OrdenProyector::Quitar { id });
                    }
                }
            }
        }
    }

    /// Deja el cue `i` en espera (clic en Edición; en Función, desde la Vista
    /// Estudio o el buscador). Nunca dispara nada: el GO lo da el operador.
    /// En Función se anota en el registro (ELEGIR), para saber qué se eligió a mano.
    pub fn ir_a(&mut self, i: usize) {
        if i < self.show.cues.len() && i != self.en_espera {
            self.en_espera = i;
            if self.modo == Modo::Funcion {
                let d = self.describir(i);
                self.registro.anotar("ELEGIR", &d);
            }
            self.soltar_desde_si_cambio();
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
                // Numeración libre, como en el libreto ("1", "1.5", "2A").
                "numero" => {
                    let s = v.as_str().ok_or_else(|| t("El número debe ser texto.", "The number must be text."))?.trim();
                    if s.is_empty() || s.chars().count() > MAX_NUMERO || s.chars().any(char::is_control) {
                        return Err(t(
                            format!("El número va de 1 a {MAX_NUMERO} letras o cifras (por ejemplo 1, 1.5 o 2A)."),
                            format!("The number has 1 to {MAX_NUMERO} letters or digits (for example 1, 1.5 or 2A)."),
                        ));
                    }
                    c.numero = s.to_string();
                }
                // Inicio y fin dentro del archivo, en milisegundos (fin null = hasta el final).
                "inicio_ms" => {
                    c.inicio_ms = v
                        .as_u64()
                        .filter(|&x| x <= 24 * 3600 * 1000)
                        .ok_or_else(|| t("El inicio debe ser un tiempo válido.", "The start must be a valid time."))?;
                }
                "fin_ms" => {
                    c.fin_ms = if v.is_null() {
                        None
                    } else {
                        Some(v.as_u64().filter(|&x| x <= 24 * 3600 * 1000).ok_or_else(|| t("El fin debe ser un tiempo válido.", "The end must be a valid time."))?)
                    };
                }
                // Encuadre (etapa 4C): solo vídeos e imágenes.
                "encuadre_modo" | "zoom" | "mover_x" | "mover_y" | "recorte_arriba" | "recorte_abajo" | "recorte_izquierda"
                | "recorte_derecha" => {
                    if c.tipo == TipoCue::Audio {
                        return Err(t("El encuadre es solo para vídeos e imágenes.", "Framing is only for videos and images."));
                    }
                    let numero = || {
                        v.as_f64()
                            .filter(|x| x.is_finite())
                            .map(|x| x as f32)
                            .ok_or_else(|| t("El encuadre necesita un número.", "Framing needs a number."))
                    };
                    let e = &mut c.encuadre;
                    match k.as_str() {
                        "encuadre_modo" => {
                            e.modo = match v.as_str() {
                                Some("ajustar") => otaconvert_show::ModoEncuadre::Ajustar,
                                Some("llenar") => otaconvert_show::ModoEncuadre::Llenar,
                                Some("estirar") => otaconvert_show::ModoEncuadre::Estirar,
                                _ => return Err(t("El encuadre es Ajustar, Llenar o Estirar.", "Framing is Fit, Fill or Stretch.")),
                            }
                        }
                        "zoom" => e.zoom = numero()?,
                        "mover_x" => e.mover_x = numero()?,
                        "mover_y" => e.mover_y = numero()?,
                        "recorte_arriba" => e.recorte_arriba = numero()?,
                        "recorte_abajo" => e.recorte_abajo = numero()?,
                        "recorte_izquierda" => e.recorte_izquierda = numero()?,
                        _ => e.recorte_derecha = numero()?,
                    }
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
            let (antes, despues) = (&self.show.cues[i], &prueba.cues[i]);
            // Lo que cambia cómo se prepara o se precarga: se vuelve a preparar.
            let rehacer = antes.encuadre != despues.encuadre
                || antes.inicio_ms != despues.inicio_ms
                || antes.fin_ms != despues.fin_ms
                || (antes.bucle != despues.bucle && despues.tipo != TipoCue::Audio);
            self.antes_de_cambiar(None);
            self.show = prueba;
            let d = self.describir(i);
            self.registro.anotar("EDITAR", &d);
            if rehacer {
                self.invalidar_cue(i);
            }
        }
        Ok(())
    }

    /// Manda a la proyección las 4 esquinas del show.
    fn enviar_esquinas(&mut self) {
        if let Some(h) = self.show.superficie.homografia_inversa() {
            self.al_proyector(OrdenProyector::Esquinas(h));
        }
    }

    /// Mueve las 4 esquinas del Proyector (solo en Edición). Se ve enseguida.
    pub fn cambiar_esquinas(&mut self, esquinas: [[f64; 2]; 4]) -> Result<(), Texto> {
        if self.modo == Modo::Funcion {
            return Err(t(
                "En Función las esquinas están bloqueadas. Pasa a Edición con Tab.",
                "Corners are locked in Live mode. Switch to Edit with Tab.",
            ));
        }
        let s = otaconvert_show::Superficie { esquinas };
        if let Some(p) = s.problema() {
            return Err(t(format!("Esas esquinas no sirven: {p}"), "Those corners can't be used: they cross or are squashed.".to_string()));
        }
        if s != self.show.superficie {
            // Arrastrar una esquina manda muchos cambios: cuentan como uno para deshacer.
            self.antes_de_cambiar(Some("esquinas"));
            self.show.superficie = s;
            self.enviar_esquinas();
        }
        Ok(())
    }

    /// Patrón de prueba en el proyector para encajar las esquinas (solo en Edición).
    pub fn mostrar_patron(&mut self, si: bool) -> Result<(), Texto> {
        if si && self.modo == Modo::Funcion {
            return Err(t("En Función no se muestra el patrón de prueba.", "The test pattern is not shown in Live mode."));
        }
        if self.patron != si {
            self.patron = si;
            self.al_proyector(OrdenProyector::Patron(si));
            self.registro.anotar("MAPEO", if si { "patrón de prueba a la vista" } else { "patrón de prueba quitado" });
        }
        Ok(())
    }

    pub fn mover_espera(&mut self, delta: i64) {
        if self.show.cues.is_empty() {
            return;
        }
        let max = self.show.cues.len() as i64;
        self.en_espera = (self.en_espera as i64 + delta).clamp(0, max) as usize;
        self.soltar_desde_si_cambio();
    }

    pub fn parar_todo(&mut self) {
        self.a_todas(|| Comando::PararTodo { fundido: 0 });
        // La proyección a negro al instante (telón), como el sonido.
        self.al_proyector(OrdenProyector::Negro);
        self.remostrar = None;
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
        if self.sonando.iter().any(|s| s.visual) {
            self.al_proyector(OrdenProyector::Fundir { ms: self.show.ajustes.fundir_todo_ms as u64 });
        }
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
        let id = s.id;
        self.fundir_uno(id);
    }

    /// Panel Sonando (etapa 5C): funde solo lo que suena con este `id` (el de
    /// la lista "sonando" del estado), en la duración de Fundir todo. Vale
    /// también en Función: es un control de la función, no cambia el show.
    pub fn fundir_uno(&mut self, id: u32) {
        let Some(s) = self.sonando.iter().find(|s| s.id == id) else { return };
        if s.fundiendo {
            return;
        }
        let (id, destino, cue, visual, voz) = (s.id, s.destino, s.cue, s.visual, s.voz);
        let f = self.cuadros(self.show.ajustes.fundir_todo_ms as u64) as u32;
        if visual {
            self.al_proyector(OrdenProyector::Fundir { ms: self.show.ajustes.fundir_todo_ms as u64 });
        }
        // Un audio, o el sonido de un vídeo: baja junto con la imagen.
        if voz {
            if let Some(son) = self.sonido.as_mut() {
                let _ = son.enviar(destino, Comando::Parar { id, fundido: f });
            }
        }
        let d = self.describir(cue);
        self.registro.anotar("FUNDIR", &format!("solo {d}"));
        let fin = Instant::now() + Duration::from_millis(self.show.ajustes.fundir_todo_ms as u64);
        if let Some(s) = self.sonando.iter_mut().find(|s| s.id == id) {
            s.duracion = Some(fin.saturating_duration_since(s.inicio).min(s.duracion.unwrap_or(Duration::MAX)));
            s.fundiendo = true;
        }
    }

    /// Panel Sonando (etapa 5C): para solo lo que suena con este `id`, con la
    /// rampa de parada (sin clic). Si es la imagen o el vídeo que se ve, el
    /// proyector queda en negro. Vale también en Función.
    pub fn parar_uno(&mut self, id: u32) {
        let Some(k) = self.sonando.iter().position(|s| s.id == id) else { return };
        let s = self.sonando.remove(k);
        if s.voz {
            if let Some(son) = self.sonido.as_mut() {
                let _ = son.enviar(s.destino, Comando::Parar { id: s.id, fundido: 0 });
            }
        }
        // Solo la imagen que se ve ahora (la última que salió) se tapa con negro.
        if s.visual && !self.sonando.iter().any(|o| o.visual) {
            self.al_proyector(OrdenProyector::Negro);
            self.remostrar = None;
        }
        let d = self.describir(s.cue);
        self.registro.anotar("PARAR", &format!("solo {d}"));
    }

    /// Pausa general (P): la primera vez pausa todo, la segunda lo reanuda.
    pub fn pausa(&mut self) {
        if self.pausado {
            self.a_todas(|| Comando::ReanudarTodo);
            self.al_proyector(OrdenProyector::Pausa(false));
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
            self.al_proyector(OrdenProyector::Pausa(true));
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
        self.atender_proyector();
    }

    /// Pide a la proyección que precargue los próximos vídeos e imágenes
    /// (debajo del telón, sin que se vean) y suelta los que ya no hacen falta.
    fn precargar_visuales(&mut self) {
        if self.proyector.is_none() {
            return;
        }
        let quiero: Vec<usize> = (self.en_espera..self.show.cues.len())
            .filter(|&i| self.show.cues[i].tipo != TipoCue::Audio)
            .take(PRECARGAR_VISUALES)
            .collect();
        let sobran: Vec<usize> = self.visuales.keys().copied().filter(|i| !quiero.contains(i)).collect();
        for i in sobran {
            self.visuales.remove(&i);
            if let Some(id) = self.show.cues.get(i).map(|c| c.id) {
                self.al_proyector(OrdenProyector::Quitar { id });
            }
        }
        let carpeta = self.carpeta();
        for i in quiero {
            // Lo que ya se ve no se vuelve a precargar (un nuevo GO lo hace empezar de nuevo).
            if self.visuales.contains_key(&i) || self.sonando.iter().any(|s| s.visual && s.cue == i) || self.remostrar == Some(i) {
                continue;
            }
            let c = &self.show.cues[i];
            match rutas::resolver(&carpeta, &c.archivo) {
                Ok(ruta) if ruta.is_file() => {
                    let o = OrdenProyector::Precargar {
                        id: c.id,
                        ruta,
                        bucle: c.bucle,
                        imagen: c.tipo == TipoCue::Imagen,
                        inicio_s: c.inicio_ms as f64 / 1000.0,
                        fin_s: c.fin_ms.map(|f| f as f64 / 1000.0),
                        // Si se eligió "salir desde aquí" para este cue, queda esperando en ese punto.
                        desde_s: self.desde_elegido.filter(|(k, _)| *k == i).map(|(_, s)| s),
                        encuadre: c.encuadre,
                    };
                    self.visuales.insert(i, Visual::Pidiendo);
                    self.al_proyector(o);
                }
                Ok(_) => {
                    let a = &c.archivo;
                    let e = t(format!("no se encontró el archivo {a}"), format!("file not found: {a}"));
                    self.registro.anotar("ERROR", &format!("{}: {}", self.describir(i), e.es));
                    self.visuales.insert(i, Visual::Error(e));
                }
                Err(e) => {
                    self.visuales.insert(i, Visual::Error(t(e.to_string(), format!("invalid file path: {e}"))));
                }
            }
        }
    }

    /// ¿El cue `i` se repite de punta a punta del archivo? Así se prepara su
    /// sonido (un bucle que empalma sigue empalmando al adaptar la frecuencia).
    fn en_bucle_completo(&self, i: usize) -> bool {
        self.show.cues.get(i).is_some_and(|c| c.bucle && c.inicio_ms == 0 && c.fin_ms.is_none())
    }

    /// Qué audio preparar ahora: el cue en espera y los siguientes. Además
    /// suelta los preparados que ya no hacen falta (o que se prepararon con
    /// otro bucle). De paso pide la precarga de los vídeos e imágenes que vienen.
    pub fn siguiente_a_preparar(&mut self) -> Option<TareaPreparar> {
        self.precargar_visuales();
        let fs = self.sonido.as_ref()?.frecuencia();
        let quiero: Vec<usize> = (self.en_espera..self.show.cues.len())
            .filter(|&i| matches!(self.show.cues[i].tipo, TipoCue::Audio | TipoCue::Video))
            .take(PREPARAR_ADELANTE)
            .collect();
        let sonando: Vec<usize> = self.sonando.iter().map(|s| s.cue).collect();
        // Si se cambió el bucle de un cue que no suena, se vuelve a preparar.
        let cambiaron: Vec<usize> = self
            .preparados
            .keys()
            .copied()
            .filter(|i| !sonando.contains(i) && self.preparado_en_bucle.get(i).copied().unwrap_or(false) != self.en_bucle_completo(*i))
            .collect();
        for i in cambiaron {
            self.preparados.remove(&i);
        }
        self.preparados.retain(|i, _| quiero.contains(i) || sonando.contains(i));
        let preparados = &self.preparados;
        self.preparado_en_bucle.retain(|i, _| preparados.contains_key(i));
        let carpeta = self.carpeta();
        for i in quiero {
            if self.preparados.contains_key(&i) || self.errores.contains_key(&i) || self.mudos.contains(&i) || self.preparando == Some(i) {
                continue;
            }
            match rutas::resolver(&carpeta, &self.show.cues[i].archivo) {
                Ok(r) if r.is_file() => {
                    self.preparando = Some(i);
                    return Some(TareaPreparar { cue: i, ruta: r, frecuencia: fs, bucle: self.en_bucle_completo(i) });
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
        // Se preparó para un orden de cues que ya cambió: se descarta (se vuelve a pedir).
        if std::mem::take(&mut self.descartar_preparacion) {
            return;
        }
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
                self.preparado_en_bucle.insert(i, p.bucle);
            }
            Err(e) => {
                let e: Texto = e.into();
                let d = self.describir(i);
                self.registro.anotar("ERROR", &format!("{d}: {}", e.es));
                self.errores.insert(i, e);
            }
        }
    }

    /// Archivo de un cue de vídeo o imagen (resuelto dentro de la carpeta del
    /// show). Para la vista previa de la ventana; nada más se entrega así.
    pub fn medio_visual(&self, i: usize) -> Option<PathBuf> {
        let c = self.show.cues.get(i)?;
        if c.tipo == TipoCue::Audio {
            return None;
        }
        rutas::resolver(&self.carpeta(), &c.archivo).ok().filter(|r| r.is_file())
    }

    /// Dónde va la foto chica de un cue de vídeo o imagen: en la carpeta del
    /// show, con un nombre que cambia si cambia el archivo o su punto de inicio
    /// (así una foto vieja nunca se muestra para un archivo nuevo).
    pub fn ruta_miniatura(&self, i: usize) -> Option<PathBuf> {
        let c = self.show.cues.get(i)?;
        let archivo = self.medio_visual(i)?;
        let meta = std::fs::metadata(&archivo).ok()?;
        let modificado = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis()).unwrap_or(0);
        // FNV-1a de 64 bits: estable entre versiones del programa.
        let mut h: u64 = 0xcbf29ce484222325;
        for b in format!("{}|{}|{}|{}", c.archivo, meta.len(), modificado, c.inicio_ms).bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Some(self.carpeta().join(".otaconvert").join("miniaturas").join(format!("{h:016x}.jpg")))
    }

    /// La foto del cue, si ya está hecha.
    pub fn miniatura_lista(&self, i: usize) -> Option<PathBuf> {
        self.ruta_miniatura(i).filter(|p| p.is_file())
    }

    /// Fotos por revisar: las de los cues de vídeo o imagen que todavía no se
    /// marcaron como hechas ni fallidas. Las que ya existen en el disco se
    /// marcan hechas aquí mismo; devuelve las que hay que hacer:
    /// (cue, archivo, destino, inicio en segundos).
    pub fn miniaturas_pendientes(&mut self) -> Vec<(usize, PathBuf, PathBuf, f64)> {
        let mut faltan = Vec::new();
        for i in 0..self.show.cues.len() {
            if self.miniaturas_hechas.contains_key(&i) || self.miniaturas_fallidas.contains(&i) {
                continue;
            }
            let (Some(destino), Some(archivo)) = (self.ruta_miniatura(i), self.medio_visual(i)) else { continue };
            if destino.is_file() {
                self.miniatura_hecha(i, &destino);
                continue;
            }
            let inicio = if self.show.cues[i].tipo == TipoCue::Video { self.show.cues[i].inicio_ms as f64 / 1000.0 } else { 0.0 };
            faltan.push((i, archivo, destino, inicio));
        }
        faltan
    }

    pub fn miniatura_hecha(&mut self, i: usize, ruta: &Path) {
        if let Some(v) = ruta.file_stem() {
            self.miniaturas_hechas.insert(i, v.to_string_lossy().into_owned());
        }
    }

    pub fn miniatura_fallida(&mut self, i: usize) {
        self.miniaturas_fallidas.insert(i);
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
                let visual = self.visuales.get(&i);
                let al_aire = self.sonando.iter().any(|s| s.visual && s.cue == i);
                let preparacion = if c.tipo != TipoCue::Audio && al_aire && self.proyector.is_some() {
                    // Lo que se ve está listo: otro GO lo hace empezar de nuevo.
                    "listo"
                } else if c.tipo != TipoCue::Audio {
                    match visual {
                        Some(Visual::Pidiendo) => "preparando",
                        // Un vídeo está listo cuando también lo está su sonido.
                        Some(Visual::Listo { .. }) if !self.sonido_del_video_listo(i) => "preparando",
                        Some(Visual::Listo { .. }) => "listo",
                        Some(Visual::Error(_)) => "error",
                        None => "",
                    }
                } else if self.sonido.is_none() {
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
                // Vídeo: manda lo que dice la proyección al precargarlo (ya con
                // inicio y fin); el sonido del vídeo puede durar un poco distinto.
                let del_video = match visual {
                    Some(Visual::Listo { duracion: Some(d) }) if c.tipo == TipoCue::Video => {
                        let ini = c.inicio_ms as f64 / 1000.0;
                        let fin = c.fin_ms.map(|f| (f as f64 / 1000.0).min(*d)).unwrap_or(*d);
                        Some((fin - ini).max(0.0))
                    }
                    _ => None,
                };
                let duracion_s = if c.tipo == TipoCue::Video { del_video.or(duracion_s) } else { duracion_s };
                let largo_s = match visual {
                    Some(Visual::Listo { duracion: Some(d) }) if c.tipo == TipoCue::Video => Some(*d),
                    _ => self.preparados.get(&i).map(|p| p.cuadros() as f64 / p.frecuencia() as f64),
                };
                let error = match visual {
                    Some(Visual::Error(e)) => Some(e.en_idioma(self.idioma)),
                    _ => self.errores.get(&i).map(|e| e.en_idioma(self.idioma)),
                };
                json!({
                    "numero": if c.numero.is_empty() { (i + 1).to_string() } else { c.numero.clone() },
                    "nombre": c.nombre,
                    "tipo": match c.tipo { TipoCue::Audio => "audio", TipoCue::Video => "video", TipoCue::Imagen => "imagen" },
                    "archivo": c.archivo,
                    "nota": c.nota,
                    "bucle": c.bucle,
                    "salida": c.salida,
                    "volumen_db": c.volumen_db,
                    "inicio_s": c.inicio_ms as f64 / 1000.0,
                    "fin_s": c.fin_ms.map(|f| f as f64 / 1000.0),
                    // Largo del archivo entero (para limitar inicio y fin en la ventana).
                    "largo_s": largo_s,
                    "fundido_entrada_ms": c.fundido_entrada_ms,
                    "fundido_salida_ms": c.fundido_salida_ms,
                    "duracion_s": duracion_s,
                    "preparacion": preparacion,
                    "error": error,
                    // Vídeo sin sonido (no es un error).
                    "mudo": self.mudos.contains(&i),
                    // Foto chica lista (la ventana la pide por /miniatura?cue=N; la versión cambia con el archivo).
                    "miniatura": self.miniaturas_hechas.get(&i),
                    "encuadre": c.encuadre,
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
                let c = self.show.cues.get(s.cue);
                let imagen = c.is_some_and(|c| c.tipo == TipoCue::Imagen);
                // Vídeo sin bucle: lo que falta según su duración (termina con el aviso Fin).
                let restante = match s.duracion {
                    Some(d) => Some(d.saturating_sub(transcurrido).as_secs_f64()),
                    None if s.visual && !imagen && largo > 0.0 && !c.is_some_and(|c| c.bucle) => Some((largo - pasado).max(0.0)),
                    None => None,
                };
                json!({
                    // Para parar o fundir solo este desde el panel Sonando.
                    "id": s.id,
                    "cue": s.cue,
                    "nombre": c.map(|c| c.nombre.clone()).unwrap_or_default(),
                    "visual": s.visual,
                    "imagen": imagen,
                    "restante_s": restante,
                    // Progreso de la vuelta actual, de 0 a 1 (en bucle vuelve a 0).
                    "progreso": if largo > 0.0 { ((pasado % largo) / largo).min(1.0) } else { 0.0 },
                    // Punto del archivo que se ve ahora (para la copia de «al aire» en la ventana).
                    "pos_s": c.map(|c| c.inicio_ms as f64 / 1000.0).unwrap_or(0.0) + if largo > 0.0 { pasado % largo } else { pasado },
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
            "con_proyeccion": self.proyector.is_some(),
            "auriculares": self.sonido.as_ref().is_some_and(|s| s.tiene_auriculares()),
            "modo": if self.modo == Modo::Funcion { "funcion" } else { "edicion" },
            // "Salir desde aquí" del cue en espera (segundos del archivo), o null.
            "desde_s": self.desde_elegido.filter(|(k, _)| *k == self.en_espera).map(|(_, s)| s),
            // Las 4 esquinas del Proyector y si se ve el patrón de prueba (etapa 4C).
            "esquinas": self.show.superficie.esquinas,
            "patron": self.patron,
            // Deshacer y rehacer (etapa 5A).
            "puede_deshacer": !self.historial.is_empty(),
            "puede_rehacer": !self.rehechos.is_empty(),
            // Edición → Ajustes del show (etapa 5B).
            "ajustes": self.show.ajustes,
            "equipo": self.equipo,
            "salidas_respondidas": self.salidas_respondidas,
        })
        .to_string()
    }
}
