//! La sesión con sonido: prepara los audios que vienen, el GO manda el cue
//! completo al mezclador correcto, pausa, fundidos, errores y salidas. Usa un
//! sonido falso que anota lo que recibe.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use otaconvert_audio::Comando;
use otaconvert_servidor::{Destino, Sesion, Sonido};
use otaconvert_show::{guardar, Cue, Show, SALIDA_AURICULARES};

static N: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Default)]
struct Falso {
    recibido: Arc<Mutex<Vec<(Destino, Comando)>>>,
    auriculares: bool,
}

impl Sonido for Falso {
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

fn dato(n: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../audio/tests/datos").join(n)
}

fn carpeta_con_audios() -> PathBuf {
    let d = std::env::temp_dir().join(format!("otaconvert-prueba-sonido-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("audio")).unwrap();
    std::fs::copy(dato("tono.wav"), d.join("audio/tono.wav")).unwrap();
    std::fs::copy(dato("tono.flac"), d.join("audio/tono.flac")).unwrap();
    d
}

fn abrir(d: &Path, cues: Vec<Cue>, auriculares: bool) -> (Sesion, Falso) {
    let show = Show { nombre: "Sonido".into(), cues, ..Default::default() };
    guardar(&show, &d.join("s.otashow"), "20261016-000000").unwrap();
    let mut s = Sesion::abrir(d).unwrap();
    let f = Falso { auriculares, ..Default::default() };
    s.conectar_sonido(Box::new(f.clone()));
    f.recibido.lock().unwrap().clear(); // ajustes iniciales
    (s, f)
}

/// Prepara todo lo que pida la sesión (lo que hace el hilo preparador).
fn preparar_todo(s: &mut Sesion) {
    while let Some((i, ruta, fs)) = s.siguiente_a_preparar() {
        let r = otaconvert_audio::preparar::preparar(&ruta, fs).map_err(|e| e.to_string());
        s.preparado(i, r);
    }
}

fn cue(n: &str, archivo: &str) -> Cue {
    Cue { numero: n.into(), nombre: format!("Cue {n}"), archivo: archivo.into(), ..Default::default() }
}

#[test]
fn go_sin_preparar_no_dispara_y_avisa() {
    let d = carpeta_con_audios();
    let (mut s, f) = abrir(&d, vec![cue("1", "audio/tono.wav")], false);
    assert!(!s.go());
    assert_eq!(s.en_espera, 0);
    assert!(s.aviso.as_deref().unwrap().contains("preparando"));
    assert!(f.recibido.lock().unwrap().is_empty());
}

#[test]
fn go_manda_el_cue_completo() {
    let d = carpeta_con_audios();
    let c = Cue {
        volumen_db: -6.0,
        fundido_entrada_ms: 1500,
        fundido_salida_ms: 250,
        inicio_ms: 100,
        fin_ms: Some(900),
        bucle: true,
        ..cue("1", "audio/tono.wav")
    };
    let (mut s, f) = abrir(&d, vec![c, cue("2", "audio/tono.flac")], false);
    preparar_todo(&mut s);
    assert!(s.esta_preparado(0) && s.esta_preparado(1), "prepara el que está en espera y los siguientes");
    assert!(s.go());
    let r = f.recibido.lock().unwrap();
    match &r[0] {
        (Destino::Principal, Comando::IniciarCue { volumen, inicio, desde, hasta, bucle, fundido_entrada, fundido_salida, pista, .. }) => {
            assert!((volumen - 0.501).abs() < 0.001, "−6 dB ≈ 0,501: {volumen}");
            assert_eq!(*inicio, 4800);
            assert_eq!(*desde, 4800);
            assert_eq!(*hasta, Some(43_200));
            assert!(*bucle);
            assert_eq!(*fundido_entrada, 72_000);
            assert_eq!(*fundido_salida, 12_000);
            assert_eq!(pista.frecuencia(), 48_000, "preparado a la frecuencia de la salida");
        }
        otro => panic!("{otro:?}"),
    }
    assert_eq!(s.en_espera, 1);
}

#[test]
fn pausa_fundidos_y_parar() {
    let d = carpeta_con_audios();
    let (mut s, f) = abrir(&d, vec![cue("1", "audio/tono.wav"), cue("2", "audio/tono.flac")], true);
    preparar_todo(&mut s);
    s.go();
    std::thread::sleep(Duration::from_millis(320));
    s.go();
    f.recibido.lock().unwrap().clear();

    s.pausa();
    s.pausa();
    s.fundir_ultimo();
    s.fundir_todo();
    s.parar_todo();
    let r = f.recibido.lock().unwrap();
    let nombres: Vec<String> = r.iter().map(|(d, c)| format!("{d:?} {}", format!("{c:?}").split([' ', '{', '(']).next().unwrap())).collect();
    assert_eq!(
        nombres,
        [
            "Principal PausarTodo",
            "Auriculares PausarTodo",
            "Principal ReanudarTodo",
            "Auriculares ReanudarTodo",
            "Principal Parar",
            "Principal PararTodo",
            "Auriculares PararTodo",
            "Principal PararTodo",
            "Auriculares PararTodo",
        ]
    );
    // Mayús+F funde solo el último disparado (id 2) en 3 s.
    assert!(matches!(r[4].1, Comando::Parar { id: 2, fundido: 144_000 }), "{:?}", r[4].1);
    assert!(matches!(r[5].1, Comando::PararTodo { fundido: 144_000 }));
    assert!(matches!(r[7].1, Comando::PararTodo { fundido: 0 }));
}

#[test]
fn archivo_que_falta_avisa_pero_la_funcion_sigue() {
    let d = carpeta_con_audios();
    let (mut s, f) = abrir(&d, vec![cue("1", "audio/no-existe.wav"), cue("2", "audio/tono.wav")], false);
    preparar_todo(&mut s);
    assert!(s.estado_json().contains("\"preparacion\":\"error\""));
    assert!(s.go(), "un cue roto no traba la función");
    assert!(s.aviso.as_deref().unwrap().contains("no sonó"));
    assert_eq!(s.en_espera, 1);
    assert!(f.recibido.lock().unwrap().is_empty());
}

#[test]
fn auriculares_sin_configurar_suena_por_principal() {
    let d = carpeta_con_audios();
    let c = Cue { salida: SALIDA_AURICULARES.into(), ..cue("1", "audio/tono.wav") };
    let (mut s, f) = abrir(&d, vec![c.clone()], false);
    preparar_todo(&mut s);
    s.go();
    assert_eq!(f.recibido.lock().unwrap()[0].0, Destino::Principal);
    assert!(s.aviso.as_deref().unwrap().contains("Auriculares"));

    let d2 = carpeta_con_audios();
    let (mut s2, f2) = abrir(&d2, vec![c], true);
    preparar_todo(&mut s2);
    s2.go();
    assert_eq!(f2.recibido.lock().unwrap()[0].0, Destino::Auriculares);
}

#[test]
fn suelta_de_memoria_lo_que_ya_no_hace_falta() {
    let d = carpeta_con_audios();
    let cues: Vec<Cue> = (1..=6).map(|i| cue(&i.to_string(), "audio/tono.wav")).collect();
    let (mut s, _f) = abrir(&d, cues, false);
    preparar_todo(&mut s);
    assert!((0..3).all(|i| s.esta_preparado(i)) && !s.esta_preparado(3));
    s.mover_espera(3);
    preparar_todo(&mut s);
    assert!(!s.esta_preparado(0), "el cue 1 ya no se va a usar");
    assert!((3..6).all(|i| s.esta_preparado(i)));
}

#[test]
fn lo_que_suena_se_ve_y_termina_solo() {
    let d = carpeta_con_audios();
    let c = Cue { fin_ms: Some(300), ..cue("1", "audio/tono.wav") };
    let (mut s, _f) = abrir(&d, vec![c], false);
    preparar_todo(&mut s);
    s.go();
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert_eq!(e["sonando"].as_array().unwrap().len(), 1);
    assert!(e["sonando"][0]["restante_s"].as_f64().unwrap() <= 0.3);
    let t = Instant::now();
    while t.elapsed() < Duration::from_millis(700) {
        std::thread::sleep(Duration::from_millis(50));
        s.limpiar();
    }
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert!(e["sonando"].as_array().unwrap().is_empty());
}

#[test]
fn en_funcion_no_se_cambia_el_show() {
    use otaconvert_servidor::sesion::Modo;
    let d = carpeta_con_audios();
    let (mut s, _f) = abrir(&d, vec![cue("1", "audio/tono.wav"), cue("2", "audio/tono.flac")], false);
    s.cambiar_modo(Modo::Funcion);
    let antes = s.show.clone();
    assert!(s.editar_cue(0, &serde_json::json!({ "nombre": "Otro" }), true).is_err());
    s.ir_a(1);
    assert_eq!(s.en_espera, 0, "en Función no se elige con el ratón");
    assert!(s.show == antes && !s.sucio);
    // ↑↓ siguen funcionando en Función.
    s.mover_espera(1);
    assert_eq!(s.en_espera, 1);
    s.cambiar_modo(Modo::Edicion);
    s.ir_a(0);
    assert_eq!(s.en_espera, 0);
    s.ir_a(99);
    assert_eq!(s.en_espera, 0, "un cue que no existe no cambia nada");
    let registro = std::fs::read_dir(d.join("registros")).unwrap().flatten().next().unwrap().path();
    let texto = std::fs::read_to_string(registro).unwrap();
    assert!(texto.contains("MODO") && texto.contains("Función") && texto.contains("Edición"));
}

#[test]
fn editar_cue_valida_todo_antes_de_cambiar() {
    let d = carpeta_con_audios();
    let (mut s, f) = abrir(&d, vec![cue("1", "audio/tono.wav")], false);
    let ok = serde_json::json!({ "nombre": "Lluvia\nfuerte", "nota": "Cuando entra la novia", "volumen_db": -6.5,
        "fundido_entrada_ms": 1500, "fundido_salida_ms": 3000, "bucle": true, "salida": "Auriculares" });
    s.editar_cue(0, &ok, false).unwrap();
    let c = &s.show.cues[0];
    assert_eq!(c.nombre, "Lluvia fuerte", "sin saltos de línea en el nombre");
    assert_eq!((c.volumen_db, c.fundido_entrada_ms, c.fundido_salida_ms, c.bucle), (-6.5, 1500, 3000, true));
    assert_eq!(c.salida, SALIDA_AURICULARES);
    assert!(s.sucio);
    // Cualquier valor malo: error y nada cambia (tampoco los campos buenos del mismo pedido).
    let antes = s.show.clone();
    for malo in [
        serde_json::json!({ "nombre": "x", "volumen_db": 99 }),
        serde_json::json!({ "volumen_db": "fuerte" }),
        serde_json::json!({ "fundido_entrada_ms": -1 }),
        serde_json::json!({ "fundido_salida_ms": 700_000 }),
        serde_json::json!({ "salida": "Sala" }),
        serde_json::json!({ "archivo": "../../Windows/system.ini" }),
        serde_json::json!({ "nombre": "a".repeat(201) }),
        serde_json::json!({ "bucle": "si" }),
        serde_json::json!("no es un objeto"),
    ] {
        assert!(s.editar_cue(0, &malo, false).is_err(), "{malo}");
        assert!(s.show == antes, "{malo}");
    }
    assert!(s.editar_cue(5, &serde_json::json!({ "nombre": "x" }), false).is_err());
    // Un cue que suena solo se edita confirmando; lo que suena no se toca.
    s.editar_cue(0, &serde_json::json!({ "salida": "Principal", "bucle": false }), false).unwrap();
    preparar_todo(&mut s);
    assert!(s.go());
    let mandados = f.recibido.lock().unwrap().len();
    assert!(s.editar_cue(0, &serde_json::json!({ "volumen_db": -3 }), false).unwrap_err().contains("sonando"));
    s.editar_cue(0, &serde_json::json!({ "volumen_db": -3 }), true).unwrap();
    assert_eq!(s.show.cues[0].volumen_db, -3.0);
    assert_eq!(f.recibido.lock().unwrap().len(), mandados, "editar no manda nada al sonido");
}

#[test]
fn el_estado_trae_lo_que_dibuja_la_ventana() {
    let d = carpeta_con_audios();
    let (mut s, _f) = abrir(&d, vec![cue("1", "audio/tono.wav")], false);
    preparar_todo(&mut s);
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert_eq!(e["modo"], "edicion");
    let c = &e["cues"][0];
    assert!((c["duracion_s"].as_f64().unwrap() - 1.0).abs() < 0.01, "{c}");
    assert_eq!(c["salida"], "Principal");
    assert_eq!(c["archivo"], "audio/tono.wav");
    assert!(s.go());
    s.fundir_ultimo();
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    let son = &e["sonando"][0];
    assert_eq!(son["fundiendo"], true);
    assert_eq!(son["salida"], "Principal");
    assert!((0.0..=1.0).contains(&son["progreso"].as_f64().unwrap()));
}
