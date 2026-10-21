//! Lo que la sesión necesita del sonido, sin depender de la tarjeta: así las
//! pruebas usan un sonido falso y el motor usa cpal (WASAPI).

use otaconvert_audio::Comando;

/// Salida lógica (CLAUDE.md: "Salidas lógicas y perfiles de teatro").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destino {
    Principal,
    Auriculares,
}

pub trait Sonido: Send {
    /// Frecuencia a la que se preparan los audios (la de la salida Principal).
    fn frecuencia(&self) -> u32;
    fn tiene_auriculares(&self) -> bool;
    /// Envía un comando al mezclador de esa salida.
    fn enviar(&mut self, destino: Destino, comando: Comando) -> Result<(), String>;
    /// Libera los audios que ya no suenan (fuera del hilo de audio).
    fn recoger_basura(&mut self);
    /// Medidor de nivel: el pico (0 a 1) de cada lado de cada salida desde la
    /// última vez, y si el limitador tuvo que actuar. Un sonido falso no mide nada.
    fn niveles(&mut self) -> Vec<(Destino, [f32; 2], bool)> {
        Vec::new()
    }
}
