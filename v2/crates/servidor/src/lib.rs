//! Canal seguro entre la ventana y el motor (regla de seguridad 1 de CLAUDE.md).
//!
//! - Escucha **solo en 127.0.0.1**, en un puerto al azar: nada de la red llega.
//! - **Token secreto** nuevo en cada arranque (32 bytes del generador del
//!   sistema): sin él no se sirve la página ni se acepta el WebSocket.
//! - **Host** debe ser `127.0.0.1:<puerto>` (contra *DNS rebinding*) y el
//!   **Origin** del WebSocket `http://127.0.0.1:<puerto>` (contra páginas web
//!   maliciosas abiertas en el navegador). Sin CORS.
//! - Límites: 16 conexiones, cabeceras de 8 KB, mensajes de 64 KB, 5 s para
//!   mandar el pedido.

pub mod sesion;

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Error as ErrorWs, Message};

pub use sesion::Sesion;

const PAGINA: &str = include_str!("../ui/index.html");
const MAX_CONEXIONES: usize = 16;
const MAX_CABECERAS: usize = 8 * 1024;
const MAX_MENSAJE: usize = 64 * 1024;
const ESPERA_PEDIDO: Duration = Duration::from_secs(5);
const GUARDADO_AUTOMATICO: Duration = Duration::from_secs(30);

struct Contexto {
    puerto: u16,
    token: String,
    sesion: Arc<Mutex<Sesion>>,
    clientes: Mutex<Vec<Sender<String>>>,
    conexiones: AtomicUsize,
}

pub struct Servidor {
    pub puerto: u16,
    token: String,
}

/// Token de 32 bytes al azar, en hexadecimal.
fn nuevo_token() -> io::Result<String> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Compara sin cortar en la primera diferencia (no revela cuánto acertó).
fn iguales(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn token_de(consulta: Option<&str>) -> Option<&str> {
    consulta?.split('&').find_map(|p| p.strip_prefix("t="))
}

impl Servidor {
    /// Arranca el servidor y el guardado automático. Vive mientras viva el proceso.
    pub fn iniciar(sesion: Arc<Mutex<Sesion>>) -> io::Result<Self> {
        let escucha = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
        let puerto = escucha.local_addr()?.port();
        let token = nuevo_token()?;
        let ctx = Arc::new(Contexto {
            puerto,
            token: token.clone(),
            sesion: sesion.clone(),
            clientes: Mutex::new(Vec::new()),
            conexiones: AtomicUsize::new(0),
        });
        let c = ctx.clone();
        std::thread::Builder::new().name("servidor".into()).spawn(move || {
            for entrante in escucha.incoming() {
                let Ok(stream) = entrante else { continue };
                if c.conexiones.fetch_add(1, Ordering::SeqCst) >= MAX_CONEXIONES {
                    c.conexiones.fetch_sub(1, Ordering::SeqCst);
                    continue; // se cierra al soltarla
                }
                let c2 = c.clone();
                let _ = std::thread::Builder::new().name("conexion".into()).spawn(move || {
                    atender(stream, &c2);
                    c2.conexiones.fetch_sub(1, Ordering::SeqCst);
                });
            }
        })?;
        std::thread::Builder::new().name("guardado-automatico".into()).spawn(move || loop {
            std::thread::sleep(GUARDADO_AUTOMATICO);
            if let Ok(mut s) = sesion.lock() {
                s.guardar_si_hace_falta();
            }
        })?;
        Ok(Self { puerto, token })
    }

    /// Dirección que abre la ventana (o el navegador).
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/?t={}", self.puerto, self.token)
    }
}

fn atender(stream: TcpStream, ctx: &Contexto) {
    let _ = stream.set_read_timeout(Some(ESPERA_PEDIDO));
    let _ = stream.set_nodelay(true);
    let mut vista = [0u8; 16];
    let n = stream.peek(&mut vista).unwrap_or(0);
    if vista[..n].starts_with(b"GET /ws") {
        websocket(stream, ctx);
    } else {
        http(stream, ctx);
    }
}

fn responder(mut s: &TcpStream, estado: &str, extra: &str, cuerpo: &str) {
    let _ = write!(
        s,
        "HTTP/1.1 {estado}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nConnection: close\r\n{extra}\r\n{cuerpo}",
        cuerpo.len()
    );
    let _ = s.flush();
}

fn http(mut stream: TcpStream, ctx: &Contexto) {
    // Leer las cabeceras hasta la línea vacía, sin pasar de 8 KB.
    let mut buf = Vec::with_capacity(1024);
    let mut trozo = [0u8; 1024];
    loop {
        match stream.read(&mut trozo) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&trozo[..n]),
        }
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > MAX_CABECERAS {
            responder(&stream, "431 Request Header Fields Too Large", "", "");
            return;
        }
    }
    let texto = String::from_utf8_lossy(&buf);
    let mut lineas = texto.split("\r\n");
    let primera = lineas.next().unwrap_or("");
    let mut partes = primera.split(' ');
    let (metodo, destino) = (partes.next().unwrap_or(""), partes.next().unwrap_or(""));
    let host = lineas
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("host"))
        .map(|(_, v)| v.trim().to_string());
    if metodo != "GET" {
        responder(&stream, "405 Method Not Allowed", "Allow: GET\r\n", "");
        return;
    }
    if host.as_deref() != Some(&format!("127.0.0.1:{}", ctx.puerto)) {
        responder(&stream, "403 Forbidden", "", "<p>Acceso no permitido.</p>");
        return;
    }
    let (ruta, consulta) = match destino.split_once('?') {
        Some((r, q)) => (r, Some(q)),
        None => (destino, None),
    };
    if ruta != "/" {
        responder(&stream, "404 Not Found", "", "");
        return;
    }
    match token_de(consulta) {
        Some(t) if iguales(t, &ctx.token) => {
            let csp = format!(
                "Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src ws://127.0.0.1:{}; img-src data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\r\n",
                ctx.puerto
            );
            responder(&stream, "200 OK", &csp, PAGINA);
        }
        _ => responder(&stream, "403 Forbidden", "", "<p>Abre Otaconvert desde el propio programa.</p>"),
    }
}

fn difundir(ctx: &Contexto, texto: String) {
    if let Ok(mut c) = ctx.clientes.lock() {
        c.retain(|tx| tx.send(texto.clone()).is_ok());
    }
}

fn ejecutar(ctx: &Contexto, texto: &str) {
    let orden = serde_json::from_str::<serde_json::Value>(texto)
        .ok()
        .and_then(|v| v.get("orden").and_then(|o| o.as_str()).map(str::to_string));
    let estado = {
        let Ok(mut s) = ctx.sesion.lock() else { return };
        match orden.as_deref() {
            Some("go") => {
                s.go();
            }
            Some("parar_todo") => s.parar_todo(),
            Some("espera_arriba") => s.mover_espera(-1),
            Some("espera_abajo") => s.mover_espera(1),
            Some("guardar") => {
                if let Err(e) = s.guardar() {
                    s.aviso = Some(format!("No se pudo guardar: {e}"));
                }
            }
            Some("estado") => {}
            _ => {
                s.aviso = Some("Orden desconocida.".into());
            }
        }
        s.estado_json()
    };
    difundir(ctx, estado);
}

fn websocket(stream: TcpStream, ctx: &Contexto) {
    let puerto = ctx.puerto;
    let token = ctx.token.clone();
    let revisar = move |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
        let cab = |n: &str| req.headers().get(n).and_then(|v| v.to_str().ok());
        let host_ok = cab("host") == Some(&format!("127.0.0.1:{puerto}"));
        let origen_ok = cab("origin") == Some(&format!("http://127.0.0.1:{puerto}"));
        let token_ok = token_de(req.uri().query()).is_some_and(|t| iguales(t, &token));
        if req.uri().path() == "/ws" && host_ok && origen_ok && token_ok {
            Ok(resp)
        } else {
            let mut e = ErrorResponse::new(Some("Acceso no permitido".into()));
            *e.status_mut() = tungstenite::http::StatusCode::FORBIDDEN;
            Err(e)
        }
    };
    let config = WebSocketConfig::default().max_message_size(Some(MAX_MENSAJE)).max_frame_size(Some(MAX_MENSAJE));
    let Ok(mut ws) = tungstenite::accept_hdr_with_config(stream, revisar, Some(config)) else { return };
    // Lecturas cortas para poder enviar los cambios que lleguen de otras ventanas.
    let _ = ws.get_ref().set_read_timeout(Some(Duration::from_millis(100)));
    let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
    if let Ok(mut c) = ctx.clientes.lock() {
        c.push(tx);
    }
    let inicial = ctx.sesion.lock().map(|s| s.estado_json()).unwrap_or_default();
    if ws.send(Message::text(inicial)).is_err() {
        return;
    }
    loop {
        match ws.read() {
            Ok(Message::Text(t)) => ejecutar(ctx, t.as_str()),
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(ErrorWs::Io(e)) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {}
            Err(_) => break,
        }
        while let Ok(m) = rx.try_recv() {
            if ws.send(Message::text(m)).is_err() {
                return;
            }
        }
    }
}
