/// Cambio lineal de un valor (ganancia) a lo largo de un número de cuadros.
#[derive(Debug, Clone, Copy)]
pub struct Rampa {
    actual: f32,
    objetivo: f32,
    paso: f32,
    restantes: u32,
}

impl Rampa {
    pub fn nueva(valor: f32) -> Self {
        Self { actual: valor, objetivo: valor, paso: 0.0, restantes: 0 }
    }

    /// Empieza a moverse desde el valor actual hasta `objetivo` en `cuadros`.
    /// Con 0 cuadros el cambio es inmediato.
    pub fn ir_a(&mut self, objetivo: f32, cuadros: u32) {
        self.objetivo = objetivo;
        if cuadros == 0 {
            self.actual = objetivo;
            self.restantes = 0;
        } else {
            self.paso = (objetivo - self.actual) / cuadros as f32;
            self.restantes = cuadros;
        }
    }

    /// Devuelve el valor de este cuadro y avanza al siguiente.
    #[inline]
    pub fn siguiente(&mut self) -> f32 {
        let v = self.actual;
        if self.restantes > 0 {
            self.restantes -= 1;
            self.actual = if self.restantes == 0 { self.objetivo } else { self.actual + self.paso };
        }
        v
    }

    pub fn valor(&self) -> f32 {
        self.actual
    }

    pub fn en_reposo(&self) -> bool {
        self.restantes == 0
    }
}

#[cfg(test)]
mod pruebas {
    use super::Rampa;

    #[test]
    fn llega_exacto_al_objetivo() {
        let mut r = Rampa::nueva(0.0);
        r.ir_a(1.0, 240);
        let mut ultimo = 0.0;
        for _ in 0..240 {
            let v = r.siguiente();
            assert!(v >= ultimo, "la rampa debe subir sin saltos atrás");
            ultimo = v;
        }
        assert!(r.en_reposo());
        assert_eq!(r.valor(), 1.0);
    }

    #[test]
    fn cero_cuadros_es_inmediato() {
        let mut r = Rampa::nueva(1.0);
        r.ir_a(0.0, 0);
        assert_eq!(r.siguiente(), 0.0);
        assert!(r.en_reposo());
    }
}
