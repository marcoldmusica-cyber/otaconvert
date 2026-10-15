//! Mensajes emergentes de cada rampa corta: aparecen al pasar el ratón o al
//! tocar el "?" junto a la opción, y recomiendan cuándo usarla con un ejemplo
//! de escena. Textos cortos, pensados para quien nunca usó un programa de cues.
//!
//! - Se muestran en el idioma del programa: todo en español o todo en inglés.
//! - Se pueden ocultar (usuarios avanzados) y siempre se pueden volver a
//!   mostrar: ver `PreferenciasConsejos`.

use crate::{ms, Ajustes};

/// Idioma de la interfaz. Los mensajes siguen siempre al del programa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Idioma {
    #[default]
    Es,
    En,
}

/// Si el usuario quiere ver los mensajes emergentes. Activados por defecto.
/// La opción para volver a mostrarlos nunca se oculta (Vista y Ayuda).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreferenciasConsejos {
    pub mostrar: bool,
}

impl Default for PreferenciasConsejos {
    fn default() -> Self {
        Self { mostrar: true }
    }
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

    /// El mensaje en el idioma del programa, siempre.
    pub fn consejo(self, idioma: Idioma) -> Consejo {
        match idioma {
            Idioma::Es => es(self),
            Idioma::En => en(self),
        }
    }

    /// El mensaje solo si el usuario quiere verlos.
    pub fn consejo_visible(self, idioma: Idioma, prefs: PreferenciasConsejos) -> Option<Consejo> {
        prefs.mostrar.then(|| self.consejo(idioma))
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
    /// Ejemplo de escena donde conviene activarla.
    pub ejemplo_activar: &'static str,
    /// Cuándo conviene dejarla en 0 (de golpe).
    pub desactivar: &'static str,
    /// Ejemplo de escena donde conviene dejarla en 0.
    pub ejemplo_desactivar: &'static str,
}

impl Consejo {
    pub fn textos(&self) -> [&'static str; 6] {
        [
            self.titulo,
            self.que_hace,
            self.activar,
            self.ejemplo_activar,
            self.desactivar,
            self.ejemplo_desactivar,
        ]
    }
}

fn es(t: TipoRampa) -> Consejo {
    match t {
        TipoRampa::Inicio => Consejo {
            titulo: "Rampa al iniciar",
            que_hace: "Sube el volumen en unos milisegundos al disparar el cue, en vez de arrancar de golpe.",
            activar: "Actívala (5 ms) si el cue empieza a mitad del archivo o se oye un clic al dispararlo. \
                      Con 2–5 ms no se nota.",
            ejemplo_activar: "Ej.: al abrir el telón la música entra desde el estribillo, a mitad de la canción: \
                              con la rampa entra limpia, sin clic.",
            desactivar: "Déjala en 0 si el sonido debe empezar con toda su fuerza. \
                         Es seguro si el archivo empieza en silencio.",
            ejemplo_desactivar: "Ej.: el disparo de la escena final o el portazo del segundo acto: \
                                 en 0 el golpe suena exacto y sincronizado con la acción.",
        },
        TipoRampa::Reanudar => Consejo {
            titulo: "Rampa al reanudar",
            que_hace: "Sube el volumen al quitar la pausa.",
            activar: "Recomendada (5 ms): el audio sigue desde donde quedó, casi siempre a mitad de onda.",
            ejemplo_activar: "Ej.: pausas la lluvia de fondo mientras habla un actor y la retomas al terminar: \
                              vuelve sin clic.",
            desactivar: "En 0 solo para un corte seco a propósito; puede sonar un clic.",
            ejemplo_desactivar: "Ej.: una radio en escena que se \"enciende\" de golpe cuando el actor la toca.",
        },
        TipoRampa::Pausa => Consejo {
            titulo: "Rampa al pausar",
            que_hace: "Baja el volumen antes de detener el audio en pausa.",
            activar: "Recomendada (10 ms): evita el clic de cortar a mitad de onda. \
                      Con 20–50 ms la pausa se siente más suave.",
            ejemplo_activar: "Ej.: congelas la música de una coreografía cuando los bailarines se quedan quietos: \
                              con 30 ms se detiene sin chasquido.",
            desactivar: "En 0 para un corte seco a propósito; puede sonar un clic.",
            ejemplo_desactivar: "Ej.: un efecto de \"tiempo detenido\" donde el sonido debe cortarse en seco.",
        },
        TipoRampa::Parada => Consejo {
            titulo: "Rampa al parar",
            que_hace: "Baja el volumen antes de parar un cue (también con Parar todo).",
            activar: "Recomendada (10 ms): Parar todo queda limpio aunque suenen muchos audios. \
                      Para salidas lentas, usa el fundido del cue.",
            ejemplo_activar: "Ej.: un actor se adelanta y tienes que cortar ambiente, música y efectos a la vez: \
                              Parar todo suena limpio.",
            desactivar: "En 0 para cortes secos a propósito; puede sonar un clic.",
            ejemplo_desactivar: "Ej.: apagón total al final de la escena: el sonido se corta en el mismo instante \
                                 que la luz.",
        },
        TipoRampa::Salto => Consejo {
            titulo: "Rampa al saltar",
            que_hace: "Baja y vuelve a subir el volumen al mover el audio a otro punto (ensayos, ir a cue).",
            activar: "Recomendada (5 ms): un salto casi siempre cae a mitad de onda.",
            ejemplo_activar: "Ej.: en el ensayo el director pide repetir desde el minuto 2 de la música: \
                              saltas sin clic en los altavoces.",
            desactivar: "En 0 casi nunca conviene: el salto sonaría con un clic.",
            ejemplo_desactivar: "Ej.: solo en pruebas técnicas, para escuchar el salto tal cual.",
        },
        TipoRampa::Volumen => Consejo {
            titulo: "Rampa de volumen",
            que_hace: "Tiempo mínimo de cualquier cambio de volumen, del cue o del maestro.",
            activar: "Recomendada (5 ms): cambiar el volumen de golpe produce un chasquido. \
                      Los fundidos largos van en el cue.",
            ejemplo_activar: "Ej.: bajas el ambiente de la fiesta cuando empieza un diálogo: \
                              el cambio es inmediato pero sin chasquido.",
            desactivar: "En 0 solo para pruebas técnicas.",
            ejemplo_desactivar: "Ej.: medir la tarjeta de sonido en la Prueba de hardware.",
        },
        TipoRampa::Fin => Consejo {
            titulo: "Rampa al final del archivo",
            que_hace: "Baja el volumen en los últimos milisegundos de un audio que no está en bucle.",
            activar: "Recomendada (10 ms) si el archivo termina de golpe o a mitad de sonido: \
                      quita el clic final.",
            ejemplo_activar: "Ej.: un efecto de motor recortado a mano que termina en pleno ruido: \
                              la rampa evita el clic al acabar.",
            desactivar: "En 0 si el archivo ya termina en silencio y debe sonar tal cual se editó.",
            ejemplo_desactivar: "Ej.: una canción con su final completo y desvanecido: no hace falta tocarla.",
        },
    }
}

fn en(t: TipoRampa) -> Consejo {
    match t {
        TipoRampa::Inicio => Consejo {
            titulo: "Start ramp",
            que_hace: "Raises the volume over a few milliseconds when the cue fires, instead of starting abruptly.",
            activar: "Turn it on (5 ms) if the cue starts mid-file or you hear a click when it fires. \
                      2–5 ms is inaudible.",
            ejemplo_activar: "E.g.: as the curtain opens, the music comes in at the chorus, mid-song: \
                              the ramp brings it in cleanly, no click.",
            desactivar: "Leave it at 0 if the sound must hit at full strength. Safe if the file starts in silence.",
            ejemplo_desactivar: "E.g.: the gunshot in the final scene or the door slam in act two: \
                                 at 0 the hit is exact and in sync with the action.",
        },
        TipoRampa::Reanudar => Consejo {
            titulo: "Resume ramp",
            que_hace: "Raises the volume when un-pausing.",
            activar: "Recommended (5 ms): audio resumes where it stopped, almost always mid-wave.",
            ejemplo_activar: "E.g.: you pause the background rain during a speech and bring it back after: \
                              it returns without a click.",
            desactivar: "0 only for a deliberate hard cut; it may click.",
            ejemplo_desactivar: "E.g.: an on-stage radio that \"switches on\" instantly when the actor touches it.",
        },
        TipoRampa::Pausa => Consejo {
            titulo: "Pause ramp",
            que_hace: "Lowers the volume before pausing.",
            activar: "Recommended (10 ms): avoids the click of cutting mid-wave. \
                      20–50 ms makes the pause feel softer.",
            ejemplo_activar: "E.g.: you freeze the choreography music when the dancers stand still: \
                              with 30 ms it stops without a crackle.",
            desactivar: "0 for a deliberate hard cut; it may click.",
            ejemplo_desactivar: "E.g.: a \"time stops\" effect where the sound must cut dead.",
        },
        TipoRampa::Parada => Consejo {
            titulo: "Stop ramp",
            que_hace: "Lowers the volume before stopping a cue (also with Stop all).",
            activar: "Recommended (10 ms): Stop all stays clean even with many sounds playing. \
                      For slow exits, use the cue's fade.",
            ejemplo_activar: "E.g.: an actor skips ahead and you must cut ambience, music and effects at once: \
                              Stop all sounds clean.",
            desactivar: "0 for deliberate hard cuts; it may click.",
            ejemplo_desactivar: "E.g.: a full blackout at the end of the scene: sound cuts at the same instant \
                                 as the lights.",
        },
        TipoRampa::Salto => Consejo {
            titulo: "Jump ramp",
            que_hace: "Dips and restores the volume when moving audio to another point (rehearsals, go to cue).",
            activar: "Recommended (5 ms): a jump almost always lands mid-wave.",
            ejemplo_activar: "E.g.: in rehearsal the director asks to restart the music from minute 2: \
                              you jump with no click in the speakers.",
            desactivar: "0 is rarely a good idea: the jump would click.",
            ejemplo_desactivar: "E.g.: only in technical tests, to hear the jump as is.",
        },
        TipoRampa::Volumen => Consejo {
            titulo: "Volume ramp",
            que_hace: "Minimum duration of any volume change, cue or master.",
            activar: "Recommended (5 ms): changing volume instantly causes a crackle. Long fades belong in the cue.",
            ejemplo_activar: "E.g.: you duck the party ambience when a dialogue starts: \
                              the change is instant but crackle-free.",
            desactivar: "0 only for technical tests.",
            ejemplo_desactivar: "E.g.: measuring the sound card in the Hardware test.",
        },
        TipoRampa::Fin => Consejo {
            titulo: "End-of-file ramp",
            que_hace: "Lowers the volume over the last milliseconds of audio that isn't looping.",
            activar: "Recommended (10 ms) if the file ends abruptly or mid-sound: removes the final click.",
            ejemplo_activar: "E.g.: an engine effect trimmed by hand that ends mid-noise: \
                              the ramp avoids the click at the end.",
            desactivar: "0 if the file already ends in silence and must sound exactly as edited.",
            ejemplo_desactivar: "E.g.: a song with its full, faded-out ending: no need to touch it.",
        },
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn toda_rampa_tiene_consejo_completo_en_ambos_idiomas() {
        for t in TipoRampa::TODAS {
            for i in [Idioma::Es, Idioma::En] {
                let c = t.consejo(i);
                for texto in c.textos() {
                    assert!(!texto.trim().is_empty(), "{t:?} {i:?}");
                    // Mensajes sutiles: cortos para una burbuja emergente.
                    assert!(texto.chars().count() <= 200, "{t:?} {i:?} demasiado largo: {texto}");
                }
                let prefijo = if i == Idioma::Es { "Ej.: " } else { "E.g.: " };
                assert!(c.ejemplo_activar.starts_with(prefijo), "{t:?} {i:?}");
                assert!(c.ejemplo_desactivar.starts_with(prefijo), "{t:?} {i:?}");
            }
        }
    }

    #[test]
    fn cada_idioma_tiene_sus_propios_textos() {
        for t in TipoRampa::TODAS {
            let (es, en) = (t.consejo(Idioma::Es), t.consejo(Idioma::En));
            for (a, b) in es.textos().iter().zip(en.textos()) {
                assert_ne!(*a, b, "{t:?}: texto sin traducir");
            }
        }
    }

    #[test]
    fn se_pueden_ocultar_y_volver_a_mostrar() {
        let mut prefs = PreferenciasConsejos::default();
        assert!(prefs.mostrar, "deben verse por defecto");
        prefs.mostrar = false;
        assert!(TipoRampa::Inicio.consejo_visible(Idioma::Es, prefs).is_none());
        prefs.mostrar = true;
        assert!(TipoRampa::Inicio.consejo_visible(Idioma::En, prefs).is_some());
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
