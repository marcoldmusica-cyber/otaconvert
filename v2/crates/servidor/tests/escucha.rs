//! «Ver antes» (etapa 5C): un cue suena solo por Auriculares, sin tocar la
//! sala, el cue en espera ni el GO.

use std::path::Path;
use std::sync::{Arc, Mutex};

use otaconvert_audio::Comando;
use otaconvert_servidor::{Destino, Modo, Sesion, Sonido};
use otaconvert_show::{guardar, Cue, Show, TipoCue};

#[derive(Clone)]
struct Son {
    auriculares: bool,
    recibido: Arc<Mutex<Vec<(Destino, Comando)>>>,
}
impl Sonido for Son {
    fn frecuencia(&self) -> u32 {
        48_000
    }
    fn tiene_auriculares(&self) -> bool {
        self.auriculares
    }
    fn enviar(&mut self, d: Destino, c: Comando) -> Result<(), String> {
        self.recibido.lock().unwrap().push((d, c));
        Ok(())
    }
    fn recoger_basura(&mut self) {}
}

fn abrir(auriculares: bool) -> (Sesion, Son) {
    let d = std::env::temp_dir().join(format!("otaconvert-prueba-escucha-{}-{auriculares}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../audio/tests/datos/tono.wav"), d.join("a.wav")).unwrap();
    let cue = |n: &str, tipo| Cue { numero: n.into(), nombre: format!("Cue {n}"), archivo: "a.wav".into(), tipo, ..Default::default() };
    let show = Show { nombre: "Escucha".into(), cues: vec![cue("1", TipoCue::Audio), cue("2", TipoCue::Audio), cue("3", TipoCue::Imagen)], ..Default::default() };
    guardar(&show, &d.join("e.otashow"), "20261021-000000").unwrap();
    let mut s = Sesion::abrir(&d).unwrap();
    let son = Son { auriculares, recibido: Arc::default() };
    s.conectar_sonido(Box::new(son.clone()));
    while let Some(t) = s.siguiente_a_preparar() {
        let r = t.hacer();
        s.resultado_preparar(t.cue, r);
    }
    son.recibido.lock().unwrap().clear();
    (s, son)
}

fn estado(s: &Sesion) -> serde_json::Value {
    serde_json::from_str(&s.estado_json()).unwrap()
}

#[test]
fn ver_antes_solo_por_auriculares() {
    let (mut s, son) = abrir(true);
    s.cambiar_modo(Modo::Funcion);
    s.ver_antes(1).unwrap();
    let rec = std::mem::take(&mut *son.recibido.lock().unwrap());
    assert!(matches!(rec.as_slice(), [(Destino::Auriculares, Comando::IniciarCue { .. })]), "solo por Auriculares: {rec:?}");
    let e = estado(&s);
    assert_eq!(e["escucha"], 1);
    assert_eq!(e["sonando"].as_array().unwrap().len(), 0, "no cuenta como lo que suena en la sala");
    assert_eq!(s.en_espera, 0, "no mueve el cue en espera");
    assert!(s.go(), "el GO sigue funcionando enseguida");

    // Tocarlo otra vez lo para.
    son.recibido.lock().unwrap().clear();
    s.ver_antes(1).unwrap();
    let rec = std::mem::take(&mut *son.recibido.lock().unwrap());
    assert!(matches!(rec.as_slice(), [(Destino::Auriculares, Comando::Parar { .. })]), "{rec:?}");
    assert!(estado(&s)["escucha"].is_null());

    // Una imagen no tiene sonido; Parar todo también para la escucha.
    assert!(s.ver_antes(2).is_err());
    s.ver_antes(1).unwrap();
    s.parar_todo();
    assert!(estado(&s)["escucha"].is_null());
}

#[test]
fn sin_auriculares_nunca_suena_por_la_sala() {
    let (mut s, son) = abrir(false);
    let err = s.ver_antes(0).unwrap_err();
    assert!(err.contiene("No hay auriculares"), "{err:?}");
    assert!(son.recibido.lock().unwrap().is_empty(), "no se mandó nada a ninguna salida");
}
