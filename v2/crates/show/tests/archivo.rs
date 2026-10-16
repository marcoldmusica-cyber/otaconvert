//! Pruebas del archivo del show: ida y vuelta, archivos dañados o maliciosos,
//! guardado seguro con copias y registro de la función.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use otaconvert_show::rutas::{self, RutaInvalida};
use otaconvert_show::*;

static N: AtomicUsize = AtomicUsize::new(0);

/// Carpeta temporal propia de cada prueba.
fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "otaconvert-prueba-show-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn show_de_ejemplo() -> Show {
    Show {
        nombre: "Bodas de sangre".into(),
        cues: vec![
            Cue { id: 1, numero: "1".into(), nombre: "Música de sala".into(), archivo: "audio/sala.mp3".into(), bucle: true, ..Default::default() },
            Cue {
                id: 2,
                numero: "1.5".into(),
                nombre: "Luna sobre el telón".into(),
                tipo: TipoCue::Video,
                archivo: "vídeo/luna lenta.mp4".into(),
                volumen_db: -12.0,
                fundido_entrada_ms: 1500,
                inicio_ms: 3000,
                fin_ms: Some(88_000),
                nota: "Cuando el actor cruza".into(),
                ..Default::default()
            },
        ],
        botonera: vec![PaginaBotonera {
            nombre: "Acto 1".into(),
            botones: vec![BotonSonido { nombre: "Aplausos".into(), archivo: "audio/aplausos.wav".into(), tecla: Some("A".into()), ..Default::default() }],
        }],
        ..Default::default()
    }
}

#[test]
fn ida_y_vuelta() {
    let d = carpeta();
    let ruta = d.join(format!("show.{EXTENSION}"));
    let s = show_de_ejemplo();
    guardar(&s, &ruta, "20261016-120000").unwrap();
    assert_eq!(abrir(&ruta).unwrap(), s);
    assert!(!d.join(format!("show.{EXTENSION}.tmp")).exists(), "quedó el temporal");
}

#[test]
fn campos_que_faltan_toman_su_valor_por_defecto() {
    let s = desde_texto("{}").unwrap();
    assert_eq!(s.version_formato, VERSION_FORMATO);
    assert_eq!(s.ajustes.rampas_ms.inicio, 0.0, "por defecto los audios arrancan de golpe");
    let s = desde_texto(r#"{"cues":[{"nombre":"Trueno","archivo":"trueno.wav"}]}"#).unwrap();
    assert_eq!(s.cues[0].salida, SALIDA_PRINCIPAL);
    assert_eq!(s.cues[0].tipo, TipoCue::Audio);
}

#[test]
fn show_de_una_version_mas_nueva_avisa() {
    let r = desde_texto(r#"{"version_formato": 99, "algo_nuevo": {"x": 1}}"#);
    assert!(matches!(r, Err(ErrorShow::VersionMasNueva(99))), "{r:?}");
    assert!(r.unwrap_err().to_string().contains("Actualiza Otaconvert"));
}

#[test]
fn archivo_danado_dice_donde() {
    match desde_texto("{\n  \"nombre\": \"x\",\n  cues: [\n}") {
        Err(ErrorShow::Danado { linea, .. }) => assert_eq!(linea, 3),
        otro => panic!("{otro:?}"),
    }
    let d = carpeta();
    let ruta = d.join("roto.otashow");
    fs::write(&ruta, [0xFF, 0xFE, 0x00, 0x41]).unwrap();
    assert!(matches!(abrir(&ruta), Err(ErrorShow::Danado { .. })));
}

#[test]
fn rutas_peligrosas_dentro_del_show_se_rechazan() {
    for mala in ["../../Windows/win.ini", "C:/Windows/win.ini", "\\\\servidor\\x\\a.wav", "audio/nul.wav"] {
        let texto = format!(r#"{{"cues":[{{"nombre":"malo","archivo":{}}}]}}"#, serde_json::to_string(mala).unwrap());
        assert!(matches!(desde_texto(&texto), Err(ErrorShow::RutaMala { .. })), "{mala}");
    }
    let texto = r#"{"botonera":[{"nombre":"p","botones":[{"nombre":"b","archivo":"../x.wav"}]}]}"#;
    assert!(matches!(desde_texto(texto), Err(ErrorShow::RutaMala { .. })));
}

#[test]
fn valores_imposibles_se_rechazan() {
    assert!(matches!(desde_texto(r#"{"cues":[{"volumen_db": 99}]}"#), Err(ErrorShow::ValorMalo(_))));
    assert!(matches!(desde_texto(r#"{"cues":[{"inicio_ms": 5000, "fin_ms": 1000}]}"#), Err(ErrorShow::ValorMalo(_))));
    assert!(matches!(desde_texto(r#"{"ajustes":{"rampas_ms":{"inicio": -3}}}"#), Err(ErrorShow::ValorMalo(_))));
}

#[test]
fn archivo_enorme_no_se_abre() {
    let d = carpeta();
    let ruta = d.join("enorme.otashow");
    let f = fs::File::create(&ruta).unwrap();
    f.set_len(archivo::TAMANO_MAXIMO + 1).unwrap();
    assert!(matches!(abrir(&ruta), Err(ErrorShow::Demasiado(_))));
}

#[test]
fn marca_bom_del_bloc_de_notas() {
    let d = carpeta();
    let ruta = d.join("bom.otashow");
    fs::write(&ruta, "\u{FEFF}{\"nombre\":\"Con BOM\"}").unwrap();
    assert_eq!(abrir(&ruta).unwrap().nombre, "Con BOM");
}

#[test]
fn guarda_copias_y_conserva_las_ultimas_20() {
    let d = carpeta();
    let ruta = d.join("show.otashow");
    let mut s = show_de_ejemplo();
    for i in 0..26 {
        s.nombre = format!("versión {i}");
        guardar(&s, &ruta, &format!("20261016-12{i:04}")).unwrap();
    }
    let copias: Vec<_> = fs::read_dir(d.join(".otaconvert/copias")).unwrap().flatten().collect();
    assert_eq!(copias.len(), archivo::COPIAS);
    assert_eq!(abrir(&ruta).unwrap().nombre, "versión 25");
    // La copia más nueva es la versión anterior.
    let mut nombres: Vec<String> = copias.iter().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    nombres.sort();
    let ultima = abrir(&d.join(".otaconvert/copias").join(nombres.last().unwrap())).unwrap();
    assert_eq!(ultima.nombre, "versión 24");
}

#[test]
fn resolver_queda_dentro_de_la_carpeta() {
    let d = carpeta();
    fs::create_dir_all(d.join("audio")).unwrap();
    fs::write(d.join("audio/a.wav"), b"x").unwrap();
    let r = rutas::resolver(&d, "audio/a.wav").unwrap();
    assert!(r.starts_with(&d));
    assert_eq!(rutas::resolver(&d, "../a.wav"), Err(RutaInvalida::SaleDeLaCarpeta));
}

/// Una unión de carpetas (junction) dentro del show que apunta afuera no debe
/// servir para leer archivos de fuera.
#[cfg(windows)]
#[test]
fn una_union_que_apunta_afuera_se_rechaza() {
    let fuera = carpeta();
    fs::write(fuera.join("secreto.wav"), b"x").unwrap();
    let show = carpeta();
    let union = show.join("enlace");
    let ok = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&union)
        .arg(&fuera)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("no se pudo crear la unión; prueba omitida");
        return;
    }
    assert_eq!(rutas::resolver(&show, "enlace/secreto.wav"), Err(RutaInvalida::FueraDeLaCarpeta));
    // Borrar la unión (no lo que apunta) para no dejar basura.
    let _ = fs::remove_dir(&union);
}

#[test]
fn registro_de_la_funcion() {
    let d = carpeta();
    let mut r = Registro::abrir(&d);
    r.anotar("GO", "Cue 3 «Campanas»");
    r.anotar("ERROR", "no se encontró audio/x.wav\nsegunda línea");
    let texto = fs::read_to_string(&r.ruta).unwrap();
    assert!(texto.contains("GO       Cue 3 «Campanas»"), "{texto}");
    assert_eq!(texto.lines().count(), 2, "un salto de línea no debe partir el registro");
    assert!(r.ruta.starts_with(d.join("registros")));
}
