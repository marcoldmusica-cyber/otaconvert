//! Botonera de sonidos (CLAUDE.md, sección 5b): disparo superpuesto, qué
//! pasa al tocar otra vez, teclas, pánico y edición validada.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use otaconvert_audio::Comando;
use otaconvert_servidor::{Destino, Modo, Sesion, Sonido};
use otaconvert_show::{guardar, AlRepetir, BotonSonido, Cue, PaginaBotonera, Show};

static N: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Default)]
struct Son {
    recibido: Arc<Mutex<Vec<(Destino, Comando)>>>,
}
impl Sonido for Son {
    fn frecuencia(&self) -> u32 {
        48_000
    }
    fn tiene_auriculares(&self) -> bool {
        false
    }
    fn enviar(&mut self, d: Destino, c: Comando) -> Result<(), String> {
        self.recibido.lock().unwrap().push((d, c));
        Ok(())
    }
    fn recoger_basura(&mut self) {}
}
impl Son {
    fn tomar(&self) -> Vec<Comando> {
        std::mem::take(&mut *self.recibido.lock().unwrap()).into_iter().map(|(_, c)| c).collect()
    }
}

fn boton(nombre: &str, archivo: &str, al_repetir: AlRepetir, tecla: Option<&str>) -> BotonSonido {
    BotonSonido { nombre: nombre.into(), archivo: archivo.into(), al_repetir, tecla: tecla.map(str::to_string), fundido_salida_ms: 500, ..Default::default() }
}

fn abrir() -> (PathBuf, Sesion, Son) {
    let d = std::env::temp_dir().join(format!("otaconvert-prueba-botonera-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let datos = Path::new(env!("CARGO_MANIFEST_DIR")).join("../audio/tests/datos");
    std::fs::copy(datos.join("tono.wav"), d.join("tono.wav")).unwrap();
    std::fs::copy(datos.join("tono.flac"), d.join("tono.flac")).unwrap();
    let show = Show {
        nombre: "Botonera".into(),
        cues: vec![Cue { numero: "1".into(), nombre: "Cue 1".into(), archivo: "tono.wav".into(), ..Default::default() }],
        botonera: vec![
            PaginaBotonera {
                nombre: "Acto 1".into(),
                botones: vec![
                    boton("Timbre", "tono.wav", AlRepetir::Parar, None),
                    boton("Risas", "tono.flac", AlRepetir::Superponer, None),
                    boton("Trueno", "tono.wav", AlRepetir::Reiniciar, Some("1")),
                ],
            },
            PaginaBotonera { nombre: "Improvisación".into(), botones: vec![boton("Perdido", "no-esta.wav", AlRepetir::Parar, None)] },
        ],
        ..Default::default()
    };
    guardar(&show, &d.join("b.otashow"), "20261021-000000").unwrap();
    let mut s = Sesion::abrir(&d).unwrap();
    let son = Son::default();
    s.conectar_sonido(Box::new(son.clone()));
    // Prepara los cues y la botonera como el hilo preparador.
    while let Some(t) = s.siguiente_a_preparar() {
        let r = t.hacer();
        s.resultado_preparar(t.cue, r);
    }
    while let Some(t) = s.siguiente_boton_a_preparar() {
        let r = t.hacer();
        s.resultado_boton(t.clave.clone(), r);
    }
    son.tomar();
    (d, s, son)
}

fn iniciados(c: &[Comando]) -> Vec<u32> {
    c.iter().filter_map(|c| if let Comando::IniciarCue { id, .. } = c { Some(*id) } else { None }).collect()
}

fn sonando(s: &Sesion, p: usize, b: usize) -> bool {
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    e["botonera"][p]["botones"][b]["sonando"].as_bool().unwrap()
}

#[test]
fn tocar_otra_vez_hace_lo_que_dice_el_boton() {
    let (_d, mut s, son) = abrir();
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert_eq!(e["botonera"][0]["botones"][0]["preparacion"], "listo");
    assert_eq!(e["botonera"][1]["botones"][0]["preparacion"], "error");
    let espera = s.en_espera;

    // Parar (por defecto): el segundo toque lo baja con su fundido de salida (500 ms).
    s.tocar_boton(0, 0).unwrap();
    let id = iniciados(&son.tomar())[0];
    assert!(sonando(&s, 0, 0));
    s.tocar_boton(0, 0).unwrap();
    let c = son.tomar();
    assert!(matches!(c.as_slice(), [Comando::Parar { id: x, fundido: 24_000 }] if *x == id), "{c:?}");
    assert!(!sonando(&s, 0, 0));

    // Superponer: dos copias a la vez.
    s.tocar_boton(0, 1).unwrap();
    s.tocar_boton(0, 1).unwrap();
    assert_eq!(iniciados(&son.tomar()).len(), 2);

    // Volver a empezar (por su tecla, también en Función): para el anterior y arranca otro.
    s.cambiar_modo(Modo::Funcion);
    assert!(s.tecla_boton("1"));
    let primero = iniciados(&son.tomar())[0];
    assert!(s.tecla_boton("1"));
    let c = son.tomar();
    assert!(matches!(c[0], Comando::Parar { id, fundido: 0 } if id == primero), "{c:?}");
    assert_eq!(iniciados(&c).len(), 1);
    assert!(!s.tecla_boton("9"), "una tecla sin botón no hace nada");

    // La botonera no mueve el cue en espera ni cuenta como GO.
    assert_eq!(s.en_espera, espera);
    assert!(s.go(), "el GO sigue funcionando enseguida (no cuenta como doble GO)");

    // Parar todo también para la botonera.
    s.parar_todo();
    assert!(!sonando(&s, 0, 1) && !sonando(&s, 0, 2));

    // Un botón cuyo archivo falta avisa y no suena.
    let err = s.tocar_boton(1, 0).unwrap_err();
    assert!(err.contiene("Falta el archivo"), "{err:?}");
    assert!(iniciados(&son.tomar()).len() <= 1);
}

#[test]
fn fundir_todo_funde_la_botonera_y_mayus_f_no() {
    let (_d, mut s, son) = abrir();
    s.tocar_boton(0, 0).unwrap();
    son.tomar();
    s.fundir_ultimo();
    assert!(son.tomar().is_empty(), "Mayús+F no toca la botonera");
    assert!(sonando(&s, 0, 0));
    s.fundir_todo();
    assert!(son.tomar().iter().any(|c| matches!(c, Comando::PararTodo { .. })));
    assert!(!sonando(&s, 0, 0), "queda fundiéndose");
}

#[test]
fn editar_la_botonera_se_valida_y_se_puede_deshacer() {
    let (_d, mut s, _son) = abrir();
    let base = serde_json::to_value(&s.show.botonera).unwrap();
    let con = |f: &dyn Fn(&mut serde_json::Value)| {
        let mut v = base.clone();
        f(&mut v);
        v
    };
    let malos = [
        con(&|v| v[0]["botones"][0]["tecla"] = "P".into()),
        con(&|v| v[0]["botones"][0]["tecla"] = "1".into()),
        con(&|v| v[0]["botones"][0]["archivo"] = "../fuera.wav".into()),
        con(&|v| v[0]["botones"][0]["archivo"] = "foto.png".into()),
        con(&|v| v[0]["botones"][0]["nombre"] = "".into()),
        con(&|v| v[0]["botones"][0]["color"] = "rojo".into()),
        con(&|v| v[0]["botones"][0]["volumen_db"] = 40.0.into()),
        con(&|v| v[0]["botones"][0]["salida"] = "Sala".into()),
        serde_json::json!("no es una lista"),
    ];
    for m in &malos {
        assert!(s.editar_botonera(m).is_err(), "{m}");
    }
    assert_eq!(serde_json::to_value(&s.show.botonera).unwrap(), base, "nada cambió");

    let bien = con(&|v| {
        v[0]["botones"][0]["nombre"] = "Timbre de sala".into();
        v[0]["botones"][0]["tecla"] = "F2".into();
    });
    s.cambiar_modo(Modo::Funcion);
    assert!(s.editar_botonera(&bien).is_err(), "en Función no se edita");
    s.cambiar_modo(Modo::Edicion);
    s.editar_botonera(&bien).unwrap();
    assert_eq!(s.show.botonera[0].botones[0].nombre, "Timbre de sala");
    assert!(s.tecla_boton("f2"), "la tecla nueva funciona (sin importar mayúsculas)");
    s.deshacer().unwrap();
    assert_eq!(s.show.botonera[0].botones[0].nombre, "Timbre");
}

#[test]
fn teclas_validas() {
    use otaconvert_servidor::tecla_valida;
    for k in ["1", "a", "Z", "F1", "f12"] {
        assert!(tecla_valida(k), "{k}");
    }
    for k in ["P", "f", "H", "F13", "F0", "AB", "", " ", "Espacio"] {
        assert!(!tecla_valida(k), "{k}");
    }
}
