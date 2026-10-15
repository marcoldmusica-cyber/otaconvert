//! Mensajes emergentes de cada rampa corta: aparecen al pasar el ratón o al
//! tocar el "?" junto a la opción, y recomiendan cuándo usarla. Textos cortos,
//! en español e inglés, pensados para quien nunca usó un programa de cues.

use crate::{ms, Ajustes};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Idioma {
    Es,
    En,
}

/// Cada una de las rampas configurables de `Ajustes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoRampa {
    Inicio,
    Reanudar,
    Pausa,
    Parada,
    Salto,
    Volumen,
    Fin,
}

impl TipoRampa {
    pub const TODAS: [TipoRampa; 7] = [
        TipoRampa::Inicio,
        TipoRampa::Reanudar,
        TipoRampa::Pausa,
        TipoRampa::Parada,
        TipoRampa::Salto,
        TipoRampa::Volumen,
        TipoRampa::Fin,
    ];

    /// Valor actual en `ajustes`, en cuadros.
    pub fn en(self, a: &Ajustes) -> u32 {
        match self {
            TipoRampa::Inicio => a.rampa_inicio,
            TipoRampa::Reanudar => a.rampa_reanudar,
            TipoRampa::Pausa => a.rampa_pausa,
            TipoRampa::Parada => a.rampa_parada,
            TipoRampa::Salto => a.rampa_salto,
            TipoRampa::Volumen => a.rampa_volumen,
            TipoRampa::Fin => a.rampa_fin,
        }
    }

    /// Milisegundos recomendados cuando la rampa está activada.
    pub fn ms_recomendados(self) -> f32 {
        match self {
            TipoRampa::Inicio | TipoRampa::Reanudar | TipoRampa::Salto | TipoRampa::Volumen => 5.0,
            TipoRampa::Pausa | TipoRampa::Parada | TipoRampa::Fin => 10.0,
        }
    }

    pub fn cuadros_recomendados(self, frecuencia: u32) -> u32 {
        ms(frecuencia, self.ms_recomendados())
    }

    pub fn consejo(self, idioma: Idioma) -> Consejo {
        match idioma {
            Idioma::Es => es(self),
            Idioma::En => en(self),
        }
    }
}

/// Lo que muestra el mensaje emergente.
#[derive(Debug, Clone, Copy)]
pub struct Consejo {
    /// Nombre de la opción.
    pub titulo: &'static str,
    /// Qué hace, en una frase.
    pub que_hace: &'static str,
    /// Cuándo conviene activarla (o subir los ms).
    pub activar: &'static str,
    /// Cuándo conviene dejarla en 0 (de golpe).
    pub desactivar: &'static str,
}

fn es(t: TipoRampa) -> Consejo {
    match t {
        TipoRampa::Inicio => Consejo {
            titulo: "Rampa al iniciar",
            que_hace: "Sube el volumen en unos pocos milisegundos al disparar el cue, en vez de arrancar de golpe.",
            activar: "Actívala (5 ms) si el cue empieza a mitad del archivo o se oye un clic al dispararlo. \
                      Con 2–5 ms no se nota y quita el clic.",
            desactivar: "Déjala en 0 para golpes, disparos, portazos o música que empieza fuerte: \
                         el ataque suena exacto. Seguro si el archivo empieza en silencio.",
        },
        TipoRampa::Reanudar => Consejo {
            titulo: "Rampa al reanudar",
            que_hace: "Sube el volumen al quitar la pausa.",
            activar: "Recomendada (5 ms): el audio sigue desde donde quedó, casi siempre a mitad de onda, \
                      y sin rampa suena un clic.",
            desactivar: "En 0 solo si buscas un efecto de corte seco, y sabiendo que puede sonar un clic.",
        },
        TipoRampa::Pausa => Consejo {
            titulo: "Rampa al pausar",
            que_hace: "Baja el volumen antes de detener el audio en pausa.",
            activar: "Recomendada (10 ms): evita el clic de cortar a mitad de onda. \
                      Súbela a 20–50 ms si quieres que la pausa se sienta más suave.",
            desactivar: "En 0 para un corte seco a propósito. Puede sonar un clic.",
        },
        TipoRampa::Parada => Consejo {
            titulo: "Rampa al parar",
            que_hace: "Baja el volumen antes de parar un cue (también con Parar todo).",
            activar: "Recomendada (10 ms): Parar todo queda limpio aunque suenen muchos audios. \
                      Para una salida más lenta usa el fundido de salida del cue.",
            desactivar: "En 0 para cortes secos sincronizados con un apagón de luces. Puede sonar un clic.",
        },
        TipoRampa::Salto => Consejo {
            titulo: "Rampa al saltar",
            que_hace: "Baja y vuelve a subir el volumen al mover el audio a otro punto (ensayos, ir a cue).",
            activar: "Recomendada (5 ms): un salto casi siempre cae a mitad de onda.",
            desactivar: "En 0 casi nunca conviene: el salto sonaría con un clic.",
        },
        TipoRampa::Volumen => Consejo {
            titulo: "Rampa de volumen",
            que_hace: "Tiempo mínimo de cualquier cambio de volumen, del cue o del maestro.",
            activar: "Recomendada (5 ms): mover el volumen de golpe produce un chasquido. \
                      Los fundidos largos se ponen en el cue, no aquí.",
            desactivar: "En 0 solo para pruebas técnicas.",
        },
        TipoRampa::Fin => Consejo {
            titulo: "Rampa al final del archivo",
            que_hace: "Baja el volumen en los últimos milisegundos de un audio que no está en bucle.",
            activar: "Recomendada (10 ms) si el archivo termina de golpe o a mitad de sonido: quita el clic final.",
            desactivar: "En 0 si el archivo ya termina en silencio y quieres que suene exactamente como se editó.",
        },
    }
}

fn en(t: TipoRampa) -> Consejo {
    match t {
        TipoRampa::Inicio => Consejo {
            titulo: "Start ramp",
            que_hace: "Raises the volume over a few milliseconds when the cue fires, instead of starting abruptly.",
            activar: "Turn it on (5 ms) if the cue starts mid-file or you hear a click when it fires. \
                      2–5 ms is inaudible and removes the click.",
            desactivar: "Leave it at 0 for hits, gunshots, door slams or music that starts loud: \
                         the attack stays exact. Safe if the file starts in silence.",
        },
        TipoRampa::Reanudar => Consejo {
            titulo: "Resume ramp",
            que_hace: "Raises the volume when un-pausing.",
            activar: "Recommended (5 ms): audio resumes where it stopped, almost always mid-wave, \
                      and clicks without a ramp.",
            desactivar: "0 only for a deliberate hard cut, knowing it may click.",
        },
        TipoRampa::Pausa => Consejo {
            titulo: "Pause ramp",
            que_hace: "Lowers the volume before pausing.",
            activar: "Recommended (10 ms): avoids the click of cutting mid-wave. \
                      Raise it to 20–50 ms for a softer-feeling pause.",
            desactivar: "0 for a deliberate hard cut. It may click.",
        },
        TipoRampa::Parada => Consejo {
            titulo: "Stop ramp",
            que_hace: "Lowers the volume before stopping a cue (also with Stop all).",
            activar: "Recommended (10 ms): Stop all stays clean even with many sounds playing. \
                      For a slower exit use the cue's fade out.",
            desactivar: "0 for hard cuts synced with a blackout. It may click.",
        },
        TipoRampa::Salto => Consejo {
            titulo: "Jump ramp",
            que_hace: "Dips and restores the volume when moving audio to another point (rehearsals, go to cue).",
            activar: "Recommended (5 ms): a jump almost always lands mid-wave.",
            desactivar: "0 is rarely a good idea: the jump would click.",
        },
        TipoRampa::Volumen => Consejo {
            titulo: "Volume ramp",
            que_hace: "Minimum duration of any volume change, cue or master.",
            activar: "Recommended (5 ms): changing volume instantly causes a crackle. \
                      Long fades belong in the cue, not here.",
            desactivar: "0 only for technical tests.",
        },
        TipoRampa::Fin => Consejo {
            titulo: "End-of-file ramp",
            que_hace: "Lowers the volume over the last milliseconds of audio that isn't looping.",
            activar: "Recommended (10 ms) if the file ends abruptly or mid-sound: removes the final click.",
            desactivar: "0 if the file already ends in silence and you want it exactly as edited.",
        },
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn toda_rampa_tiene_consejo_en_ambos_idiomas() {
        for t in TipoRampa::TODAS {
            for i in [Idioma::Es, Idioma::En] {
                let c = t.consejo(i);
                for texto in [c.titulo, c.que_hace, c.activar, c.desactivar] {
                    assert!(!texto.trim().is_empty(), "{t:?} {i:?}");
                    // Mensajes sutiles: cortos para una burbuja emergente.
                    assert!(texto.chars().count() <= 200, "{t:?} {i:?} demasiado largo: {texto}");
                }
            }
        }
    }

    #[test]
    fn recomendados_coinciden_con_suaves() {
        let fs = 48_000;
        let a = Ajustes::suaves(fs);
        for t in TipoRampa::TODAS {
            assert_eq!(t.en(&a), t.cuadros_recomendados(fs), "{t:?}");
        }
        // Por defecto solo el inicio arranca de golpe.
        let d = Ajustes::para(fs);
        for t in TipoRampa::TODAS {
            let esperado = if t == TipoRampa::Inicio { 0 } else { t.cuadros_recomendados(fs) };
            assert_eq!(t.en(&d), esperado, "{t:?}");
        }
    }
}
