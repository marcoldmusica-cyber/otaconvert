//! La sesión con proyección (etapa 4A): precarga de los próximos vídeos e
//! imágenes, GO, Parar todo, fundidos, pausa, errores y recuperación tras una
//! caída de la proyección. Usa un proyector falso que anota las órdenes.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use otaconvert_servidor::{AvisoProyector, OrdenProyector, Proyector, Sesion, Texto};
use otaconvert_show::{guardar, Cue, Show, TipoCue};

static N: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Default)]
struct Falso {
    ordenes: Arc<Mutex<Vec<OrdenProyector>>>,
    avisos: Arc<Mutex<Vec<AvisoProyector>>>,
}

impl Proyector for Falso {
    fn enviar(&mut self, o: OrdenProyector) {
        self.ordenes.lock().unwrap().push(o);
    }
    fn recoger(&mut self) -> Vec<AvisoProyector> {
        std::mem::take(&mut *self.avisos.lock().unwrap())
    }
}

impl Falso {
    fn tomar(&self) -> Vec<OrdenProyector> {
        std::mem::take(&mut *self.ordenes.lock().unwrap())
    }
    fn avisar(&self, a: AvisoProyector) {
        self.avisos.lock().unwrap().push(a);
    }
}

fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!("otaconvert-prueba-proyeccion-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("video")).unwrap();
    for f in ["a.mp4", "b.mp4", "c.png", "d.mp4"] {
        std::fs::write(d.join("video").join(f), b"no es un video de verdad").unwrap();
    }
    d
}

fn visual(n: &str, archivo: &str, tipo: TipoCue) -> Cue {
    Cue { numero: n.into(), nombre: format!("Cue {n}"), archivo: archivo.into(), tipo, ..Default::default() }
}

fn abrir(d: &Path, cues: Vec<Cue>) -> (Sesion, Falso) {
    let show = Show { nombre: "Proyección".into(), cues, ..Default::default() };
    guardar(&show, &d.join("s.otashow"), "20261016-000000").unwrap();
    let mut s = Sesion::abrir(d).unwrap();
    let f = Falso::default();
    s.conectar_proyector(Box::new(f.clone()));
    (s, f)
}

fn show_basico(d: &Path) -> (Sesion, Falso) {
    abrir(
        d,
        vec![
            visual("1", "video/a.mp4", TipoCue::Video),
            visual("2", "video/b.mp4", TipoCue::Video),
            visual("3", "video/c.png", TipoCue::Imagen),
            visual("4", "video/d.mp4", TipoCue::Video),
        ],
    )
}

fn precargas(o: &[OrdenProyector]) -> Vec<u64> {
    o.iter().filter_map(|x| if let OrdenProyector::Precargar { id, .. } = x { Some(*id) } else { None }).collect()
}

#[test]
fn precarga_los_dos_siguientes_con_ruta_completa() {
    let d = carpeta();
    let (mut s, f) = show_basico(&d);
    s.siguiente_a_preparar();
    let o = f.tomar();
    assert_eq!(precargas(&o), vec![0, 1], "solo los dos próximos");
    match &o[0] {
        OrdenProyector::Precargar { ruta, imagen, .. } => {
            assert!(ruta.is_absolute() && ruta.ends_with("video/a.mp4"), "{ruta:?}");
            assert!(!imagen);
        }
        x => panic!("{x:?}"),
    }
    // No se repite si ya se pidió.
    s.siguiente_a_preparar();
    assert!(f.tomar().is_empty());
}

#[test]
fn go_sin_precargar_no_dispara_y_con_listo_muestra() {
    let d = carpeta();
    let (mut s, f) = show_basico(&d);
    s.siguiente_a_preparar();
    f.tomar();
    assert!(!s.go(), "todavía se está preparando");
    assert!(s.aviso.as_ref().unwrap().contiene("preparando"));
    f.avisar(AvisoProyector::Listo { id: 0, duracion: Some(10.0) });
    s.limpiar();
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert_eq!(e["cues"][0]["preparacion"], "listo");
    assert_eq!(e["cues"][0]["duracion_s"], 10.0);
    assert!(s.go());
    assert_eq!(f.tomar(), vec![OrdenProyector::Mostrar { id: 0 }]);
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert_eq!(e["sonando"][0]["visual"], true);
    assert!(e["sonando"][0]["restante_s"].as_f64().unwrap() > 9.0);
    // Ahora se precarga el que viene después de los dos siguientes.
    s.siguiente_a_preparar();
    assert_eq!(precargas(&f.tomar()), vec![2]);
}

#[test]
fn parar_fundir_y_pausa_llegan_a_la_proyeccion() {
    let d = carpeta();
    let (mut s, f) = show_basico(&d);
    s.siguiente_a_preparar();
    f.avisar(AvisoProyector::Listo { id: 0, duracion: Some(10.0) });
    s.limpiar();
    assert!(s.go());
    f.tomar();
    s.pausa();
    s.pausa();
    assert_eq!(f.tomar(), vec![OrdenProyector::Pausa(true), OrdenProyector::Pausa(false)]);
    s.fundir_ultimo();
    assert_eq!(f.tomar(), vec![OrdenProyector::Fundir { ms: 3000 }]);
    s.fundir_todo();
    assert_eq!(f.tomar(), vec![OrdenProyector::Fundir { ms: 3000 }]);
    s.parar_todo();
    assert_eq!(f.tomar(), vec![OrdenProyector::Negro]);
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert!(e["sonando"].as_array().unwrap().is_empty());
}

#[test]
fn el_fin_del_video_lo_saca_de_sonando() {
    let d = carpeta();
    let (mut s, f) = show_basico(&d);
    s.siguiente_a_preparar();
    f.avisar(AvisoProyector::Listo { id: 0, duracion: Some(1.0) });
    s.limpiar();
    assert!(s.go());
    f.avisar(AvisoProyector::Fin { id: 0 });
    s.limpiar();
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert!(e["sonando"].as_array().unwrap().is_empty());
}

#[test]
fn un_error_avanza_y_avisa_y_un_archivo_que_falta_no_se_pide() {
    let d = carpeta();
    let (mut s, f) = abrir(&d, vec![visual("1", "video/a.mp4", TipoCue::Video), visual("2", "video/no-existe.mp4", TipoCue::Video)]);
    s.siguiente_a_preparar();
    assert_eq!(precargas(&f.tomar()), vec![0], "el que falta no se pide");
    f.avisar(AvisoProyector::Error { id: 0, motivo: Texto::nuevo("no se pudo abrir el archivo", "the file could not be opened") });
    s.limpiar();
    assert!(s.go(), "un cue roto no traba la función");
    assert!(s.aviso.as_ref().unwrap().contiene("no se vio"));
    assert_eq!(s.en_espera, 1);
    // Pasado el tiempo mínimo entre GO (protección contra doble GO).
    std::thread::sleep(std::time::Duration::from_millis(350));
    assert!(s.go());
    assert!(s.aviso.as_ref().unwrap().contiene("no se encontró"));
}

#[test]
fn mover_la_espera_suelta_lo_que_ya_no_hace_falta() {
    let d = carpeta();
    let (mut s, f) = show_basico(&d);
    s.siguiente_a_preparar();
    f.tomar();
    s.mover_espera(2);
    s.siguiente_a_preparar();
    let o = f.tomar();
    assert!(o.contains(&OrdenProyector::Quitar { id: 0 }) && o.contains(&OrdenProyector::Quitar { id: 1 }), "{o:?}");
    assert_eq!(precargas(&o), vec![2, 3]);
    match o.iter().find(|x| matches!(x, OrdenProyector::Precargar { id: 2, .. })).unwrap() {
        OrdenProyector::Precargar { imagen, .. } => assert!(imagen, "el cue 3 es una imagen"),
        _ => unreachable!(),
    }
}

#[test]
fn tras_una_caida_vuelve_lo_que_se_veia() {
    let d = carpeta();
    let (mut s, f) = show_basico(&d);
    s.siguiente_a_preparar();
    f.avisar(AvisoProyector::Listo { id: 0, duracion: Some(30.0) });
    s.limpiar();
    assert!(s.go());
    std::thread::sleep(std::time::Duration::from_millis(120));
    f.tomar();
    f.avisar(AvisoProyector::Reiniciado { ms: 90 });
    s.limpiar();
    assert!(s.aviso.as_ref().unwrap().contiene("se volvió a abrir"));
    let o = f.tomar();
    match o.as_slice() {
        [OrdenProyector::Precargar { id: 0, inicio_s, .. }] => assert!(*inicio_s >= 0.1, "sigue desde donde iba: {inicio_s}"),
        x => panic!("{x:?}"),
    }
    // Cuando está lista, se vuelve a mostrar sola; y se piden de nuevo las precargas perdidas.
    f.avisar(AvisoProyector::Listo { id: 0, duracion: Some(30.0) });
    s.limpiar();
    assert_eq!(f.tomar(), vec![OrdenProyector::Mostrar { id: 0 }]);
    s.siguiente_a_preparar();
    assert_eq!(precargas(&f.tomar()), vec![1, 2]);
}

#[test]
fn sin_proyeccion_el_go_avanza_y_avisa() {
    let d = carpeta();
    let show = Show { nombre: "x".into(), cues: vec![visual("1", "video/a.mp4", TipoCue::Video)], ..Default::default() };
    guardar(&show, &d.join("s.otashow"), "20261016-000000").unwrap();
    let mut s = Sesion::abrir(&d).unwrap();
    assert!(s.go());
    assert!(s.aviso.as_ref().unwrap().contiene("no hay proyección"));
}
