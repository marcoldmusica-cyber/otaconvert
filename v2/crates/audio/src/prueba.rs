//! Escenario de estrés compartido por las pruebas automáticas y por
//! `otaconvert-motor --prueba-vivo`: miles de órdenes al azar (pero
//! repetibles con la misma semilla) sobre tonos de prueba.

use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use crate::{Comando, Control, Detector, Pista};

const AMPLITUD: f32 = 0.12;
/// Voces simultáneas que el escenario intenta no pasar.
const VOCES: u32 = 5;
/// Peor caso contando las que todavía se están apagando.
const VOCES_PEOR_CASO: f32 = 8.0;

/// Generador pseudoaleatorio sencillo (xorshift64*), sin dependencias.
#[derive(Debug, Clone)]
pub struct Azar(u64);

impl Azar {
    pub fn nuevo(semilla: u64) -> Self {
        Self(semilla.max(1))
    }

    pub fn u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Entero en `[0, n)`.
    pub fn hasta(&mut self, n: u64) -> u64 {
        self.u64() % n.max(1)
    }

    /// Real en `[a, b)`.
    pub fn entre(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * (self.u64() >> 40) as f32 / (1u64 << 24) as f32
    }
}

struct Entrada {
    pista: Arc<Pista>,
    bucle: bool,
}

pub struct Escenario {
    azar: Azar,
    frecuencia: u32,
    pistas: Vec<Entrada>,
    hz_maximo: f32,
    recientes: [u32; 8],
    siguiente_id: u32,
    pub ordenes: u64,
}

impl Escenario {
    pub fn nuevo(semilla: u64, frecuencia: u32) -> Self {
        let fs = frecuencia as f32;
        let mut pistas = Vec::new();
        let mut hz_maximo: f32 = 0.0;
        // Tonos que terminan en cualquier punto de la onda: el final del
        // archivo pone a prueba la rampa de salida.
        for (i, hz) in [220.0, 277.18, 329.63, 440.0, 554.37, 659.25].into_iter().enumerate() {
            let segundos = 1.3 + 0.47 * i as f32;
            let canales = if i % 2 == 0 { 2 } else { 1 };
            let p = Pista::tono(hz, AMPLITUD, (segundos * fs) as usize, canales, frecuencia);
            pistas.push(Entrada { pista: Arc::new(p), bucle: false });
            hz_maximo = hz_maximo.max(hz);
        }
        // Bucles: periodo entero de 100 y 80 cuadros y largo múltiplo de 400,
        // así la vuelta al principio es continua a cualquier frecuencia.
        let largo = 400 * (frecuencia as usize / 400).max(1);
        for periodo in [100.0, 80.0] {
            let hz = fs / periodo;
            let p = Pista::tono(hz, AMPLITUD, largo, 2, frecuencia);
            pistas.push(Entrada { pista: Arc::new(p), bucle: true });
            hz_maximo = hz_maximo.max(hz);
        }
        Self {
            azar: Azar::nuevo(semilla),
            frecuencia,
            pistas,
            hz_maximo,
            recientes: [0; 8],
            siguiente_id: 1,
            ordenes: 0,
        }
    }

    /// Umbral del detector para este escenario.
    pub fn umbral(&self) -> f32 {
        Detector::umbral_para_tonos(AMPLITUD * VOCES_PEOR_CASO, self.hz_maximo, self.frecuencia)
    }

    fn cuadros(&mut self, min_s: f32, max_s: f32) -> u32 {
        (self.azar.entre(min_s, max_s) * self.frecuencia as f32) as u32
    }

    fn id_reciente(&mut self) -> u32 {
        self.recientes[self.azar.hasta(8) as usize]
    }

    /// Envía una orden al azar y devuelve cuántos cuadros esperar hasta la próxima.
    pub fn paso(&mut self, control: &mut Control) -> u32 {
        let activas = control.estadisticas.voces_activas.load(Relaxed);
        let tirada = self.azar.hasta(100);
        let comando = if tirada < 35 && activas < VOCES {
            let e = &self.pistas[self.azar.hasta(self.pistas.len() as u64) as usize];
            let (pista, bucle) = (e.pista.clone(), e.bucle);
            let desde = self.azar.hasta(pista.cuadros() as u64 / 2) as usize;
            let fundido = if self.azar.hasta(10) < 3 { self.cuadros(0.0, 0.5) } else { 0 };
            let id = self.siguiente_id;
            self.siguiente_id += 1;
            self.recientes[self.azar.hasta(8) as usize] = id;
            let volumen = self.azar.entre(0.3, 1.0);
            Comando::Iniciar { id, pista, volumen, desde, bucle, fundido }
        } else if tirada < 55 {
            let fundido = if self.azar.hasta(2) == 0 { 0 } else { self.cuadros(0.0, 1.0) };
            Comando::Parar { id: self.id_reciente(), fundido }
        } else if tirada < 65 {
            Comando::Pausar { id: self.id_reciente() }
        } else if tirada < 75 {
            Comando::Reanudar { id: self.id_reciente() }
        } else if tirada < 85 {
            let volumen = self.azar.entre(0.0, 1.0);
            let cuadros = self.cuadros(0.0, 0.3);
            Comando::Volumen { id: self.id_reciente(), volumen, cuadros }
        } else if tirada < 95 {
            let cuadro = self.azar.hasta(self.frecuencia as u64) as usize;
            Comando::Saltar { id: self.id_reciente(), cuadro }
        } else if tirada < 98 {
            let fundido = if self.azar.hasta(2) == 0 { 0 } else { self.cuadros(0.0, 2.0) };
            Comando::PararTodo { fundido }
        } else {
            let volumen = self.azar.entre(0.5, 1.0);
            let cuadros = self.cuadros(0.0, 0.5);
            Comando::VolumenMaestro { volumen, cuadros }
        };
        if control.enviar(comando).is_ok() {
            self.ordenes += 1;
        }
        self.cuadros(0.05, 0.6)
    }
}
