//! Preparar audios: cada formato se lee, pasa a 48 kHz sin deformarse y
//! conserva su volumen; lo que no se puede leer da un error, nunca un cierre.
//! Los archivos de `tests/datos` son un tono de 1 kHz a -6 dBFS (pico 0,5)
//! de 1 s a 44,1 kHz estéreo, creados con FFmpeg.

use std::path::{Path, PathBuf};

use otaconvert_audio::preparar::{preparar, ErrorPreparar, Preparado};
use otaconvert_audio::Detector;

fn dato(n: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/datos").join(n)
}

/// Frecuencia medida contando cruces por cero en la parte central.
fn frecuencia_medida(p: &Preparado) -> f64 {
    let fs = p.pista.frecuencia() as usize;
    let canal0: Vec<f32> = (0..p.pista.cuadros()).map(|i| p.pista.muestras()[i * p.pista.canales() as usize]).collect();
    let tramo = &canal0[fs / 10..fs * 8 / 10];
    let cruces = tramo.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
    cruces as f64 / (tramo.len() as f64 / fs as f64)
}

#[test]
fn todos_los_formatos_a_48k() {
    for (archivo, tolerancia_s) in [
        ("tono.wav", 0.001),
        ("tono.flac", 0.001),
        ("tono.aiff", 0.001),
        ("tono.mp3", 0.06),
        ("tono.ogg", 0.06),
        ("tono.m4a", 0.06),
    ] {
        let p = preparar(&dato(archivo), 48_000).unwrap_or_else(|e| panic!("{archivo}: {e}"));
        assert_eq!(p.pista.frecuencia(), 48_000, "{archivo}");
        assert_eq!(p.pista.canales(), 2, "{archivo}");
        assert_eq!(p.frecuencia_original, 44_100, "{archivo}");
        assert!((p.duracion_s() - 1.0).abs() <= tolerancia_s, "{archivo}: dura {}", p.duracion_s());
        assert!((p.pico - 0.5).abs() < 0.05, "{archivo}: pico {}", p.pico);
        let f = frecuencia_medida(&p);
        assert!((f - 1000.0).abs() < 10.0, "{archivo}: frecuencia {f}");
        assert_eq!(p.paquetes_danados, 0, "{archivo}");

        // Sin saltos raros en la parte central (los extremos dependen del códec).
        let fs = 48_000;
        let centro = &p.pista.muestras()[2 * fs / 10..2 * fs * 8 / 10];
        let mut d = Detector::nuevo(2, Detector::umbral_para_tonos(0.55, 1000.0, fs as u32));
        d.analizar(&centro[..4]);
        d.discontinuidades = 0;
        d.analizar(&centro[4..]);
        assert_eq!(d.discontinuidades, 0, "{archivo}: discontinuidad máxima {}", d.maxima);
    }
}

#[test]
fn misma_frecuencia_no_se_toca() {
    let p = preparar(&dato("tono-mono-48k.wav"), 48_000).unwrap();
    assert_eq!(p.pista.canales(), 1);
    assert_eq!(p.pista.cuadros(), 24_000);
    assert_eq!(p.frecuencia_original, 48_000);
}

#[test]
fn mas_de_dos_canales_pasan_a_estereo() {
    let p = preparar(&dato("tono-6canales-48k.wav"), 48_000).unwrap();
    assert_eq!(p.canales_originales, 6);
    assert_eq!(p.pista.canales(), 2);
    assert!(p.pico > 0.2, "pico {}", p.pico);
}

#[test]
fn opus_no_esta_soportado_y_lo_dice() {
    match preparar(&dato("tono.opus"), 48_000) {
        Err(e @ ErrorPreparar::FormatoNoSoportado(_)) => assert!(e.to_string().contains("Convertir")),
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn archivos_malos_no_cierran_nada() {
    assert!(matches!(preparar(&dato("no-existe.wav"), 48_000), Err(ErrorPreparar::NoSePudoAbrir(_))));
    let basura = std::env::temp_dir().join(format!("otaconvert-basura-{}.wav", std::process::id()));
    let bytes: Vec<u8> = (0..20_000u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
    std::fs::write(&basura, bytes).unwrap();
    assert!(preparar(&basura, 48_000).is_err());
    // Un WAV cortado por la mitad.
    let entero = std::fs::read(dato("tono.wav")).unwrap();
    let cortado = std::env::temp_dir().join(format!("otaconvert-cortado-{}.wav", std::process::id()));
    std::fs::write(&cortado, &entero[..entero.len() / 2]).unwrap();
    if let Ok(p) = preparar(&cortado, 48_000) {
        assert!(p.duracion_s() < 0.6, "debería leer solo la mitad");
    }
    let _ = std::fs::remove_file(basura);
    let _ = std::fs::remove_file(cortado);
}
