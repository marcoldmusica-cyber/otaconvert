//! Mezclador de audio de Otaconvert V2 (Fase 0).
//!
//! Reglas que cumple el hilo de audio (`Mezclador::procesar`):
//! - No pide ni libera memoria: los comandos llegan por una cola sin bloqueo
//!   y los audios que dejan de sonar se devuelven al hilo de control para
//!   liberarlos allí.
//! - Cada inicio, parada, pausa, reanudación, salto, cambio de volumen y
//!   final de archivo usa la rampa que elija el usuario (`Ajustes`, también
//!   en vivo con `Comando::CambiarAjustes`); 0 = de golpe. Por defecto el
//!   inicio es de golpe y lo demás lleva una rampa corta.
//! - No remuestrea: rechaza audios que no estén a la frecuencia de la salida.

pub mod consejos;
mod detector;
mod mezclador;
mod pista;
pub mod prueba;
mod rampa;

pub use detector::Detector;
pub use mezclador::{crear, ms, Ajustes, Comando, Control, Estadisticas, Mezclador};
pub use pista::Pista;
pub use rampa::Rampa;
