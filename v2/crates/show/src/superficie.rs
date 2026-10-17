//! Encuadre de cada vídeo o imagen y superficie de 4 esquinas del Proyector
//! (etapa 4C). Solo datos y cuentas: dibujar lo hace la proyección.

use serde::{Deserialize, Serialize};

/// Cómo entra la imagen en la pantalla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModoEncuadre {
    /// Entera, con bandas negras si no tiene la forma de la pantalla.
    #[default]
    Ajustar,
    /// Llena la pantalla recortando lo que sobra.
    Llenar,
    /// Llena la pantalla deformándola.
    Estirar,
}

/// Encuadre de un cue de vídeo o imagen. Valores neutros por defecto.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Encuadre {
    pub modo: ModoEncuadre,
    /// 1,0 = tamaño normal; 2,0 = el doble; 0,5 = la mitad.
    pub zoom: f32,
    /// Mover, en fracciones del tamaño de la imagen (0,5 = media imagen a la derecha / abajo).
    pub mover_x: f32,
    pub mover_y: f32,
    /// Recortes de cada borde, en fracciones de la imagen (0,1 = el 10 %).
    pub recorte_arriba: f32,
    pub recorte_abajo: f32,
    pub recorte_izquierda: f32,
    pub recorte_derecha: f32,
}

impl Default for Encuadre {
    fn default() -> Self {
        Self {
            modo: ModoEncuadre::Ajustar,
            zoom: 1.0,
            mover_x: 0.0,
            mover_y: 0.0,
            recorte_arriba: 0.0,
            recorte_abajo: 0.0,
            recorte_izquierda: 0.0,
            recorte_derecha: 0.0,
        }
    }
}

pub const ZOOM_MIN: f32 = 0.1;
pub const ZOOM_MAX: f32 = 10.0;
pub const MOVER_MAX: f32 = 2.0;
/// Cada borde se puede recortar hasta el 45 %; entre los dos de un lado, hasta el 90 %.
pub const RECORTE_MAX: f32 = 0.45;

impl Encuadre {
    pub fn es_normal(&self) -> bool {
        *self == Self::default()
    }

    /// Qué está mal, o nada.
    pub fn problema(&self) -> Option<String> {
        let todos = [
            self.zoom,
            self.mover_x,
            self.mover_y,
            self.recorte_arriba,
            self.recorte_abajo,
            self.recorte_izquierda,
            self.recorte_derecha,
        ];
        if todos.iter().any(|v| !v.is_finite()) {
            return Some("encuadre con un número no válido".into());
        }
        if !(ZOOM_MIN..=ZOOM_MAX).contains(&self.zoom) {
            return Some(format!("zoom {} (de {ZOOM_MIN} a {ZOOM_MAX})", self.zoom));
        }
        if self.mover_x.abs() > MOVER_MAX || self.mover_y.abs() > MOVER_MAX {
            return Some(format!("mover {} / {} (como mucho {MOVER_MAX})", self.mover_x, self.mover_y));
        }
        for v in &todos[3..] {
            if !(0.0..=RECORTE_MAX).contains(v) {
                return Some(format!("recorte {v} (de 0 a {RECORTE_MAX})"));
            }
        }
        None
    }
}

/// Superficie del Proyector: dónde caen las 4 esquinas de la imagen, en
/// fracciones de la pantalla (0,0 arriba a la izquierda; 1,1 abajo a la
/// derecha). Orden: arriba-izquierda, arriba-derecha, abajo-derecha, abajo-izquierda.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Superficie {
    pub esquinas: [[f64; 2]; 4],
}

pub const ESQUINAS_NORMALES: [[f64; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
/// Las esquinas pueden salir un poco de la pantalla (para agrandar la imagen).
pub const ESQUINA_MIN: f64 = -1.0;
pub const ESQUINA_MAX: f64 = 2.0;

impl Default for Superficie {
    fn default() -> Self {
        Self { esquinas: ESQUINAS_NORMALES }
    }
}

impl Superficie {
    pub fn es_normal(&self) -> bool {
        self.esquinas == ESQUINAS_NORMALES
    }

    pub fn problema(&self) -> Option<String> {
        for [x, y] in self.esquinas {
            if !x.is_finite() || !y.is_finite() || !(ESQUINA_MIN..=ESQUINA_MAX).contains(&x) || !(ESQUINA_MIN..=ESQUINA_MAX).contains(&y) {
                return Some(format!("esquina fuera de lugar ({x}, {y})"));
            }
        }
        if !self.es_convexa() || self.homografia_inversa().is_none() {
            return Some("las 4 esquinas no forman una figura válida (se cruzan o quedan aplastadas)".into());
        }
        None
    }

    /// Las 4 esquinas, en orden, forman una figura sin cruces (todas las vueltas hacia el mismo lado).
    pub fn es_convexa(&self) -> bool {
        let e = self.esquinas;
        let mut signo = 0.0f64;
        for k in 0..4 {
            let [ax, ay] = e[k];
            let [bx, by] = e[(k + 1) % 4];
            let [cx, cy] = e[(k + 2) % 4];
            let cruz = (bx - ax) * (cy - by) - (by - ay) * (cx - bx);
            if cruz.abs() < 1e-9 {
                return false;
            }
            if signo == 0.0 {
                signo = cruz.signum();
            } else if cruz.signum() != signo {
                return false;
            }
        }
        true
    }

    /// Los 8 números que usa el shader: de un punto de la salida (0–1) al
    /// punto de la imagen (0–1). Es la inversa de la homografía que lleva el
    /// cuadrado de la imagen a las 4 esquinas.
    pub fn homografia_inversa(&self) -> Option<[f64; 8]> {
        let [[x0, y0], [x1, y1], [x2, y2], [x3, y3]] = self.esquinas;
        let (dx1, dx2, dx3) = (x1 - x2, x3 - x2, x0 - x1 + x2 - x3);
        let (dy1, dy2, dy3) = (y1 - y2, y3 - y2, y0 - y1 + y2 - y3);
        let (g, h) = if dx3.abs() < 1e-12 && dy3.abs() < 1e-12 {
            (0.0, 0.0)
        } else {
            let den = dx1 * dy2 - dx2 * dy1;
            if den.abs() < 1e-12 {
                return None;
            }
            ((dx3 * dy2 - dx2 * dy3) / den, (dx1 * dy3 - dx3 * dy1) / den)
        };
        let m = [[x1 - x0 + g * x1, x3 - x0 + h * x3, x0], [y1 - y0 + g * y1, y3 - y0 + h * y3, y0], [g, h, 1.0]];
        let c = |r: usize, k: usize| m[r][k];
        let det = c(0, 0) * (c(1, 1) * c(2, 2) - c(1, 2) * c(2, 1)) - c(0, 1) * (c(1, 0) * c(2, 2) - c(1, 2) * c(2, 0))
            + c(0, 2) * (c(1, 0) * c(2, 1) - c(1, 1) * c(2, 0));
        if det.abs() < 1e-12 {
            return None;
        }
        let inv = [
            [c(1, 1) * c(2, 2) - c(1, 2) * c(2, 1), c(0, 2) * c(2, 1) - c(0, 1) * c(2, 2), c(0, 1) * c(1, 2) - c(0, 2) * c(1, 1)],
            [c(1, 2) * c(2, 0) - c(1, 0) * c(2, 2), c(0, 0) * c(2, 2) - c(0, 2) * c(2, 0), c(0, 2) * c(1, 0) - c(0, 0) * c(1, 2)],
            [c(1, 0) * c(2, 1) - c(1, 1) * c(2, 0), c(0, 1) * c(2, 0) - c(0, 0) * c(2, 1), c(0, 0) * c(1, 1) - c(0, 1) * c(1, 0)],
        ];
        let n = inv[2][2];
        if n.abs() < 1e-12 {
            return None;
        }
        let r = [inv[0][0] / n, inv[0][1] / n, inv[0][2] / n, inv[1][0] / n, inv[1][1] / n, inv[1][2] / n, inv[2][0] / n, inv[2][1] / n];
        r.iter().all(|v| v.is_finite()).then_some(r)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_homografia_lleva_cada_esquina_a_su_lugar() {
        let s = Superficie { esquinas: [[0.12, 0.08], [0.80, 0.18], [0.92, 0.88], [0.20, 0.95]] };
        let h = s.homografia_inversa().unwrap();
        let ir = |x: f64, y: f64| {
            let z = h[6] * x + h[7] * y + 1.0;
            ((h[0] * x + h[1] * y + h[2]) / z, (h[3] * x + h[4] * y + h[5]) / z)
        };
        for (k, (u, v)) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].into_iter().enumerate() {
            let (a, b) = ir(s.esquinas[k][0], s.esquinas[k][1]);
            assert!((a - u).abs() < 1e-9 && (b - v).abs() < 1e-9, "esquina {k}");
        }
        assert_eq!(Superficie::default().homografia_inversa().unwrap(), [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]);
        assert!(s.problema().is_none());
    }

    #[test]
    fn figuras_imposibles_se_rechazan() {
        for esquinas in [
            [[0.0, 0.0], [1.0, 1.0], [1.0, 0.0], [0.0, 1.0]], // se cruzan
            [[0.0, 0.0], [0.5, 0.0], [1.0, 0.0], [0.2, 0.0]], // aplastada
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [f64::NAN, 1.0]],
            [[0.0, 0.0], [5.0, 0.0], [1.0, 1.0], [0.0, 1.0]], // muy afuera
        ] {
            assert!(Superficie { esquinas }.problema().is_some(), "{esquinas:?}");
        }
    }

    #[test]
    fn encuadres_imposibles_se_rechazan() {
        assert!(Encuadre::default().problema().is_none());
        for e in [
            Encuadre { zoom: 0.0, ..Default::default() },
            Encuadre { zoom: f32::INFINITY, ..Default::default() },
            Encuadre { mover_x: 3.0, ..Default::default() },
            Encuadre { recorte_izquierda: 0.6, ..Default::default() },
            Encuadre { recorte_abajo: -0.1, ..Default::default() },
        ] {
            assert!(e.problema().is_some(), "{e:?}");
        }
    }
}
