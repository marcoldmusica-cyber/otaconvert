//! Preparar audios: cada formato se lee, pasa a 48 kHz sin deformarse y
//! conserva su volumen; lo que no se puede leer da un error, nunca un cierre.
//! Los archivos de `tests/datos` son un tono de 1 kHz a -6 dBFS (pico 0,5)
//! de 1 s a 44,1 kHz estéreo, creados con FFmpeg.

use std::path::{Path, PathBuf};

use otaconvert_audio::preparar::{preparar, preparar_con, ErrorPreparar, Preparado};
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

/// Etapa 4B: el sonido de un vídeo (MP4 con imagen H.264 y sonido AAC) se
/// prepara igual que un audio; un vídeo sin sonido dice "sin sonido".
#[test]
fn el_sonido_de_un_video_se_prepara() {
    let p = preparar(&dato("tono-video.mp4"), 48_000).expect("el sonido del vídeo");
    assert!((p.duracion_s() - 1.0).abs() < 0.06, "duración {}", p.duracion_s());
    assert!((p.pico - 0.5).abs() < 0.05, "pico {}", p.pico);
    assert!((frecuencia_medida(&p) - 1000.0).abs() < 10.0);
    match preparar(&dato("video-mudo.mp4"), 48_000) {
        Err(otaconvert_audio::preparar::ErrorPreparar::SinAudio) => {}
        otro => panic!("un vídeo sin sonido debe dar SinAudio: {:?}", otro.map(|p| p.duracion_s())),
    }
}

/// Escribe un WAV de 16 bits estéreo con un tono que empalma perfecto en
/// bucle (un número entero de ciclos).
fn wav_en_bucle(ruta: &Path, fs: u32, hz: f32, ciclos: u32, amp: f32) {
    let n = (ciclos as f64 * fs as f64 / hz as f64).round() as u32;
    let mut d = Vec::new();
    d.extend_from_slice(b"RIFF");
    d.extend_from_slice(&(36 + n * 4).to_le_bytes());
    d.extend_from_slice(b"WAVEfmt ");
    for v in [16u32] {
        d.extend_from_slice(&v.to_le_bytes());
    }
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&2u16.to_le_bytes());
    d.extend_from_slice(&fs.to_le_bytes());
    d.extend_from_slice(&(fs * 4).to_le_bytes());
    d.extend_from_slice(&4u16.to_le_bytes());
    d.extend_from_slice(&16u16.to_le_bytes());
    d.extend_from_slice(b"data");
    d.extend_from_slice(&(n * 4).to_le_bytes());
    for i in 0..n {
        let v = (amp * (2.0 * std::f64::consts::PI * hz as f64 * i as f64 / fs as f64).sin() as f32 * 32767.0) as i16;
        d.extend_from_slice(&v.to_le_bytes());
        d.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(ruta, d).unwrap();
}

/// Salto en la unión del bucle: segunda diferencia entre el final y el principio.
fn salto_en_la_union(p: &Preparado) -> f32 {
    let ch = p.pista.canales() as usize;
    let m = p.pista.muestras();
    let n = m.len() / ch;
    let x = |k: isize| m[(k.rem_euclid(n as isize) as usize) * ch];
    (-1..=1).map(|k| (x(k) - 2.0 * x(k - 1) + x(k - 2)).abs()).fold(0.0, f32::max)
}

#[test]
fn un_bucle_que_empalma_sigue_empalmando_al_adaptar_la_frecuencia() {
    // Hallado en la función simulada: un ambiente en bucle a 44,1 kHz hacía
    // clic en cada vuelta con la tarjeta a 48 kHz.
    let dir = std::env::temp_dir().join(format!("otaconvert-bucle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (fs, hz, ciclos) in [(44_100u32, 440.0f32, 1760u32), (22_050, 441.0, 441), (96_000, 1000.0, 2000)] {
        let ruta = dir.join(format!("bucle-{fs}.wav"));
        wav_en_bucle(&ruta, fs, hz, ciclos, 0.5);
        let umbral = Detector::umbral_para_tonos(0.5, hz, 48_000);
        let suelto = preparar(&ruta, 48_000).unwrap();
        let en_bucle = preparar_con(&ruta, 48_000, true).unwrap();
        // La vuelta dura exactamente lo que dura el archivo (rubato agrega a veces
        // un cuadro de más al final de un sonido suelto: en un bucle sería un salto).
        let exacto = (ciclos as f64 * fs as f64 / hz as f64).round() * 48_000.0 / fs as f64;
        assert_eq!(en_bucle.pista.cuadros() as f64, exacto.round(), "{fs} Hz");
        assert!(suelto.pista.cuadros().abs_diff(en_bucle.pista.cuadros()) <= 1, "{fs} Hz");
        let s = salto_en_la_union(&en_bucle);
        assert!(s <= umbral, "{fs} Hz: salto en la unión {s} (umbral {umbral})");
        // Sin avisar que es un bucle, el principio y el final se adaptan como
        // un sonido suelto (desde y hacia el silencio): ahí sí hay salto.
        if fs == 44_100 {
            assert!(salto_en_la_union(&suelto) > umbral, "la prueba tiene que notar el problema");
        }
        // El volumen no cambia en los bordes ni en el medio.
        let m = en_bucle.pista.muestras();
        let pico_borde = m[..400].iter().chain(&m[m.len() - 400..]).fold(0f32, |a, x| a.max(x.abs()));
        assert!((pico_borde - 0.5).abs() < 0.02, "{fs} Hz: pico en los bordes {pico_borde}");
        let medio = m.len() / 2;
        for k in (medio..medio + 2000).step_by(7) {
            assert!((m[k] - suelto.pista.muestras()[k]).abs() < 1e-3, "{fs} Hz: el medio cambió");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mp4_y_m4a_saltan_el_arranque_del_codificador_como_ffmpeg() {
    // Hallado en la función simulada: el sonido de los vídeos empezaba con
    // ~21 ms de "arranque" del codificador AAC (Symphonia 0.6.1 no aplica la
    // lista de edición del MP4). Quedaba atrasado respecto de la imagen y un
    // bucle saltaba a ese silencio en cada vuelta.
    for (archivo, fs, segundos) in [("tono-video.mp4", 48_000usize, 1.0f64), ("tono.m4a", 44_100, 1.0)] {
        let p = preparar(&dato(archivo), fs as u32).unwrap();
        let ch = p.pista.canales() as usize;
        let m = p.pista.muestras();
        let cuadros = m.len() / ch;
        // (Al adaptar la frecuencia, un sonido suelto puede llevar un cuadro de más.)
        assert!(cuadros.abs_diff((segundos * fs as f64) as usize) <= 1, "{archivo}: dura {cuadros} cuadros");
        // El tono empieza enseguida (no hay ~1024 cuadros de silencio delante).
        let primero = (0..cuadros).find(|&k| m[k * ch].abs() > 0.05).unwrap();
        assert!(primero < 48, "{archivo}: el sonido empieza en el cuadro {primero}");
    }
    // Lo mismo vale para el vídeo del show de prueba, en bucle: empalma.
    let p = preparar_con(&dato("tono-video.mp4"), 48_000, true).unwrap();
    let s = salto_en_la_union(&p);
    println!("salto en la unión del vídeo en bucle: {s}");
}
