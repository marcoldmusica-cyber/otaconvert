//! Mezclador de audio de Otaconvert V2 (Fase 0).
//!
//! Reglas que cumple el hilo de audio (`Mezclador::procesar`):
//! - No pide ni libera memoria: los comandos llegan por una cola sin bloqueo
//!   y los audios que dejan de sonar se devuelven al hilo de control para
//!   liberarlos allí.
//! - Nunca cambia el volumen de golpe: todo inicio, parada, pausa, salto y
//!   cambio de volumen pasa por una rampa.
//! - No remuestrea: rechaza audios que no estén a la frecuencia de la salida.

mod detector;
mod mezclador;
mod pista;
pub mod prueba;
mod rampa;

pub use detector::Detector;
pub use mezclador::{crear, Ajustes, Comando, Control, Estadisticas, Mezclador};
pub use pista::Pista;
pub use rampa::Rampa;
