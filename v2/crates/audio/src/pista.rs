/// Un audio entero en memoria, ya decodificado a la frecuencia de la salida.
/// Muestras intercaladas en float 32 (L, R, L, R…).
#[derive(Debug)]
pub struct Pista {
    muestras: Vec<f32>,
    canales: u16,
    frecuencia: u32,
}

impl Pista {
    pub fn nueva(muestras: Vec<f32>, canales: u16, frecuencia: u32) -> Self {
        assert!(canales > 0, "una pista necesita al menos un canal");
        assert_eq!(muestras.len() % canales as usize, 0, "faltan muestras en el último cuadro");
        Self { muestras, canales, frecuencia }
    }

    /// Tono senoidal que empieza en fase 0. Útil para pruebas.
    pub fn tono(hz: f32, amplitud: f32, cuadros: usize, canales: u16, frecuencia: u32) -> Self {
        let paso = std::f64::consts::TAU * hz as f64 / frecuencia as f64;
        let mut muestras = Vec::with_capacity(cuadros * canales as usize);
        for n in 0..cuadros {
            let s = (amplitud as f64 * (paso * n as f64).sin()) as f32;
            for _ in 0..canales {
                muestras.push(s);
            }
        }
        Self::nueva(muestras, canales, frecuencia)
    }

    pub fn cuadros(&self) -> usize {
        self.muestras.len() / self.canales as usize
    }

    /// Muestras intercaladas (para medir y para la onda).
    pub fn muestras(&self) -> &[f32] {
        &self.muestras
    }

    pub fn canales(&self) -> u16 {
        self.canales
    }

    pub fn frecuencia(&self) -> u32 {
        self.frecuencia
    }

    /// Pico absoluto (0 dBFS = 1.0). La comprobación previa marca > 1.0.
    pub fn pico(&self) -> f32 {
        self.muestras.iter().fold(0.0f32, |m, s| m.max(s.abs()))
    }

    /// Izquierda y derecha del cuadro `pos`. Una pista mono suena igual en ambas.
    #[inline]
    pub(crate) fn cuadro(&self, pos: usize) -> (f32, f32) {
        let c = self.canales as usize;
        let i = pos * c;
        if c == 1 {
            let s = self.muestras[i];
            (s, s)
        } else {
            (self.muestras[i], self.muestras[i + 1])
        }
    }
}
