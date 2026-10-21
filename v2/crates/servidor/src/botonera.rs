//! Botonera de sonidos (CLAUDE.md, sección 5b): sonidos sueltos que se
//! disparan a mano **superpuestos** a lo que suene, sin mover el cue en espera
//! ni contar como GO. Sus audios quedan **siempre en memoria** desde que se abre
//! el show (se preparan después de los cues en espera), para sonar al instante.
//! Parar todo y Fundir todo también los paran o funden; Mayús+F no los toca.

use super::*;
use otaconvert_show::{AlRepetir, BotonSonido, PaginaBotonera};

/// Límites de lo que se puede armar desde la ventana.
const MAX_PAGINAS: usize = 20;
const MAX_BOTONES: usize = 64;
const MAX_NOMBRE_BOTON: usize = 60;
/// Teclas que ya usa el programa: no se pueden dar a un botón.
const TECLAS_RESERVADAS: [&str; 3] = ["P", "F", "H"];

/// Una voz de la botonera sonando (en el mezclador con este `id`).
pub(super) struct VozBoton {
    pub id: u32,
    pub pagina: usize,
    pub boton: usize,
    pub destino: Destino,
    /// Cuándo termina sola (`None` = en bucle).
    pub fin: Option<Instant>,
    pub fundiendo: bool,
}

/// Preparar el audio de un botón (en el hilo preparador, sin la sesión tomada).
pub struct TareaBoton {
    pub clave: String,
    pub ruta: PathBuf,
    pub frecuencia: u32,
    pub bucle: bool,
}

impl TareaBoton {
    pub fn hacer(&self) -> Result<otaconvert_audio::preparar::Preparado, otaconvert_audio::preparar::ErrorPreparar> {
        otaconvert_audio::preparar::preparar_con(&self.ruta, self.frecuencia, self.bucle)
    }
}

/// Un mismo archivo en bucle o no se prepara distinto (el empalme del bucle).
fn clave(b: &BotonSonido) -> String {
    format!("{}|{}", b.archivo, b.bucle)
}

/// Tecla válida para un botón: una letra (salvo P, F y H), un número o F1–F12.
pub fn tecla_valida(t: &str) -> bool {
    let t = t.to_ascii_uppercase();
    if TECLAS_RESERVADAS.contains(&t.as_str()) {
        return false;
    }
    let mut c = t.chars();
    match (c.next(), c.next()) {
        (Some(x), None) => x.is_ascii_alphanumeric(),
        (Some('F'), Some(_)) => t[1..].parse::<u8>().is_ok_and(|n| (1..=12).contains(&n)),
        _ => false,
    }
}

impl Sesion {
    /// El próximo audio de la botonera que falta tener en memoria.
    pub fn siguiente_boton_a_preparar(&mut self) -> Option<TareaBoton> {
        let fs = self.sonido.as_ref()?.frecuencia();
        if self.boton_preparando.is_some() {
            return None;
        }
        let carpeta = self.carpeta();
        for b in self.show.botonera.iter().flat_map(|p| &p.botones) {
            let k = clave(b);
            if self.botones_prep.contains_key(&k) || self.botones_err.contains_key(&k) {
                continue;
            }
            match rutas::resolver(&carpeta, &b.archivo) {
                Ok(ruta) if ruta.is_file() => {
                    self.boton_preparando = Some(k.clone());
                    return Some(TareaBoton { clave: k, ruta, frecuencia: fs, bucle: b.bucle });
                }
                Ok(_) => {
                    self.botones_err.insert(k, t(format!("Falta el archivo {}.", b.archivo), format!("The file {} is missing.", b.archivo)));
                }
                Err(e) => {
                    self.botones_err.insert(k, t(format!("Ruta no válida ({e})."), format!("Invalid path ({e}).")));
                }
            }
        }
        None
    }

    pub fn resultado_boton(&mut self, k: String, r: Result<otaconvert_audio::preparar::Preparado, otaconvert_audio::preparar::ErrorPreparar>) {
        if self.boton_preparando.as_deref() == Some(k.as_str()) {
            self.boton_preparando = None;
        }
        // La frecuencia pudo cambiar (otra tarjeta) mientras se preparaba.
        let fs = self.sonido.as_ref().map(|s| s.frecuencia());
        match r {
            Ok(p) if Some(p.pista.frecuencia()) == fs => {
                if p.pico > 1.0 {
                    self.registro.anotar("AVISO", &format!("botonera {}: pasa de 0 dB (pico {:.2}); el limitador lo controla", k, p.pico));
                }
                self.botones_prep.insert(k, Arc::new(p.pista));
            }
            Ok(_) => {}
            Err(e) => {
                let e = Texto::from(e);
                self.registro.anotar("ERROR", &format!("botonera {k}: {}", e.es));
                self.botones_err.insert(k, e);
            }
        }
    }

    /// Toca un botón (Edición y Función). Lo que pasa si ya suena lo dice el
    /// botón: parar (por defecto), volver a empezar o superponer otra copia.
    pub fn tocar_boton(&mut self, pagina: usize, boton: usize) -> Result<(), Texto> {
        let b = self
            .show
            .botonera
            .get(pagina)
            .and_then(|p| p.botones.get(boton))
            .cloned()
            .ok_or_else(|| t("Ese botón no existe.", "That button does not exist."))?;
        if self.sonido.is_none() {
            return Err(t("No hay salida de sonido.", "There is no sound output."));
        }
        let k = clave(&b);
        let Some(pista) = self.botones_prep.get(&k).cloned() else {
            return Err(match self.botones_err.get(&k) {
                Some(e) => Texto::nuevo(format!("«{}»: {}", b.nombre, e.es), format!("“{}”: {}", b.nombre, e.en)),
                None => t(format!("«{}» todavía se está preparando.", b.nombre), format!("“{}” is still being prepared.", b.nombre)),
            });
        };
        let activas: Vec<u32> = self.voces_boton.iter().filter(|v| v.pagina == pagina && v.boton == boton && !v.fundiendo).map(|v| v.id).collect();
        if !activas.is_empty() {
            match b.al_repetir {
                AlRepetir::Parar => {
                    let f = self.cuadros(b.fundido_salida_ms as u64) as u32;
                    let fin = Instant::now() + Duration::from_millis(b.fundido_salida_ms as u64);
                    for id in activas {
                        if let Some(v) = self.voces_boton.iter_mut().find(|v| v.id == id) {
                            let destino = v.destino;
                            v.fundiendo = true;
                            v.fin = Some(fin);
                            if let Some(s) = self.sonido.as_mut() {
                                let _ = s.enviar(destino, Comando::Parar { id, fundido: f });
                            }
                        }
                    }
                    self.registro.anotar("BOTON", &format!("«{}» (parar)", b.nombre));
                    return Ok(());
                }
                AlRepetir::Reiniciar => {
                    for id in activas {
                        if let Some(k) = self.voces_boton.iter().position(|v| v.id == id) {
                            let v = self.voces_boton.remove(k);
                            if let Some(s) = self.sonido.as_mut() {
                                let _ = s.enviar(v.destino, Comando::Parar { id, fundido: 0 });
                            }
                        }
                    }
                }
                AlRepetir::Superponer => {}
            }
        }
        let destino = if b.salida == "Auriculares" && self.sonido.as_ref().is_some_and(|s| s.tiene_auriculares()) {
            Destino::Auriculares
        } else {
            Destino::Principal
        };
        let id = self.siguiente_id;
        self.siguiente_id = self.siguiente_id.wrapping_add(1).max(1);
        let duracion = Duration::from_secs_f64(pista.cuadros() as f64 / pista.frecuencia() as f64);
        let comando = Comando::IniciarCue {
            id,
            pista,
            volumen: db_a_lineal(b.volumen_db),
            inicio: 0,
            desde: 0,
            hasta: None,
            bucle: b.bucle,
            fundido_entrada: self.cuadros(b.fundido_entrada_ms as u64) as u32,
            fundido_salida: self.cuadros(b.fundido_salida_ms as u64) as u32,
        };
        self.sonido
            .as_mut()
            .map(|s| s.enviar(destino, comando))
            .unwrap_or(Ok(()))
            .map_err(|e| t(format!("No se pudo tocar «{}»: {e}", b.nombre), format!("Could not play “{}”: {e}", b.nombre)))?;
        self.voces_boton.push(VozBoton { id, pagina, boton, destino, fin: if b.bucle { None } else { Some(Instant::now() + duracion) }, fundiendo: false });
        self.registro.anotar("BOTON", &format!("«{}»", b.nombre));
        Ok(())
    }

    /// Una tecla de la ventana: si es la de un botón, lo toca. `true` = era de un botón.
    pub fn tecla_boton(&mut self, tecla: &str) -> bool {
        let tecla = tecla.to_ascii_uppercase();
        let lugar = self.show.botonera.iter().enumerate().find_map(|(p, pag)| {
            pag.botones.iter().position(|b| b.tecla.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(&tecla))).map(|b| (p, b))
        });
        match lugar {
            Some((p, b)) => {
                if let Err(e) = self.tocar_boton(p, b) {
                    self.aviso = Some(e);
                }
                true
            }
            None => false,
        }
    }

    /// Reemplaza la botonera entera (la arma la ventana, en Edición). Se valida
    /// todo antes de cambiar nada y se puede deshacer.
    pub fn editar_botonera(&mut self, v: &serde_json::Value) -> Result<(), Texto> {
        self.solo_en_edicion()?;
        let nueva: Vec<PaginaBotonera> = serde_json::from_value(v.clone()).map_err(|e| t(format!("Botonera no válida: {e}"), format!("Invalid sound board: {e}")))?;
        if nueva.len() > MAX_PAGINAS {
            return Err(t(format!("Como mucho {MAX_PAGINAS} páginas."), format!("At most {MAX_PAGINAS} pages.")));
        }
        let mut teclas = std::collections::HashSet::new();
        for p in &nueva {
            if p.nombre.chars().count() > MAX_NOMBRE_BOTON || p.nombre.chars().any(char::is_control) {
                return Err(t("Nombre de página no válido.", "Invalid page name."));
            }
            if p.botones.len() > MAX_BOTONES {
                return Err(t(format!("Como mucho {MAX_BOTONES} botones por página."), format!("At most {MAX_BOTONES} buttons per page.")));
            }
            for b in &p.botones {
                let nombre_ok = !b.nombre.trim().is_empty() && b.nombre.chars().count() <= MAX_NOMBRE_BOTON && !b.nombre.chars().any(char::is_control);
                if !nombre_ok {
                    return Err(t("Cada botón necesita un nombre (hasta 60 letras).", "Each button needs a name (up to 60 letters)."));
                }
                let color_ok = b.color.len() == 7 && b.color.starts_with('#') && b.color[1..].chars().all(|c| c.is_ascii_hexdigit());
                if !color_ok {
                    return Err(t(format!("«{}»: color no válido.", b.nombre), format!("“{}”: invalid colour.", b.nombre)));
                }
                if !(-60.0..=12.0).contains(&b.volumen_db) || b.fundido_entrada_ms > 600_000 || b.fundido_salida_ms > 600_000 {
                    return Err(t(format!("«{}»: volumen o fundido fuera de rango.", b.nombre), format!("“{}”: volume or fade out of range.", b.nombre)));
                }
                if b.salida != "Principal" && b.salida != "Auriculares" {
                    return Err(t(format!("«{}»: salida no válida.", b.nombre), format!("“{}”: invalid output.", b.nombre)));
                }
                if otaconvert_show::tipo_por_extension(std::path::Path::new(&b.archivo)) != Some(TipoCue::Audio) {
                    return Err(t(format!("«{}»: el sonido tiene que ser un audio.", b.nombre), format!("“{}”: the sound must be an audio file.", b.nombre)));
                }
                if let Some(k) = &b.tecla {
                    if !tecla_valida(k) {
                        return Err(t(
                            format!("«{}»: la tecla «{k}» no se puede usar (una letra salvo P, F y H, un número o F1 a F12).", b.nombre),
                            format!("“{}”: the key “{k}” can't be used (a letter except P, F and H, a digit or F1 to F12).", b.nombre),
                        ));
                    }
                    if !teclas.insert(k.to_ascii_uppercase()) {
                        return Err(t(format!("La tecla «{k}» está en dos botones."), format!("The key “{k}” is on two buttons.")));
                    }
                }
            }
        }
        let mut prueba = self.show.clone();
        prueba.botonera = nueva.clone();
        otaconvert_show::validar(&prueba).map_err(|e| t(format!("Botonera no válida: {e}"), format!("Invalid sound board: {e}")))?;
        if nueva == self.show.botonera {
            return Ok(());
        }
        self.antes_de_cambiar(None);
        self.show.botonera = nueva;
        // Lo que suena sigue, pero ya no se puede tocar de nuevo para pararlo
        // (los botones pudieron cambiar de lugar): Parar todo lo para.
        self.voces_boton.clear();
        self.botones_err.clear();
        self.registro.anotar("BOTONERA", &format!("{} página(s)", self.show.botonera.len()));
        Ok(())
    }

    /// Suelta de memoria los audios que ningún botón usa y olvida lo que ya terminó.
    pub(super) fn limpiar_botones(&mut self) {
        let ahora = Instant::now();
        self.voces_boton.retain(|v| v.fin.is_none_or(|f| f > ahora));
        let usadas: std::collections::HashSet<String> = self.show.botonera.iter().flat_map(|p| &p.botones).map(clave).collect();
        self.botones_prep.retain(|k, _| usadas.contains(k));
    }

    pub(super) fn estado_botonera(&self) -> serde_json::Value {
        let paginas: Vec<serde_json::Value> = self
            .show
            .botonera
            .iter()
            .enumerate()
            .map(|(p, pag)| {
                let botones: Vec<serde_json::Value> = pag
                    .botones
                    .iter()
                    .enumerate()
                    .map(|(b, bt)| {
                        let k = clave(bt);
                        let preparacion = if self.botones_prep.contains_key(&k) {
                            "listo"
                        } else if self.botones_err.contains_key(&k) {
                            "error"
                        } else if self.sonido.is_some() {
                            "preparando"
                        } else {
                            ""
                        };
                        json!({
                            "nombre": bt.nombre, "color": bt.color, "archivo": bt.archivo, "volumen_db": bt.volumen_db,
                            "fundido_entrada_ms": bt.fundido_entrada_ms, "fundido_salida_ms": bt.fundido_salida_ms,
                            "bucle": bt.bucle, "salida": bt.salida, "tecla": bt.tecla, "al_repetir": bt.al_repetir,
                            "preparacion": preparacion,
                            "error": self.botones_err.get(&k).map(|e| e.en_idioma(self.idioma)),
                            "sonando": self.voces_boton.iter().any(|v| v.pagina == p && v.boton == b && !v.fundiendo),
                        })
                    })
                    .collect();
                json!({ "nombre": pag.nombre, "botones": botones })
            })
            .collect();
        json!(paginas)
    }
}
