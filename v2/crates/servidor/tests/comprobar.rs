//! Luz verde (etapa 5C): la comprobación antes de la función encuentra lo que
//! hay que revisar y no marca lo que está bien.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use otaconvert_servidor::comprobar::{revisar, Nivel, Pedido};
use otaconvert_show::{Cue, TipoCue};

static N: AtomicUsize = AtomicUsize::new(0);

fn dato(n: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../audio/tests/datos").join(n)
}

/// WAV PCM 16 bits estéreo a 48 kHz con todas las muestras en `valor`.
fn wav_constante(ruta: &Path, valor: f32, cuadros: u32) {
    let datos = cuadros * 4;
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + datos).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&48_000u32.to_le_bytes());
    b.extend_from_slice(&(48_000u32 * 4).to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&datos.to_le_bytes());
    let m = ((valor * 32767.0) as i16).to_le_bytes();
    for _ in 0..cuadros * 2 {
        b.extend_from_slice(&m);
    }
    std::fs::write(ruta, b).unwrap();
}

fn cue(n: &str, archivo: &str, tipo: TipoCue) -> (String, Cue) {
    (n.to_string(), Cue { numero: n.into(), nombre: format!("Cue {n}"), archivo: archivo.into(), tipo, ..Default::default() })
}

fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!("otaconvert-prueba-comprobar-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::copy(dato("tono.wav"), d.join("tono.wav")).unwrap();
    std::fs::copy(dato("tono.opus"), d.join("tono.opus")).unwrap();
    wav_constante(&d.join("mitad.wav"), 0.5, 4800);
    d
}

fn pedido(d: &Path, cues: Vec<(String, Cue)>) -> Pedido {
    Pedido {
        carpeta: d.to_path_buf(),
        cues,
        frecuencia: Some(48_000),
        rampa_inicio_ms: 0.0,
        con_proyeccion: true,
        auriculares: false,
        sucio: false,
        errores: Vec::new(),
    }
}

fn textos(p: &[otaconvert_servidor::comprobar::Punto], n: Nivel) -> Vec<String> {
    p.iter().filter(|x| x.nivel == n).map(|x| x.texto.es.clone()).collect()
}

#[test]
fn un_show_sano_no_tiene_nada_que_revisar() {
    let d = carpeta();
    let mut avance = Vec::new();
    let r = revisar(&pedido(&d, vec![cue("1", "tono.wav", TipoCue::Audio)]), |h, t| avance.push((h, t)));
    assert!(textos(&r, Nivel::Problema).is_empty(), "{:?}", textos(&r, Nivel::Problema));
    assert!(textos(&r, Nivel::Aviso).is_empty(), "{:?}", textos(&r, Nivel::Aviso));
    assert!(textos(&r, Nivel::Bien).iter().any(|t| t.contains("1 de 1 cues")));
    assert_eq!(avance, vec![(1, 1)]);
    // Y en inglés también hay texto.
    assert!(r.iter().all(|p| !p.texto.en.is_empty() && p.texto.en != p.texto.es));
}

#[test]
fn encuentra_lo_que_hay_que_revisar() {
    let d = carpeta();
    let mut fuerte = cue("2", "tono.wav", TipoCue::Audio);
    fuerte.1.volumen_db = 12.0; // 0,5 × 4 = +6 dB
    let mut auris = cue("3", "tono.wav", TipoCue::Audio);
    auris.1.salida = "Auriculares".into();
    let mut p = pedido(
        &d,
        vec![
            cue("1", "falta.wav", TipoCue::Audio),
            fuerte,
            auris,
            cue("4", "tono.opus", TipoCue::Audio),
            cue("5", "mitad.wav", TipoCue::Audio),
            cue("6", "foto.png", TipoCue::Imagen),
        ],
    );
    p.sucio = true;
    p.con_proyeccion = false;
    let r = revisar(&p, |_, _| {});
    let malos = textos(&r, Nivel::Problema);
    let avisos = textos(&r, Nivel::Aviso);
    assert!(malos.iter().any(|t| t.contains("Cue 1") && t.contains("falta el archivo")), "{malos:?}");
    assert!(malos.iter().any(|t| t.contains("Cue 4") && t.contains("no se puede reproducir")), "{malos:?}");
    assert!(malos.iter().any(|t| t.contains("Cue 6") && t.contains("falta el archivo")), "{malos:?}");
    assert!(malos.iter().any(|t| t.contains("proyección no está abierta")), "{malos:?}");
    assert!(avisos.iter().any(|t| t.contains("Cue 2") && t.contains("pasa de 0 dB")), "{avisos:?}");
    assert!(avisos.iter().any(|t| t.contains("Cue 3") && t.contains("Auriculares")), "{avisos:?}");
    assert!(avisos.iter().any(|t| t.contains("Cue 5") && t.contains("clic")), "{avisos:?}");
    assert!(avisos.iter().any(|t| t.contains("sin guardar")), "{avisos:?}");
    // El tono que empieza en silencio no se marca por clic.
    assert!(!avisos.iter().any(|t| t.contains("Cue 3") && t.contains("clic")));

    // Con la rampa al iniciar (o un fundido inicial) ya no hay riesgo de clic.
    p.rampa_inicio_ms = 5.0;
    let r = revisar(&p, |_, _| {});
    assert!(!textos(&r, Nivel::Aviso).iter().any(|t| t.contains("clic")));
}

#[test]
fn sin_salida_de_sonido_es_un_problema() {
    let d = carpeta();
    let mut p = pedido(&d, vec![cue("1", "tono.wav", TipoCue::Audio)]);
    p.frecuencia = None;
    let r = revisar(&p, |_, _| {});
    assert!(textos(&r, Nivel::Problema).iter().any(|t| t.contains("No hay salida de sonido")));
}

#[test]
fn la_luz_es_el_peor_nivel() {
    use otaconvert_servidor::comprobar::{a_json, Punto};
    let p = vec![Punto::nuevo(Nivel::Bien, "a", "a2"), Punto::nuevo(Nivel::Aviso, "b", "b2")];
    let j = a_json(&p, otaconvert_servidor::Idioma::En, "21:30");
    assert_eq!(j["luz"], "aviso");
    assert_eq!(j["puntos"][0]["texto"], "b2", "primero lo más grave, en el idioma pedido");
    assert_eq!(j["hora"], "21:30");
    assert_eq!(a_json(&[], otaconvert_servidor::Idioma::Es, "")["luz"], "bien");
}
