/// Busca chasquidos en una señal ya mezclada.
///
/// Mide la segunda diferencia `x[n] - 2·x[n-1] + x[n-2]` de cada canal. En una
/// suma de senos de amplitud total `A` y frecuencia máxima `f` nunca supera
/// `A·(2·sin(π·f/fs))²`; un corte o un arranque de golpe la dispara hasta el
/// tamaño del salto. Así se detectan saltos de apenas 0,01 aunque la señal
/// sea mucho más grande.
#[derive(Debug, Clone)]
pub struct Detector {
    canales: usize,
    umbral: f32,
    previo1: Vec<f32>,
    previo2: Vec<f32>,
    cuadros: u64,
    /// Cuadros donde la segunda diferencia superó el umbral.
    pub discontinuidades: u64,
    /// (cuadro, canal, valor) de la primera discontinuidad, para depurar.
    pub primera: Option<(u64, usize, f32)>,
    pub maxima: f32,
    /// Muestras por encima de 0 dBFS.
    pub recortes: u64,
    pub no_numeros: u64,
}

impl Detector {
    /// La señal se supone en silencio antes del primer bloque.
    pub fn nuevo(canales: usize, umbral: f32) -> Self {
        Self {
            canales,
            umbral,
            previo1: vec![0.0; canales],
            previo2: vec![0.0; canales],
            cuadros: 0,
            discontinuidades: 0,
            primera: None,
            maxima: 0.0,
            recortes: 0,
            no_numeros: 0,
        }
    }

    /// Umbral para una suma de tonos con margen del 50 % y una holgura fija
    /// para las esquinas de las rampas.
    pub fn umbral_para_tonos(amplitud_total: f32, hz_maximo: f32, frecuencia: u32) -> f32 {
        let w = 2.0 * (std::f32::consts::PI * hz_maximo / frecuencia as f32).sin();
        1.5 * amplitud_total * w * w + 0.002
    }

    /// No pide memoria: se puede llamar desde el hilo de audio.
    pub fn analizar(&mut self, bloque: &[f32]) {
        for cuadro in bloque.chunks_exact(self.canales) {
            for (c, &x) in cuadro.iter().enumerate() {
                if !x.is_finite() {
                    self.no_numeros += 1;
                    continue;
                }
                if x.abs() > 1.0 {
                    self.recortes += 1;
                }
                let d = (x - 2.0 * self.previo1[c] + self.previo2[c]).abs();
                if d > self.maxima {
                    self.maxima = d;
                }
                if d > self.umbral {
                    if self.primera.is_none() {
                        self.primera = Some((self.cuadros, c, d));
                    }
                    self.discontinuidades += 1;
                }
                self.previo2[c] = self.previo1[c];
                self.previo1[c] = x;
            }
            self.cuadros += 1;
        }
    }

    pub fn cuadros(&self) -> u64 {
        self.cuadros
    }
}

#[cfg(test)]
mod pruebas {
    use super::Detector;
    use crate::Pista;

    #[test]
    fn tono_limpio_no_da_falsos_avisos() {
        let fs = 48_000;
        let p = Pista::tono(1000.0, 0.9, fs as usize, 1, fs);
        let mut d = Detector::nuevo(1, Detector::umbral_para_tonos(0.9, 1000.0, fs));
        let muestras: Vec<f32> = (0..p.cuadros()).map(|i| p.cuadro(i).0).collect();
        // Arrancar un seno sin rampa sí es un salto de pendiente: se ignoran
        // los dos primeros cuadros, que solo llenan la memoria del detector.
        d.analizar(&muestras[..2]);
        d.discontinuidades = 0;
        d.analizar(&muestras[2..]);
        assert_eq!(d.discontinuidades, 0, "máxima {}", d.maxima);
    }

    #[test]
    fn detecta_un_salto_pequeno() {
        let mut d = Detector::nuevo(1, 0.01);
        d.analizar(&[0.0, 0.0, 0.0, 0.02, 0.02, 0.02]);
        assert!(d.discontinuidades > 0);
    }
}
