//! Limitador maestro transparente (CLAUDE.md: "limitador maestro activo por
//! defecto").
//!
//! - Mira `L` cuadros hacia adelante (1,5 ms): retrasa la señal `L` cuadros.
//! - Para cada cuadro calcula la ganancia necesaria para no pasar el techo y
//!   toma el **mínimo** de los próximos `L`; después promedia esos mínimos en
//!   `L` cuadros. Cada promedio incluye solo valores ≤ la ganancia necesaria
//!   del cuadro que sale, así que **nunca pasa del techo**, y la bajada es
//!   una rampa suave de `L` cuadros (sin chasquido).
//! - La subida vuelve despacio (unos 100 ms) para que no "bombee".
//! - Si nada pasa del techo, la ganancia es exactamente 1: la salida es la
//!   entrada, idéntica, con 1,5 ms de retraso.
//! - Todo el espacio se reserva al crearlo: no pide memoria en el hilo de audio.

/// Techo: -0,1 dBFS.
pub const TECHO: f32 = 0.988_553;
const MIRAR_MS: f32 = 1.5;
const VOLVER_MS: f32 = 100.0;

pub struct Limitador {
    canales: usize,
    l: usize,
    /// Señal retrasada (L cuadros × canales), circular.
    retraso: Vec<f32>,
    /// Ganancia necesaria de los últimos L+1 cuadros (incluye el que sale), circular.
    necesaria: Vec<f32>,
    pos_n: usize,
    /// Mínimos de ventana de los últimos L cuadros, circular (para el promedio).
    minimos: Vec<f32>,
    suma_minimos: f64,
    pos: usize,
    ganancia: f32,
    coef_vuelta: f32,
    pub cuadros_limitados: u64,
}

impl Limitador {
    pub fn nuevo(frecuencia: u32, canales: usize) -> Self {
        let l = ((frecuencia as f32 * MIRAR_MS / 1000.0).round() as usize).max(1);
        Self {
            canales,
            l,
            retraso: vec![0.0; l * canales],
            necesaria: vec![1.0; l + 1],
            pos_n: 0,
            minimos: vec![1.0; l],
            suma_minimos: l as f64,
            pos: 0,
            ganancia: 1.0,
            coef_vuelta: 1.0 - (-1.0 / (frecuencia as f32 * VOLVER_MS / 1000.0)).exp(),
            cuadros_limitados: 0,
        }
    }

    pub fn ganancia(&self) -> f32 {
        self.ganancia
    }

    /// Retraso que agrega, en cuadros.
    pub fn retraso_cuadros(&self) -> usize {
        self.l
    }

    /// Procesa un bloque intercalado en el lugar.
    pub fn procesar(&mut self, bloque: &mut [f32]) {
        let (c, l) = (self.canales, self.l);
        for cuadro in bloque.chunks_exact_mut(c) {
            // Ganancia que necesita el cuadro que entra.
            let pico = cuadro.iter().fold(0.0f32, |p, x| p.max(x.abs()));
            let necesita = if pico > TECHO { TECHO / pico } else { 1.0 };
            self.necesaria[self.pos_n] = necesita;
            self.pos_n = (self.pos_n + 1) % (l + 1);
            // Mínimo de la ventana [t-L, t]: incluye el cuadro que sale ahora y
            // todos los que saldrán en los próximos L. Así cada promedio de
            // mínimos es ≤ la ganancia que necesita el cuadro que sale.
            let minimo = self.necesaria.iter().fold(1.0f32, |m, &g| m.min(g));
            self.suma_minimos += minimo as f64 - self.minimos[self.pos] as f64;
            self.minimos[self.pos] = minimo;
            let objetivo = ((self.suma_minimos / l as f64) as f32).min(1.0);
            // Baja al instante (ya viene suavizada); sube despacio.
            self.ganancia = if objetivo < self.ganancia {
                objetivo
            } else {
                self.ganancia + (objetivo - self.ganancia) * self.coef_vuelta
            };
            if objetivo >= 1.0 && self.ganancia > 0.999_999 {
                self.ganancia = 1.0;
            }
            if self.ganancia < 1.0 {
                self.cuadros_limitados += 1;
            }
            // Sale el cuadro de hace L cuadros con la ganancia; entra el nuevo.
            let base = self.pos * c;
            for (k, x) in cuadro.iter_mut().enumerate() {
                let viejo = self.retraso[base + k];
                self.retraso[base + k] = *x;
                *x = viejo * self.ganancia;
            }
            self.pos = (self.pos + 1) % l;
        }
        // Evita que la suma acumule error con las horas.
        if self.pos == 0 {
            self.suma_minimos = self.minimos.iter().map(|&m| m as f64).sum();
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn tono(amp: f32, cuadros: usize) -> Vec<f32> {
        (0..cuadros)
            .flat_map(|i| {
                let s = amp * (std::f32::consts::TAU * 440.0 * i as f32 / 48_000.0).sin();
                [s, s]
            })
            .collect()
    }

    #[test]
    fn sin_picos_es_transparente() {
        let mut lim = Limitador::nuevo(48_000, 2);
        let l = lim.retraso_cuadros();
        let entrada = tono(0.9, 48_000);
        let mut salida = entrada.clone();
        for b in salida.chunks_mut(2 * 300) {
            lim.procesar(b);
        }
        // Idéntica, con L cuadros de retraso.
        assert_eq!(&salida[2 * l..], &entrada[..entrada.len() - 2 * l]);
        assert_eq!(lim.cuadros_limitados, 0);
    }

    #[test]
    fn nunca_pasa_del_techo() {
        let mut lim = Limitador::nuevo(48_000, 2);
        // Tono a +6 dBFS, con saltos bruscos de nivel.
        let mut entrada = tono(0.5, 12_000);
        entrada.extend(tono(2.0, 12_000));
        entrada.extend(tono(0.3, 12_000));
        entrada.extend(tono(4.0, 300));
        // La vuelta es despacio (constante de 100 ms): 1 s alcanza de sobra.
        entrada.extend(tono(0.5, 48_000));
        let mut salida = entrada.clone();
        for b in salida.chunks_mut(2 * 137) {
            lim.procesar(b);
        }
        let maximo = salida.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(maximo <= TECHO + 1e-6, "máximo {maximo}");
        assert!(lim.cuadros_limitados > 0);
        // Después de limitar vuelve a 1 sin quedarse bajo.
        let fin = &salida[salida.len() - 2 * 1000..];
        let pico_final = fin.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(pico_final > 0.49, "no volvió a su nivel: {pico_final}");
    }

    #[test]
    fn la_bajada_es_suave() {
        // Un golpe brusco: la ganancia cambia como mucho 1/L por cuadro.
        let mut lim = Limitador::nuevo(48_000, 1);
        let l = lim.retraso_cuadros() as f32;
        let mut ganancias = Vec::new();
        for i in 0..10_000 {
            let mut x = [if i < 5_000 { 0.5f32 } else { 3.0 }];
            lim.procesar(&mut x);
            assert!(x[0].abs() <= TECHO + 1e-6, "cuadro {i}: {}", x[0]);
            ganancias.push(lim.ganancia());
        }
        let paso_max = ganancias.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
        assert!(paso_max <= 1.0 / l + 1e-4, "paso {paso_max} > 1/L");
        assert!(ganancias.last().unwrap() < &0.34);
    }
}
