//! Vista Lista (como QLab) contra Vista Estudio (como OBS): en la Lista el GO
//! suena encima de lo anterior; en la Estudio lo reemplaza con la transición
//! de Ajustes del show (cuchilla, fundido o blackout). Sonido y proyector falsos.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use otaconvert_audio::Comando;
use otaconvert_servidor::{AvisoProyector, Destino, OrdenProyector, Proyector, Sesion, Sonido};
use otaconvert_show::{guardar, Cue, Show, TipoCue, TipoTransicion, Transicion};

static N: AtomicUsize = AtomicUsize::new(0);

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
    fn tomar(&self) -> Vec<Comando> {
        std::mem::take(&mut *self.recibido.lock().unwrap()).into_iter().map(|(_, c)| c).collect()
    }
}

#[derive(Clone, Default)]
struct ProyFalso {
    ordenes: Arc<Mutex<Vec<OrdenProyector>>>,
    avisos: Arc<Mutex<Vec<AvisoProyector>>>,
}

impl Proyector for ProyFalso {
    fn enviar(&mut self, o: OrdenProyector) {
        self.ordenes.lock().unwrap().push(o);
    }
    fn recoger(&mut self) -> Vec<AvisoProyector> {
        std::mem::take(&mut *self.avisos.lock().unwrap())
    }
}

impl ProyFalso {
    fn tomar(&self) -> Vec<OrdenProyector> {
        std::mem::take(&mut *self.ordenes.lock().unwrap())
    }
}

fn dato(n: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../audio/tests/datos").join(n)
}

fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!("otaconvert-prueba-transicion-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("medios")).unwrap();
    std::fs::copy(dato("tono.wav"), d.join("medios/a.wav")).unwrap();
    std::fs::copy(dato("tono.flac"), d.join("medios/b.flac")).unwrap();
    std::fs::copy(dato("tono-video.mp4"), d.join("medios/v.mp4")).unwrap();
    d
}

fn cue(n: &str, archivo: &str, tipo: TipoCue) -> Cue {
    Cue { numero: n.into(), nombre: format!("Cue {n}"), archivo: archivo.into(), tipo, bucle: true, ..Default::default() }
}

fn abrir(trans: Transicion) -> (Sesion, SonidoFalso, ProyFalso) {
    let d = carpeta();
    let mut show = Show {
        nombre: "Transiciones".into(),
        cues: vec![cue("1", "medios/a.wav", TipoCue::Audio), cue("2", "medios/b.flac", TipoCue::Audio), cue("3", "medios/v.mp4", TipoCue::Video)],
        ..Default::default()
    };
    show.ajustes.transicion = trans;
    show.ajustes.tiempo_minimo_entre_go_ms = 0;
    guardar(&show, &d.join("s.otashow"), "20261022-000000").unwrap();
    let mut s = Sesion::abrir(&d).unwrap();
    let son = SonidoFalso::default();
    let pr = ProyFalso::default();
    s.conectar_sonido(Box::new(son.clone()));
    s.conectar_proyector(Box::new(pr.clone()));
    preparar(&mut s);
    son.tomar();
    (s, son, pr)
}

fn paradas(c: &[Comando]) -> Vec<(u32, u32)> {
    c.iter().filter_map(|x| if let Comando::Parar { id, fundido } = x { Some((*id, *fundido)) } else { None }).collect()
}

fn inicios(c: &[Comando]) -> Vec<(u32, u32)> {
    c.iter().filter_map(|x| if let Comando::IniciarCue { id, fundido_entrada, .. } = x { Some((*id, *fundido_entrada)) } else { None }).collect()
}

fn sonando(s: &Sesion) -> usize {
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    e["sonando"].as_array().unwrap().len()
}

#[test]
fn en_la_vista_lista_los_cues_se_superponen() {
    let (mut s, son, _) = abrir(Transicion { tipo: TipoTransicion::Fundido, ms: 1000 });
    assert!(s.go() && s.go());
    let c = son.tomar();
    assert_eq!(inicios(&c).len(), 2);
    assert!(paradas(&c).is_empty(), "el segundo no para al primero: {c:?}");
    assert_eq!(sonando(&s), 2);
}

#[test]
fn cuchilla_corta_lo_anterior() {
    let (mut s, son, _) = abrir(Transicion { tipo: TipoTransicion::Corte, ms: 1000 });
    assert!(s.go_con(true));
    let primero = inicios(&son.tomar())[0].0;
    assert!(s.go_con(true));
    let c = son.tomar();
    // Fundido 0 = la rampa corta de parada (sin clic).
    assert_eq!(paradas(&c), vec![(primero, 0)]);
    assert_eq!(inicios(&c).len(), 1);
    assert_eq!(sonando(&s), 1, "solo queda el nuevo");
}

#[test]
fn fundido_baja_lo_anterior_mientras_sube_lo_nuevo() {
    let (mut s, son, _) = abrir(Transicion { tipo: TipoTransicion::Fundido, ms: 1000 });
    assert!(s.go_con(true));
    let primero = inicios(&son.tomar())[0].0;
    assert!(s.go_con(true));
    let c = son.tomar();
    assert_eq!(paradas(&c), vec![(primero, 48_000)], "1000 ms a 48 kHz");
    assert_eq!(inicios(&c)[0].1, 48_000, "lo nuevo entra con el mismo fundido");
}

#[test]
fn blackout_espera_a_que_lo_anterior_se_apague() {
    let (mut s, son, _) = abrir(Transicion { tipo: TipoTransicion::Negro, ms: 400 });
    // Nada antes: sale desde negro sin esperar, con media transición de fundido.
    assert!(s.go_con(true));
    let c = son.tomar();
    assert_eq!(inicios(&c).len(), 1);
    assert_eq!(inicios(&c)[0].1, 9_600, "200 ms a 48 kHz");
    let primero = inicios(&c)[0].0;
    assert!(s.go_con(true));
    let c = son.tomar();
    assert_eq!(paradas(&c), vec![(primero, 9_600)], "lo anterior baja en la primera mitad");
    assert!(inicios(&c).is_empty(), "lo nuevo todavía no empezó");
    assert!(s.hay_pendientes());
    std::thread::sleep(Duration::from_millis(230));
    s.atender_pendientes();
    let c = son.tomar();
    assert_eq!(inicios(&c).len(), 1, "a la mitad empieza lo nuevo");
    assert_eq!(inicios(&c)[0].1, 9_600);
    // Parar todo en plena transición: lo que esperaba ya no empieza.
    s.go_con(true);
    son.tomar();
    s.parar_todo();
    assert!(!s.hay_pendientes());
}

#[test]
fn un_video_reemplaza_con_la_transicion_de_la_proyeccion() {
    let t = Transicion { tipo: TipoTransicion::Fundido, ms: 1500 };
    let (mut s, son, pr) = abrir(t);
    s.en_espera = 1;
    assert!(s.go_con(true), "el audio 2");
    let audio = inicios(&son.tomar())[0].0;
    // El vídeo 3, precargado.
    s.siguiente_a_preparar();
    pr.avisos.lock().unwrap().push(AvisoProyector::Listo { id: 2, duracion: Some(1.0) });
    s.atender_proyector();
    pr.tomar();
    assert!(s.go_con(true), "el vídeo");
    assert_eq!(pr.tomar(), vec![OrdenProyector::MostrarCon { id: 2, transicion: t }]);
    let c = son.tomar();
    assert_eq!(paradas(&c), vec![(audio, 72_000)], "el audio anterior baja en 1,5 s");
    assert_eq!(inicios(&c).len(), 1, "el sonido del vídeo entra");
    // Con un audio después (en la Vista Estudio) la imagen se funde a negro.
    s.en_espera = 0;
    preparar(&mut s);
    assert!(s.go_con(true));
    assert_eq!(pr.tomar(), vec![OrdenProyector::Fundir { ms: 1500 }]);
}

#[test]
fn la_transicion_se_elige_en_ajustes_del_show() {
    let (mut s, _, _) = abrir(Transicion::default());
    assert!(s.cambiar_ajustes(&serde_json::json!({ "transicion_tipo": "nada" })).is_err());
    assert!(s.cambiar_ajustes(&serde_json::json!({ "transicion_ms": 20000 })).is_err());
    s.cambiar_ajustes(&serde_json::json!({ "transicion_tipo": "negro", "transicion_ms": 2000 })).unwrap();
    let e: serde_json::Value = serde_json::from_str(&s.estado_json()).unwrap();
    assert_eq!(e["ajustes"]["transicion"], serde_json::json!({ "tipo": "negro", "ms": 2000 }));
    // Los shows viejos (sin el ajuste) abren con la cuchilla.
    let viejo: Show = serde_json::from_str(r#"{"version_formato":1,"nombre":"x","cues":[],"ajustes":{"fundir_todo_ms":3000}}"#).unwrap();
    assert_eq!(viejo.ajustes.transicion, Transicion { tipo: TipoTransicion::Corte, ms: 1000 });
}

/// Prepara lo que pida la sesión (lo que hace el hilo preparador).
fn preparar(s: &mut Sesion) {
    while let Some(t) = s.siguiente_a_preparar() {
        let r = t.hacer();
        s.resultado_preparar(t.cue, r);
    }
}
