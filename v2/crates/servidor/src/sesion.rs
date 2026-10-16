//! Un show abierto: qué cue está en espera, qué se disparó, si hay cambios
//! sin guardar. En la etapa 1 el GO avanza la lista y lo anota en el
//! registro; el sonido llega en la etapa 2.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use otaconvert_show::{abrir, guardar, marca_de_tiempo, ErrorShow, Registro, Show, TipoCue, EXTENSION};
use serde_json::json;

pub struct Sesion {
    pub show: Show,
    pub ruta: PathBuf,
    pub en_espera: usize,
    pub ultimo_disparado: Option<usize>,
    /// Hay cambios sin guardar.
    pub sucio: bool,
    pub aviso: Option<String>,
    registro: Registro,
    ultimo_go: Option<Instant>,
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

impl Sesion {
    pub fn abrir(ruta: &Path) -> Result<Self, ErrorShow> {
        let archivo = buscar_archivo(ruta)?;
        let show = abrir(&archivo)?;
        let carpeta = archivo.parent().unwrap_or(Path::new(".")).to_path_buf();
        let mut registro = Registro::abrir(&carpeta);
        registro.anotar("ABRIR", &format!("Show «{}», {} cues", show.nombre, show.cues.len()));
        Ok(Self { show, ruta: archivo, en_espera: 0, ultimo_disparado: None, sucio: false, aviso: None, registro, ultimo_go: None })
    }

    fn describir(&self, i: usize) -> String {
        let c = &self.show.cues[i];
        let n = if c.numero.is_empty() { (i + 1).to_string() } else { c.numero.clone() };
        format!("Cue {n} «{}»", c.nombre)
    }

    /// GO: dispara el cue en espera y deja en espera el siguiente.
    /// Devuelve `false` si se ignoró (lista vacía o doble GO demasiado rápido).
    pub fn go(&mut self) -> bool {
        if self.show.cues.is_empty() || self.en_espera >= self.show.cues.len() {
            self.aviso = Some("No hay ningún cue en espera.".into());
            return false;
        }
        let minimo = Duration::from_millis(self.show.ajustes.tiempo_minimo_entre_go_ms as u64);
        if let Some(t) = self.ultimo_go {
            if t.elapsed() < minimo {
                self.aviso = Some("GO ignorado: llegó demasiado rápido después del anterior (protección contra doble GO).".into());
                let d = self.describir(self.en_espera);
                self.registro.anotar("IGNORADO", &format!("doble GO; seguía en espera {d}"));
                return false;
            }
        }
        self.ultimo_go = Some(Instant::now());
        let i = self.en_espera;
        let d = self.describir(i);
        self.registro.anotar("GO", &d);
        self.ultimo_disparado = Some(i);
        self.en_espera = i + 1;
        self.aviso = None;
        true
    }

    pub fn mover_espera(&mut self, delta: i64) {
        if self.show.cues.is_empty() {
            return;
        }
        let max = self.show.cues.len() as i64; // len = después del último (fin de la lista)
        self.en_espera = (self.en_espera as i64 + delta).clamp(0, max) as usize;
    }

    pub fn parar_todo(&mut self) {
        self.registro.anotar("PARAR", "Parar todo");
        self.aviso = None;
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
                self.aviso = Some(format!("No se pudo guardar solo: {e}"));
                self.registro.anotar("ERROR", &e.to_string());
            }
        }
    }

    pub fn anotar_error(&mut self, texto: &str) {
        self.registro.anotar("ERROR", texto);
    }

    /// Lo que la ventana necesita para dibujarse.
    pub fn estado_json(&self) -> String {
        let cues: Vec<_> = self
            .show
            .cues
            .iter()
            .enumerate()
            .map(|(i, c)| {
                json!({
                    "numero": if c.numero.is_empty() { (i + 1).to_string() } else { c.numero.clone() },
                    "nombre": c.nombre,
                    "tipo": match c.tipo { TipoCue::Audio => "audio", TipoCue::Video => "video", TipoCue::Imagen => "imagen" },
                    "nota": c.nota,
                    "bucle": c.bucle,
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
            "aviso": self.aviso,
        })
        .to_string()
    }
}
