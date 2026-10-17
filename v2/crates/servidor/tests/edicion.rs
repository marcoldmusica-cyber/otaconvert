//! Etapa 5A: editar el show desde la ventana (agregar, quitar, mover,
//! numerar, inicio y fin, deshacer y rehacer) sin que lo que suena, lo
//! preparado o lo precargado quede apuntando al cue equivocado.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use otaconvert_audio::Comando;
use otaconvert_servidor::{AvisoProyector, Destino, Modo, OrdenProyector, Proyector, Sesion, Sonido};
use otaconvert_show::{guardar, Cue, Show, TipoCue};

static N: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Default)]
struct Proy {
    ordenes: Arc<Mutex<Vec<OrdenProyector>>>,
    avisos: Arc<Mutex<Vec<AvisoProyector>>>,
}
impl Proyector for Proy {
    fn enviar(&mut self, o: OrdenProyector) {
        self.ordenes.lock().unwrap().push(o);
    }
    fn recoger(&mut self) -> Vec<AvisoProyector> {
        std::mem::take(&mut *self.avisos.lock().unwrap())
    }
}
impl Proy {
    fn tomar(&self) -> Vec<OrdenProyector> {
        std::mem::take(&mut *self.ordenes.lock().unwrap())
    }
    fn avisar(&self, a: AvisoProyector) {
        self.avisos.lock().unwrap().push(a);
    }
}

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
    fn parados(&self) -> Vec<u32> {
        self.recibido.lock().unwrap().iter().filter_map(|(_, c)| if let Comando::Parar { id, .. } = c { Some(*id) } else { None }).collect()
    }
}

fn dato(n: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../audio/tests/datos").join(n)
}

/// Carpeta con 3 audios de verdad, un "vídeo" y una imagen (estos dos solo para la proyección falsa).
fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!("otaconvert-prueba-edicion-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("medios")).unwrap();
    for n in ["a.wav", "b.wav", "c.wav", "nuevo.wav"] {
        std::fs::copy(dato("tono.wav"), d.join("medios").join(n)).unwrap();
    }
    std::fs::write(d.join("medios/v.mp4"), b"no es un video").unwrap();
    std::fs::write(d.join("medios/foto.png"), b"no es una imagen").unwrap();
    std::fs::write(d.join("medios/notas.txt"), b"texto").unwrap();
    d
}

fn cue(n: &str, archivo: &str, tipo: TipoCue) -> Cue {
    Cue { numero: n.into(), nombre: format!("Cue {n}"), archivo: archivo.into(), tipo, ..Default::default() }
}

fn abrir(d: &Path) -> (Sesion, Proy, Son) {
    let show = Show {
        nombre: "Edición".into(),
        cues: vec![
            cue("1", "medios/a.wav", TipoCue::Audio),
            cue("2", "medios/v.mp4", TipoCue::Video),
            cue("3", "medios/b.wav", TipoCue::Audio),
            cue("4", "medios/c.wav", TipoCue::Audio),
        ],
        ..Default::default()
    };
    guardar(&show, &d.join("s.otashow"), "20261017-000000").unwrap();
    let mut s = Sesion::abrir(d).unwrap();
    let (p, son) = (Proy::default(), Son::default());
    s.conectar_sonido(Box::new(son.clone()));
    s.conectar_proyector(Box::new(p.clone()));
    son.recibido.lock().unwrap().clear();
    p.tomar();
    (s, p, son)
}

fn preparar_todo(s: &mut Sesion) {
    while let Some(t) = s.siguiente_a_preparar() {
        let r = t.hacer();
        s.resultado_preparar(t.cue, r);
    }
}

fn estado(s: &Sesion) -> serde_json::Value {
    serde_json::from_str(&s.estado_json()).unwrap()
}

fn nombres(s: &Sesion) -> Vec<String> {
    s.show.cues.iter().map(|c| c.nombre.clone()).collect()
}

#[test]
fn cada_cue_tiene_su_numero_interno_distinto() {
    let d = carpeta();
    let (s, _, _) = abrir(&d);
    let ids: Vec<u64> = s.show.cues.iter().map(|c| c.id).collect();
    assert_eq!(ids, vec![0, 1, 2, 3], "repetidos (todos en 0) → se numeran en orden");
}

#[test]
fn agregar_cues_de_la_carpeta() {
    let d = carpeta();
    let (mut s, _, _) = abrir(&d);
    s.ir_a(2);
    let nuevos = s.agregar_cues(&["medios/nuevo.wav".into(), "medios/foto.png".into()], Some(0)).unwrap();
    assert_eq!(nuevos, vec![1, 2]);
    assert_eq!(nombres(&s), vec!["Cue 1", "nuevo", "foto", "Cue 2", "Cue 3", "Cue 4"]);
    assert_eq!((s.show.cues[1].tipo, s.show.cues[2].tipo), (TipoCue::Audio, TipoCue::Imagen));
    assert_eq!((s.show.cues[1].numero.as_str(), s.show.cues[2].numero.as_str()), ("5", "6"), "números libres");
    assert_eq!(s.show.cues[1].id, 4, "número interno nuevo");
    assert_eq!(s.en_espera, 4, "el cue en espera sigue siendo el mismo (Cue 3)");
    assert!(s.sucio);
    // Lo que no se puede agregar no cambia nada.
    for malo in ["medios/notas.txt", "medios/no-existe.wav", "../afuera.wav", "C:/Windows/win.ini"] {
        assert!(s.agregar_cues(&[malo.into()], None).is_err(), "{malo}");
    }
    assert_eq!(s.show.cues.len(), 6);
    // En Función no se agrega.
    s.cambiar_modo(Modo::Funcion);
    assert!(s.agregar_cues(&["medios/nuevo.wav".into()], None).is_err());
}

#[test]
fn quitar_un_cue_suelta_lo_suyo_y_deja_en_espera_el_que_corresponde() {
    let d = carpeta();
    let (mut s, p, _) = abrir(&d);
    s.ir_a(1);
    s.siguiente_a_preparar();
    assert!(p.tomar().iter().any(|o| matches!(o, OrdenProyector::Precargar { id: 1, .. })));
    // Se quita el vídeo precargado (que estaba en espera): se suelta en la proyección.
    s.quitar_cue(1).unwrap();
    assert_eq!(nombres(&s), vec!["Cue 1", "Cue 3", "Cue 4"]);
    assert!(p.tomar().contains(&OrdenProyector::Quitar { id: 1 }));
    assert_eq!(s.en_espera, 1, "queda en espera el que vino a su lugar");
    // Quitar uno de antes del que está en espera: la espera sigue en el mismo cue.
    s.ir_a(2);
    s.quitar_cue(0).unwrap();
    assert_eq!(s.show.cues[s.en_espera].nombre, "Cue 4");
    // Lo que suena no se quita.
    s.ir_a(0);
    preparar_todo(&mut s);
    assert!(s.go());
    assert!(s.quitar_cue(0).is_err());
}

#[test]
fn mover_con_algo_sonando_y_cosas_preparadas_sigue_cada_cosa_en_su_cue() {
    let d = carpeta();
    let (mut s, p, son) = abrir(&d);
    preparar_todo(&mut s);
    assert!(s.go(), "suena el Cue 1");
    let voz = son.recibido.lock().unwrap().iter().find_map(|(_, c)| if let Comando::IniciarCue { id, .. } = c { Some(*id) } else { None }).unwrap();
    // El vídeo (en espera) se precarga y queda listo.
    p.avisar(AvisoProyector::Listo { id: 1, duracion: Some(5.0) });
    s.limpiar();
    assert_eq!(estado(&s)["cues"][1]["preparacion"], "listo");
    // Se mueve el Cue 1 (que suena) al final.
    s.mover_cue(0, 3).unwrap();
    assert_eq!(nombres(&s), vec!["Cue 2", "Cue 3", "Cue 4", "Cue 1"]);
    let e = estado(&s);
    assert_eq!(e["sonando"][0]["nombre"], "Cue 1", "lo que suena sigue siendo el Cue 1");
    assert_eq!(e["sonando"][0]["cue"], 3);
    assert_eq!(e["cues"][0]["preparacion"], "listo", "el vídeo sigue listo en su lugar nuevo");
    assert_eq!(s.show.cues[s.en_espera].nombre, "Cue 2", "la espera sigue en el vídeo");
    // Un aviso de la proyección con el número interno del vídeo llega a su lugar nuevo.
    p.avisar(AvisoProyector::Error { id: 1, motivo: otaconvert_servidor::motivo("abrir") });
    s.limpiar();
    assert_eq!(estado(&s)["cues"][0]["preparacion"], "error");
    // Parar todo para el sonido del Cue 1.
    s.parar_todo();
    assert!(estado(&s)["sonando"].as_array().unwrap().is_empty());
    let _ = voz;
}

#[test]
fn una_preparacion_vieja_se_descarta_y_se_vuelve_a_pedir() {
    let d = carpeta();
    let (mut s, _, _) = abrir(&d);
    let t = s.siguiente_a_preparar().unwrap();
    assert_eq!(t.cue, 0);
    // Mientras se prepara, cambia el orden.
    s.mover_cue(0, 2).unwrap();
    let r = t.hacer();
    s.resultado_preparar(t.cue, r);
    let e = estado(&s);
    // En el lugar 0 quedó el vídeo: si el audio viejo se hubiera guardado ahí,
    // tendría su duración (1 s).
    assert_eq!(e["cues"][0]["nombre"], "Cue 2");
    assert!(e["cues"][0]["duracion_s"].is_null(), "no quedó guardado en el lugar equivocado");
    preparar_todo(&mut s);
    let e = estado(&s);
    assert_eq!(e["cues"][2]["nombre"], "Cue 1");
    assert_eq!(e["cues"][2]["preparacion"], "listo", "se volvió a preparar en su lugar nuevo");
}

#[test]
fn deshacer_y_rehacer() {
    let d = carpeta();
    let (mut s, _, _) = abrir(&d);
    assert!(s.deshacer().is_err(), "al abrir no hay nada para deshacer");
    s.editar_cue(0, &serde_json::json!({ "nombre": "Sala" }), false).unwrap();
    s.mover_cue(3, 0).unwrap();
    s.quitar_cue(1).unwrap();
    assert_eq!(nombres(&s), vec!["Cue 4", "Cue 2", "Cue 3"]);
    s.deshacer().unwrap();
    assert_eq!(nombres(&s), vec!["Cue 4", "Sala", "Cue 2", "Cue 3"]);
    s.deshacer().unwrap();
    assert_eq!(nombres(&s), vec!["Sala", "Cue 2", "Cue 3", "Cue 4"]);
    s.deshacer().unwrap();
    assert_eq!(nombres(&s), vec!["Cue 1", "Cue 2", "Cue 3", "Cue 4"]);
    s.rehacer().unwrap();
    assert_eq!(nombres(&s), vec!["Sala", "Cue 2", "Cue 3", "Cue 4"]);
    // Un cambio nuevo borra lo que se podía rehacer.
    s.editar_cue(1, &serde_json::json!({ "nota": "con la luz" }), false).unwrap();
    assert!(s.rehacer().is_err());
    // Arrastrar las esquinas (muchos cambios seguidos) se deshace de una vez.
    for k in 1..=5 {
        let x = 0.01 * k as f64;
        s.cambiar_esquinas([[x, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]).unwrap();
    }
    s.deshacer().unwrap();
    assert!(s.show.superficie.es_normal(), "las 5 juntas");
    // En Función no se deshace.
    s.cambiar_modo(Modo::Funcion);
    assert!(s.deshacer().is_err());
}

#[test]
fn numero_inicio_y_fin() {
    let d = carpeta();
    let (mut s, _, _) = abrir(&d);
    s.editar_cue(0, &serde_json::json!({ "numero": "1.5", "inicio_ms": 100, "fin_ms": 800 }), false).unwrap();
    let c = &s.show.cues[0];
    assert_eq!((c.numero.as_str(), c.inicio_ms, c.fin_ms), ("1.5", 100, Some(800)));
    s.editar_cue(0, &serde_json::json!({ "fin_ms": null }), false).unwrap();
    assert_eq!(s.show.cues[0].fin_ms, None);
    for malo in [
        serde_json::json!({ "numero": "" }),
        serde_json::json!({ "numero": "un número larguísimo de más de veinte" }),
        serde_json::json!({ "inicio_ms": 900, "fin_ms": 500 }),
        serde_json::json!({ "inicio_ms": -1 }),
    ] {
        assert!(s.editar_cue(0, &malo, false).is_err(), "{malo}");
    }
    assert_eq!(s.show.cues[0].numero, "1.5");
    // Cambiar el inicio de un audio preparado lo vuelve a preparar.
    preparar_todo(&mut s);
    assert_eq!(estado(&s)["cues"][0]["preparacion"], "listo");
    s.editar_cue(0, &serde_json::json!({ "inicio_ms": 200 }), false).unwrap();
    assert_eq!(estado(&s)["cues"][0]["preparacion"], "");
}

#[test]
fn quitar_lo_que_suena_no_se_puede_pero_deshacer_un_agregado_si() {
    let d = carpeta();
    let (mut s, _, son) = abrir(&d);
    s.agregar_cues(&["medios/nuevo.wav".into()], None).unwrap();
    assert_eq!(s.show.cues.len(), 5);
    s.deshacer().unwrap();
    assert_eq!(s.show.cues.len(), 4);
    assert!(son.parados().is_empty());
}
