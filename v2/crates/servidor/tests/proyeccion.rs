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
        [OrdenProyector::Precargar { id: 0, inicio_s, desde_s: Some(d), .. }] => {
            assert_eq!(*inicio_s, 0.0, "el bucle sigue volviendo al inicio del cue");
            assert!(*d >= 0.1, "sigue desde donde iba: {d}")
        }
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

// ---- Etapa 4B: el sonido de los vídeos va por nuestro mezclador ----

use otaconvert_audio::Comando;
use otaconvert_servidor::{Destino, Sonido};

#[derive(Clone, Default)]
struct SonidoFalso {
    recibido: Arc<Mutex<Vec<(Destino, Comando)>>>,
}

impl Sonido for SonidoFalso {
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

impl SonidoFalso {
    /// Las voces que se mandaron a sonar: (voz, hasta, bucle).
    fn iniciados(&self) -> Vec<(u32, Option<usize>, bool)> {
        self.recibido
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(_, c)| match c {
                Comando::IniciarCue { id, hasta, bucle, .. } => Some((*id, *hasta, *bucle)),
                _ => None,
            })
            .collect()
    }
    fn parados(&self) -> Vec<(u32, u32)> {
        self.recibido
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(_, c)| if let Comando::Parar { id, fundido } = c { Some((*id, *fundido)) } else { None })
            .collect()
    }
}

/// Carpeta con dos vídeos de verdad: uno con sonido (1 s de tono) y uno mudo.
fn carpeta_con_videos() -> PathBuf {
    let d = carpeta();
    let datos = Path::new(env!("CARGO_MANIFEST_DIR")).join("../audio/tests/datos");
    std::fs::copy(datos.join("tono-video.mp4"), d.join("video/con-sonido.mp4")).unwrap();
    std::fs::copy(datos.join("video-mudo.mp4"), d.join("video/mudo.mp4")).unwrap();
    d
}

fn abrir_con_sonido(d: &Path, cues: Vec<Cue>) -> (Sesion, Falso, SonidoFalso) {
    let show = Show { nombre: "Vídeos".into(), cues, ..Default::default() };
    guardar(&show, &d.join("s.otashow"), "20261016-000000").unwrap();
    let mut s = Sesion::abrir(d).unwrap();
    let son = SonidoFalso::default();
    s.conectar_sonido(Box::new(son.clone()));
    son.recibido.lock().unwrap().clear();
    let f = Falso::default();
    s.conectar_proyector(Box::new(f.clone()));
    (s, f, son)
}

/// Prepara todo lo que pide la sesión (lo que hace el hilo preparador).
fn preparar_todo(s: &mut Sesion) {
    while let Some((i, ruta, fs)) = s.siguiente_a_preparar() {
        s.resultado_preparar(i, otaconvert_audio::preparar::preparar(&ruta, fs));
    }
}

#[test]
fn el_video_con_sonido_suena_por_el_mezclador_y_en_bucle_vuelve_con_la_imagen() {
    let d = carpeta_con_videos();
    let cue = Cue { bucle: true, ..visual("1", "video/con-sonido.mp4", TipoCue::Video) };
    let (mut s, f, son) = abrir_con_sonido(&d, vec![cue]);
    // Pide la imagen y empieza a preparar el sonido (todavía sin terminar).
    let (i, ruta, fs) = s.siguiente_a_preparar().expect("el sonido del vídeo se prepara");
    f.avisar(AvisoProyector::Listo { id: 0, duracion: Some(0.9) });
    s.limpiar();
    // La imagen está, pero el sonido no: no dispara.
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert_eq!(e["cues"][0]["preparacion"], "preparando");
    assert!(!s.go());
    s.resultado_preparar(i, otaconvert_audio::preparar::preparar(&ruta, fs));
    preparar_todo(&mut s);
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert_eq!(e["cues"][0]["preparacion"], "listo");
    assert!((e["cues"][0]["duracion_s"].as_f64().unwrap() - 0.9).abs() < 0.001, "manda la duración del vídeo");
    f.tomar();
    assert!(s.go());
    assert_eq!(f.tomar(), vec![OrdenProyector::Mostrar { id: 0 }]);
    // El sonido arranca a la vez, en bucle, y vuelve donde vuelve la imagen (0,9 s = 43.200 cuadros).
    match son.iniciados().as_slice() {
        [(_, Some(hasta), true)] => assert_eq!(*hasta, 43_200),
        x => panic!("{x:?}"),
    }
    // Mayús+F funde la imagen y también su sonido.
    s.fundir_ultimo();
    assert_eq!(f.tomar(), vec![OrdenProyector::Fundir { ms: 3000 }]);
    let voz = son.iniciados()[0].0;
    assert_eq!(son.parados(), vec![(voz, 3000 * 48)]);
}

#[test]
fn un_video_mudo_no_espera_sonido_y_un_video_nuevo_para_el_sonido_del_anterior() {
    let d = carpeta_con_videos();
    let (mut s, f, son) = abrir_con_sonido(
        &d,
        vec![visual("1", "video/con-sonido.mp4", TipoCue::Video), visual("2", "video/mudo.mp4", TipoCue::Video)],
    );
    preparar_todo(&mut s);
    f.avisar(AvisoProyector::Listo { id: 0, duracion: Some(1.0) });
    f.avisar(AvisoProyector::Listo { id: 1, duracion: Some(1.0) });
    s.limpiar();
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert_eq!(e["cues"][1]["mudo"], true);
    assert_eq!(e["cues"][1]["preparacion"], "listo", "un vídeo mudo no espera sonido");
    assert!(e["cues"][1]["error"].is_null(), "mudo no es un error");
    assert!(s.go());
    let voz = son.iniciados()[0].0;
    std::thread::sleep(std::time::Duration::from_millis(350));
    assert!(s.go(), "el vídeo mudo se muestra");
    assert_eq!(son.iniciados().len(), 1, "el mudo no manda sonido");
    assert_eq!(son.parados(), vec![(voz, 0)], "el sonido del vídeo anterior se para");
}

impl SonidoFalso {
    /// (voz, desde) de cada IniciarCue mandado.
    fn desdes(&self) -> Vec<usize> {
        self.recibido
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(_, c)| if let Comando::IniciarCue { desde, .. } = c { Some(*desde) } else { None })
            .collect()
    }
}

#[test]
fn salir_desde_aqui_precarga_en_ese_punto_y_el_sonido_sale_desde_el_mismo_segundo() {
    let d = carpeta_con_videos();
    let (mut s, f, son) = abrir_con_sonido(
        &d,
        vec![
            visual("1", "video/con-sonido.mp4", TipoCue::Video),
            Cue { numero: "2".into(), nombre: "Audio".into(), archivo: "video/con-sonido.mp4".into(), ..Default::default() },
        ],
    );
    preparar_todo(&mut s);
    f.avisar(AvisoProyector::Listo { id: 0, duracion: Some(1.0) });
    s.limpiar();
    f.tomar();
    // Solo el vídeo en espera, y dentro del vídeo.
    assert!(s.elegir_desde(1, Some(0.5)).is_err(), "no es el cue en espera");
    assert!(s.elegir_desde(0, Some(5.0)).is_err(), "fuera del vídeo");
    assert!(s.elegir_desde(0, Some(f64::NAN)).is_err());
    // Elegir 0,5 s: se vuelve a precargar en ese punto.
    s.elegir_desde(0, Some(0.5)).unwrap();
    let o = f.tomar();
    assert!(o.contains(&OrdenProyector::Quitar { id: 0 }), "{o:?}");
    match o.iter().find(|x| matches!(x, OrdenProyector::Precargar { .. })) {
        Some(OrdenProyector::Precargar { desde_s: Some(x), inicio_s, .. }) => {
            assert_eq!(*x, 0.5);
            assert_eq!(*inicio_s, 0.0, "el bucle sigue volviendo al inicio del cue");
        }
        x => panic!("{x:?}"),
    }
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert_eq!(e["desde_s"], 0.5);
    assert!(!s.go(), "hasta que esté lista en el punto nuevo, no dispara");
    f.avisar(AvisoProyector::Listo { id: 0, duracion: Some(1.0) });
    s.limpiar();
    std::thread::sleep(std::time::Duration::from_millis(350));
    assert!(s.go());
    assert!(f.tomar().contains(&OrdenProyector::Mostrar { id: 0 }));
    assert_eq!(son.desdes(), vec![24_000], "el sonido sale desde 0,5 s (24.000 cuadros a 48 kHz)");
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert!(e["desde_s"].is_null(), "se usa una sola vez");
    let falta = e["sonando"][0]["restante_s"].as_f64().unwrap();
    assert!((0.4..=0.51).contains(&falta), "le falta medio segundo: {falta}");
}

#[test]
fn cambiar_de_cue_olvida_el_punto_elegido() {
    let d = carpeta_con_videos();
    let (mut s, f, _son) = abrir_con_sonido(
        &d,
        vec![visual("1", "video/con-sonido.mp4", TipoCue::Video), visual("2", "video/mudo.mp4", TipoCue::Video)],
    );
    preparar_todo(&mut s);
    f.avisar(AvisoProyector::Listo { id: 0, duracion: Some(1.0) });
    s.limpiar();
    s.elegir_desde(0, Some(0.5)).unwrap();
    f.tomar();
    s.ir_a(1);
    assert!(f.tomar().contains(&OrdenProyector::Quitar { id: 0 }), "se suelta la precarga en ese punto");
    s.ir_a(0);
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert!(e["desde_s"].is_null());
    s.siguiente_a_preparar();
    match f.tomar().iter().find(|x| matches!(x, OrdenProyector::Precargar { id: 0, .. })) {
        Some(OrdenProyector::Precargar { desde_s, .. }) => assert_eq!(*desde_s, None, "vuelve desde su inicio"),
        x => panic!("{x:?}"),
    }
}
