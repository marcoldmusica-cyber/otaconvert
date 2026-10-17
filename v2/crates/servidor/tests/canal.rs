//! Pruebas del canal ventana ↔ motor: solo entra quien tiene el token, con
//! el Host y el Origin correctos; GO, doble GO, varias ventanas, guardado.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use otaconvert_servidor::{Servidor, Sesion};
use otaconvert_show::{guardar, Cue, Show};
use tungstenite::client::IntoClientRequest;
use tungstenite::Message;

static N: AtomicUsize = AtomicUsize::new(0);

fn show_de_prueba() -> (PathBuf, Arc<Mutex<Sesion>>, Servidor) {
    let d = std::env::temp_dir().join(format!("otaconvert-prueba-canal-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let mut show = Show { nombre: "Prueba <b>sin HTML</b>".into(), ..Default::default() };
    for i in 1..=4 {
        show.cues.push(Cue { id: i, numero: i.to_string(), nombre: format!("Cue {i}"), archivo: format!("audio/{i}.wav"), ..Default::default() });
    }
    show.ajustes.tiempo_minimo_entre_go_ms = 300;
    let ruta = d.join("prueba.otashow");
    guardar(&show, &ruta, "20261016-000000").unwrap();
    let sesion = Arc::new(Mutex::new(Sesion::abrir(&d).unwrap()));
    let srv = Servidor::iniciar(sesion.clone()).unwrap();
    (d, sesion, srv)
}

fn token(srv: &Servidor) -> String {
    srv.url().split("?t=").nth(1).unwrap().to_string()
}

/// Pedido HTTP crudo; devuelve la respuesta completa.
fn pedir(puerto: u16, pedido: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", puerto)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    s.write_all(pedido.as_bytes()).unwrap();
    let mut r = String::new();
    let _ = s.read_to_string(&mut r);
    r
}

fn get(puerto: u16, destino: &str, host: &str) -> String {
    pedir(puerto, &format!("GET {destino} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"))
}

#[test]
fn la_pagina_solo_con_token_y_host_correctos() {
    let (_d, _s, srv) = show_de_prueba();
    let p = srv.puerto;
    let host = format!("127.0.0.1:{p}");
    let t = token(&srv);
    assert_eq!(t.len(), 64);

    let ok = get(p, &format!("/?t={t}"), &host);
    assert!(ok.starts_with("HTTP/1.1 200"), "{ok}");
    assert!(ok.contains("Content-Security-Policy: default-src 'none'"));
    assert!(ok.contains(&format!("connect-src ws://127.0.0.1:{p}")));
    assert!(ok.contains("Referrer-Policy: no-referrer"));
    assert!(ok.contains("X-Frame-Options: DENY"));
    assert!(!ok.to_ascii_lowercase().contains("access-control-allow-origin"), "sin CORS");

    assert!(get(p, "/", &host).starts_with("HTTP/1.1 403"), "sin token");
    assert!(get(p, "/?t=0000", &host).starts_with("HTTP/1.1 403"), "token corto");
    let casi: String = t.chars().rev().collect();
    assert!(get(p, &format!("/?t={casi}"), &host).starts_with("HTTP/1.1 403"), "token equivocado");
    // DNS rebinding: un sitio malicioso que resuelve a 127.0.0.1 manda su propio Host.
    assert!(get(p, &format!("/?t={t}"), "malicioso.com").starts_with("HTTP/1.1 403"));
    assert!(get(p, &format!("/?t={t}"), &format!("localhost:{p}")).starts_with("HTTP/1.1 403"));
    assert!(get(p, &format!("/otra?t={t}"), &host).starts_with("HTTP/1.1 404"));
    let post = pedir(p, &format!("POST /?t={t} HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\n\r\n"));
    assert!(post.starts_with("HTTP/1.1 405"), "{post}");
}

#[test]
fn cabeceras_enormes_se_cortan() {
    let (_d, _s, srv) = show_de_prueba();
    let relleno = "X-Relleno: ".to_string() + &"a".repeat(10_000);
    let r = pedir(srv.puerto, &format!("GET / HTTP/1.1\r\n{relleno}\r\n\r\n"));
    assert!(r.starts_with("HTTP/1.1 431"), "{}", &r[..r.len().min(40)]);
}

fn conectar_ws(srv: &Servidor, origen: Option<&str>, tok: &str) -> Result<tungstenite::WebSocket<TcpStream>, String> {
    let p = srv.puerto;
    let mut req = format!("ws://127.0.0.1:{p}/ws?t={tok}").into_client_request().unwrap();
    if let Some(o) = origen {
        req.headers_mut().insert("Origin", o.parse().unwrap());
    }
    let s = TcpStream::connect(("127.0.0.1", p)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    tungstenite::client(req, s).map(|(ws, _)| ws).map_err(|e| e.to_string())
}

/// Lee estados hasta uno que cumpla `cond` (el motor también manda refrescos periódicos).
fn esperar(ws: &mut tungstenite::WebSocket<TcpStream>, cond: impl Fn(&serde_json::Value) -> bool) -> serde_json::Value {
    let fin = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        let e = leer_estado(ws);
        if cond(&e) {
            return e;
        }
        assert!(std::time::Instant::now() < fin, "no llegó el estado esperado; último: {e}");
    }
}

fn leer_estado(ws: &mut tungstenite::WebSocket<TcpStream>) -> serde_json::Value {
    loop {
        if let Message::Text(t) = ws.read().unwrap() {
            let v: serde_json::Value = serde_json::from_str(t.as_str()).unwrap();
            if v["tipo"] == "estado" {
                return v;
            }
        }
    }
}

#[test]
fn websocket_rechaza_origen_o_token_malos() {
    let (_d, _s, srv) = show_de_prueba();
    let p = srv.puerto;
    let t = token(&srv);
    let bueno = format!("http://127.0.0.1:{p}");
    assert!(conectar_ws(&srv, Some("https://pagina-maliciosa.com"), &t).is_err(), "otro origen");
    assert!(conectar_ws(&srv, Some(&format!("http://localhost:{p}")), &t).is_err(), "localhost no es el origen");
    assert!(conectar_ws(&srv, None, &t).is_err(), "sin origen");
    assert!(conectar_ws(&srv, Some(&bueno), "malo").is_err(), "token malo");
    assert!(conectar_ws(&srv, Some(&bueno), &t).is_ok());
}

#[test]
fn go_avanza_y_protege_contra_doble_go() {
    let (d, _s, srv) = show_de_prueba();
    let t = token(&srv);
    let mut ws = conectar_ws(&srv, Some(&format!("http://127.0.0.1:{}", srv.puerto)), &t).unwrap();
    let e = leer_estado(&mut ws);
    assert_eq!(e["show"], "Prueba <b>sin HTML</b>");
    assert_eq!(e["cues"].as_array().unwrap().len(), 4);
    assert_eq!(e["en_espera"], 0);

    ws.send(Message::text(r#"{"orden":"go"}"#)).unwrap();
    let e = esperar(&mut ws, |e| e["en_espera"] == 1);
    assert_eq!(e["ultimo"], 0);

    // Segundo GO enseguida: ignorado.
    ws.send(Message::text(r#"{"orden":"go"}"#)).unwrap();
    let e = esperar(&mut ws, |e| e["aviso"].as_str().is_some_and(|a| a.contains("doble GO")));
    assert_eq!(e["en_espera"], 1, "el doble GO no debe disparar");

    std::thread::sleep(Duration::from_millis(350));
    ws.send(Message::text(r#"{"orden":"go"}"#)).unwrap();
    esperar(&mut ws, |e| e["en_espera"] == 2);

    ws.send(Message::text(r#"{"orden":"espera_arriba"}"#)).unwrap();
    esperar(&mut ws, |e| e["en_espera"] == 1);
    ws.send(Message::text("esto no es JSON")).unwrap();
    esperar(&mut ws, |e| e["aviso"] == "Orden desconocida.");

    // El registro de la función anotó los GO y el ignorado.
    let reg = std::fs::read_dir(d.join("registros")).unwrap().next().unwrap().unwrap().path();
    let texto = std::fs::read_to_string(reg).unwrap();
    assert_eq!(texto.matches("GO ").count(), 2, "{texto}");
    assert!(texto.contains("IGNORADO"));
}

#[test]
fn varias_ventanas_ven_lo_mismo() {
    let (_d, _s, srv) = show_de_prueba();
    let t = token(&srv);
    let o = format!("http://127.0.0.1:{}", srv.puerto);
    let mut a = conectar_ws(&srv, Some(&o), &t).unwrap();
    let mut b = conectar_ws(&srv, Some(&o), &t).unwrap();
    leer_estado(&mut a);
    leer_estado(&mut b);
    a.send(Message::text(r#"{"orden":"go"}"#)).unwrap();
    esperar(&mut a, |e| e["en_espera"] == 1);
    esperar(&mut b, |e| e["en_espera"] == 1); // la otra ventana también se entera
}

#[test]
fn mensaje_gigante_cierra_la_conexion() {
    let (_d, s, srv) = show_de_prueba();
    let t = token(&srv);
    let mut ws = conectar_ws(&srv, Some(&format!("http://127.0.0.1:{}", srv.puerto)), &t).unwrap();
    leer_estado(&mut ws);
    let _ = ws.send(Message::text("x".repeat(200_000)));
    // El motor corta esa conexión pero sigue vivo para las demás.
    let mut cerrada = false;
    for _ in 0..20 {
        match ws.read() {
            Ok(Message::Close(_)) | Err(_) => {
                cerrada = true;
                break;
            }
            Ok(_) => {}
        }
    }
    assert!(cerrada);
    assert_eq!(s.lock().unwrap().en_espera, 0);
    assert!(conectar_ws(&srv, Some(&format!("http://127.0.0.1:{}", srv.puerto)), &t).is_ok());
}

#[test]
fn guardado_automatico_solo_si_hay_cambios() {
    let (d, s, _srv) = show_de_prueba();
    let ruta = d.join("prueba.otashow");
    let antes = std::fs::metadata(&ruta).unwrap().modified().unwrap();
    s.lock().unwrap().guardar_si_hace_falta();
    assert_eq!(std::fs::metadata(&ruta).unwrap().modified().unwrap(), antes, "sin cambios no guarda");
    {
        let mut g = s.lock().unwrap();
        g.show.nombre = "Cambiado".into();
        g.sucio = true;
        g.guardar_si_hace_falta();
        assert!(!g.sucio);
    }
    assert_eq!(otaconvert_show::abrir(&ruta).unwrap().nombre, "Cambiado");
}

/// Vista Estudio: la ventana pide la foto y el archivo de un cue de vídeo o
/// imagen por su número. Solo con token y Host correctos, solo cues visuales
/// del show, y el vídeo por partes (Range) sin cargarlo entero.
#[test]
fn miniatura_y_medio_solo_de_cues_del_show() {
    let d = std::env::temp_dir().join(format!("otaconvert-prueba-medios-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("video")).unwrap();
    let datos: Vec<u8> = (0..100u8).collect();
    std::fs::write(d.join("video/a.mp4"), &datos).unwrap();
    std::fs::write(d.join("secreto.txt"), b"no se entrega").unwrap();
    let show = Show {
        nombre: "Medios".into(),
        cues: vec![
            Cue { id: 1, numero: "1".into(), nombre: "Audio".into(), archivo: "audio/x.wav".into(), ..Default::default() },
            Cue { id: 2, numero: "2".into(), nombre: "Vídeo".into(), archivo: "video/a.mp4".into(), tipo: otaconvert_show::TipoCue::Video, ..Default::default() },
        ],
        ..Default::default()
    };
    guardar(&show, &d.join("m.otashow"), "20261016-000000").unwrap();
    let sesion = Arc::new(Mutex::new(Sesion::abrir(&d).unwrap()));
    let srv = Servidor::iniciar(sesion.clone()).unwrap();
    let (p, t) = (srv.puerto, token(&srv));
    let host = format!("127.0.0.1:{p}");
    let pide = |destino: &str, host: &str, extra: &str| pedir(p, &format!("GET {destino} HTTP/1.1\r\nHost: {host}\r\n{extra}Connection: close\r\n\r\n"));

    // Sin token, con token malo o con otro Host: nada.
    assert!(pide("/medio?cue=1", &host, "").starts_with("HTTP/1.1 403"));
    assert!(pide("/medio?t=00&cue=1", &host, "").starts_with("HTTP/1.1 403"));
    assert!(pide(&format!("/medio?t={t}&cue=1"), "malicioso.com", "").starts_with("HTTP/1.1 403"));
    assert!(pide(&format!("/miniatura?t={t}&cue=1"), "localhost", "").starts_with("HTTP/1.1 403"));
    // Un audio, un cue que no existe, una ruta pedida por la ventana o una foto que todavía no está: 404.
    assert!(pide(&format!("/medio?t={t}&cue=0"), &host, "").starts_with("HTTP/1.1 404"));
    assert!(pide(&format!("/medio?t={t}&cue=99"), &host, "").starts_with("HTTP/1.1 404"));
    assert!(pide(&format!("/medio?t={t}&cue=../secreto.txt"), &host, "").starts_with("HTTP/1.1 404"));
    assert!(pide(&format!("/medio?t={t}&archivo=secreto.txt"), &host, "").starts_with("HTTP/1.1 404"));
    assert!(pide(&format!("/miniatura?t={t}&cue=1"), &host, "").starts_with("HTTP/1.1 404"));

    // El vídeo entero y por partes.
    let r = pide(&format!("/medio?t={t}&cue=1"), &host, "");
    assert!(r.starts_with("HTTP/1.1 200") && r.contains("Content-Type: video/mp4") && r.contains("Content-Length: 100") && r.contains("Accept-Ranges: bytes"), "{r}");
    let mut s = TcpStream::connect(("127.0.0.1", p)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    s.write_all(format!("GET /medio?t={t}&cue=1 HTTP/1.1\r\nHost: {host}\r\nRange: bytes=10-19\r\nConnection: close\r\n\r\n").as_bytes()).unwrap();
    let mut crudo = Vec::new();
    let _ = s.read_to_end(&mut crudo);
    let fin = crudo.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    let cab = String::from_utf8_lossy(&crudo[..fin]);
    assert!(cab.starts_with("HTTP/1.1 206") && cab.contains("Content-Range: bytes 10-19/100"), "{cab}");
    assert_eq!(&crudo[fin..], &datos[10..20]);
    assert!(pide(&format!("/medio?t={t}&cue=1"), &host, "Range: bytes=500-600\r\n").starts_with("HTTP/1.1 416"));

    // La página permite cargar imágenes y vídeo solo del propio motor.
    let pag = pide(&format!("/?t={t}"), &host, "");
    assert!(pag.contains("img-src 'self' data:") && pag.contains("media-src 'self'"));
    let _ = std::fs::remove_dir_all(&d);
}

/// Etapa 5A: archivos arrastrados desde fuera de la carpeta del show. Se copian
/// a `Medios/` (sin pisar uno distinto con el mismo nombre), quedan como cues
/// detrás del que está en espera, y lo que no es un medio se rechaza sin tocar nada.
#[test]
fn arrastrar_archivos_de_afuera_los_copia_y_agrega() {
    let (d, sesion, srv) = show_de_prueba();
    let t = token(&srv);
    let origen = format!("http://127.0.0.1:{}", srv.puerto);
    let mut ws = conectar_ws(&srv, Some(&origen), &t).unwrap();
    let _ = leer_estado(&mut ws);

    let afuera = d.with_extension("afuera");
    let _ = std::fs::remove_dir_all(&afuera);
    std::fs::create_dir_all(&afuera).unwrap();
    std::fs::write(afuera.join("Timbre.wav"), b"RIFF datos de prueba").unwrap();
    std::fs::write(afuera.join("notas.txt"), b"no es un medio").unwrap();
    // Uno distinto con el mismo nombre ya en Medios: no se pisa.
    std::fs::create_dir_all(d.join("Medios")).unwrap();
    std::fs::write(d.join("Medios/Timbre.wav"), b"otro contenido").unwrap();

    // Lo que no es un medio: aviso y nada cambia.
    let importar = |rutas: &[std::path::PathBuf], despues_de: Option<usize>| {
        let rutas: Vec<String> = rutas.iter().map(|r| r.to_string_lossy().into_owned()).collect();
        Message::text(serde_json::json!({ "orden": "importar", "rutas": rutas, "despues_de": despues_de }).to_string())
    };
    ws.send(importar(&[afuera.join("notas.txt")], Some(0))).unwrap();
    let e = esperar(&mut ws, |e| e["aviso"].as_str().is_some_and(|a| a.contains("notas.txt")));
    assert_eq!(e["cues"].as_array().unwrap().len(), 4);

    let wav = afuera.join("Timbre.wav");
    ws.send(importar(&[wav.clone()], Some(0))).unwrap();
    let e = esperar(&mut ws, |e| e["cues"].as_array().unwrap().len() == 5);
    let nuevo = &e["cues"][1];
    assert_eq!(nuevo["nombre"], "Timbre (1)");
    assert_eq!(nuevo["archivo"], "Medios/Timbre (1).wav");
    assert_eq!(std::fs::read(d.join("Medios/Timbre (1).wav")).unwrap(), b"RIFF datos de prueba");
    assert_eq!(std::fs::read(d.join("Medios/Timbre.wav")).unwrap(), b"otro contenido", "no se pisa");
    // El original sigue donde estaba y no quedan temporales.
    assert!(afuera.join("Timbre.wav").is_file());
    assert!(!std::fs::read_dir(d.join("Medios")).unwrap().flatten().any(|f| f.file_name().to_string_lossy().contains("copiando")));

    // Se puede deshacer (el archivo copiado queda en la carpeta).
    ws.send(Message::text(r#"{"orden":"deshacer"}"#)).unwrap();
    let e = esperar(&mut ws, |e| e["cues"].as_array().unwrap().len() == 4);
    assert_eq!(e["puede_rehacer"], true);

    // En Función no se agrega nada.
    sesion.lock().unwrap().cambiar_modo(otaconvert_servidor::Modo::Funcion);
    ws.send(importar(&[wav], None)).unwrap();
    let e = esperar(&mut ws, |e| e["aviso"].as_str().is_some_and(|a| a.contains("Función")));
    assert_eq!(e["cues"].as_array().unwrap().len(), 4);
    let _ = std::fs::remove_dir_all(&afuera);
}
