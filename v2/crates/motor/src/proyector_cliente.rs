//! El motor abre y vigila el proceso de proyección (`proyector.rs`).
//!
//! - Le manda órdenes por la entrada estándar y lee sus avisos de la salida.
//! - **Vigilante:** si el proceso termina, o pasa 1 s sin latido, se cierra y
//!   se vuelve a abrir (en negro). Cuando el nuevo dice "hola" se avisa
//!   `Reiniciado` y la sesión vuelve a precargar y a mostrar lo que se veía.
//! - Más de 3 caídas en un minuto: se deja de intentar y se avisa `Caido`.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use otaconvert_servidor::proyector::motivo;
use otaconvert_servidor::{AvisoProyector, OrdenProyector, Proyector, Texto};
use serde_json::{json, Value};

const SIN_LATIDO: Duration = Duration::from_secs(1);
const LIMITE_ARRANQUE: Duration = Duration::from_secs(10);
const MAX_CAIDAS: usize = 3;
const VENTANA_CAIDAS: Duration = Duration::from_secs(60);

/// Lo que comparte el hilo lector de un proceso con el cliente.
#[derive(Default)]
struct Compartido {
    avisos: Vec<Value>,
    latido: Option<Instant>,
    hola: bool,
    terminado: bool,
}

struct Proceso {
    hijo: Child,
    entrada: ChildStdin,
    compartido: Arc<Mutex<Compartido>>,
    arranque: Instant,
}

pub struct ProyectorProceso {
    args: Vec<String>,
    proceso: Option<Proceso>,
    /// Desde cuándo se está reiniciando (para medir cuánto tardó).
    reiniciando: Option<Instant>,
    caidas: Vec<Instant>,
}

fn lanzar(args: &[String]) -> Result<Proceso, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut hijo = Command::new(exe)
        .arg("--proyector")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("no se pudo abrir la proyección: {e}"))?;
    let entrada = hijo.stdin.take().ok_or("sin entrada")?;
    let salida = hijo.stdout.take().ok_or("sin salida")?;
    let compartido = Arc::new(Mutex::new(Compartido::default()));
    let c = compartido.clone();
    std::thread::Builder::new()
        .name("avisos-proyector".into())
        .spawn(move || {
            for linea in BufReader::new(salida).lines() {
                let Ok(l) = linea else { break };
                let Ok(v) = serde_json::from_str::<Value>(&l) else { continue };
                let Ok(mut g) = c.lock() else { break };
                match v["a"].as_str() {
                    Some("latido") => g.latido = Some(Instant::now()),
                    Some("hola") => {
                        g.hola = true;
                        g.latido = Some(Instant::now());
                    }
                    _ => g.avisos.push(v),
                }
            }
            if let Ok(mut g) = c.lock() {
                g.terminado = true;
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(Proceso { hijo, entrada, compartido, arranque: Instant::now() })
}

impl ProyectorProceso {
    /// Abre la proyección en el rectángulo dado; `vista` = vista previa (una sola pantalla).
    pub fn abrir(x: i32, y: i32, ancho: i32, alto: i32, vista: bool) -> Result<Self, String> {
        let args = vec![
            x.to_string(),
            y.to_string(),
            ancho.to_string(),
            alto.to_string(),
            if vista { "vista" } else { "completa" }.to_string(),
        ];
        let proceso = Some(lanzar(&args)?);
        Ok(Self { args, proceso, reiniciando: None, caidas: Vec::new() })
    }

    /// Espera a que el proceso diga "hola" (para pruebas y para el arranque).
    pub fn esperar_hola(&self, limite: Duration) -> bool {
        let fin = Instant::now() + limite;
        while Instant::now() < fin {
            if self.proceso.as_ref().is_some_and(|p| p.compartido.lock().is_ok_and(|g| g.hola)) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    /// Solo para pruebas: cierra el proceso a la fuerza (como si se cayera).
    pub fn matar_para_probar(&mut self) {
        if let Some(p) = self.proceso.as_mut() {
            let _ = p.hijo.kill();
        }
    }

    fn necesita_reinicio(p: &mut Proceso) -> bool {
        if matches!(p.hijo.try_wait(), Ok(Some(_))) {
            return true;
        }
        let Ok(g) = p.compartido.lock() else { return true };
        if g.terminado {
            return true;
        }
        if !g.hola {
            return p.arranque.elapsed() > LIMITE_ARRANQUE;
        }
        g.latido.is_none_or(|t| t.elapsed() > SIN_LATIDO)
    }
}

fn convertir(v: &Value) -> Option<AvisoProyector> {
    let id = v["id"].as_u64().unwrap_or(0);
    Some(match v["a"].as_str()? {
        "listo" => AvisoProyector::Listo { id, duracion: v["duracion"].as_f64() },
        "error" => AvisoProyector::Error { id, motivo: motivo(v["motivo"].as_str().unwrap_or("")) },
        "mostrando" => AvisoProyector::Mostrando { id },
        "fin" => AvisoProyector::Fin { id },
        _ => return None,
    })
}

impl Proyector for ProyectorProceso {
    fn enviar(&mut self, orden: OrdenProyector) {
        let v = match orden {
            OrdenProyector::Precargar { id, ruta, bucle, imagen, inicio_s, fin_s, desde_s } => json!({
                "o": "precargar", "id": id, "ruta": ruta.to_string_lossy(), "bucle": bucle,
                "imagen": imagen, "inicio": inicio_s, "fin": fin_s, "desde": desde_s,
            }),
            OrdenProyector::Mostrar { id } => json!({ "o": "mostrar", "id": id }),
            OrdenProyector::Negro => json!({ "o": "negro" }),
            OrdenProyector::Fundir { ms } => json!({ "o": "fundir", "ms": ms }),
            OrdenProyector::Pausa(si) => json!({ "o": "pausa", "si": si }),
            OrdenProyector::Quitar { id } => json!({ "o": "quitar", "id": id }),
        };
        if let Some(p) = self.proceso.as_mut() {
            // Si el proceso se cayó, el vigilante lo nota en `recoger`.
            let _ = writeln!(p.entrada, "{v}").and_then(|_| p.entrada.flush());
        }
    }

    fn recoger(&mut self) -> Vec<AvisoProyector> {
        let mut avisos = Vec::new();
        let Some(p) = self.proceso.as_mut() else { return avisos };
        let (crudos, hola) = match p.compartido.lock() {
            Ok(mut g) => (std::mem::take(&mut g.avisos), g.hola),
            Err(_) => (Vec::new(), false),
        };
        // El reinicio terminó: el proceso nuevo ya está.
        if hola {
            if let Some(t) = self.reiniciando.take() {
                avisos.push(AvisoProyector::Reiniciado { ms: t.elapsed().as_millis() as u64 });
            }
        }
        avisos.extend(crudos.iter().filter_map(convertir));

        if Self::necesita_reinicio(p) {
            let _ = p.hijo.kill();
            let _ = p.hijo.wait();
            let ahora = Instant::now();
            self.caidas.retain(|t| ahora.duration_since(*t) < VENTANA_CAIDAS);
            self.caidas.push(ahora);
            if self.caidas.len() > MAX_CAIDAS {
                self.proceso = None;
                self.reiniciando = None;
                avisos.push(AvisoProyector::Caido {
                    motivo: Texto::nuevo(
                        "se cerró sola varias veces seguidas; revisa la tarjeta gráfica y el proyector",
                        "it closed by itself several times in a row; check the graphics card and the projector",
                    ),
                });
                return avisos;
            }
            self.reiniciando.get_or_insert(ahora);
            match lanzar(&self.args) {
                Ok(nuevo) => self.proceso = Some(nuevo),
                Err(e) => {
                    self.proceso = None;
                    self.reiniciando = None;
                    avisos.push(AvisoProyector::Caido { motivo: Texto::from(e) });
                }
            }
        }
        avisos
    }
}

impl Drop for ProyectorProceso {
    fn drop(&mut self) {
        if let Some(p) = self.proceso.as_mut() {
            let _ = writeln!(p.entrada, "{}", json!({ "o": "salir" }));
            let _ = p.entrada.flush();
            let fin = Instant::now() + Duration::from_millis(500);
            while Instant::now() < fin && matches!(p.hijo.try_wait(), Ok(None)) {
                std::thread::sleep(Duration::from_millis(10));
            }
            let _ = p.hijo.kill();
            let _ = p.hijo.wait();
        }
    }
}
