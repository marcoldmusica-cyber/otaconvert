//! Etapa 3A: la ventana propia del programa (WebView2 con wry + tao).
//!
//! La ventana es un **proceso aparte** (`otaconvert-motor --ventana`): si se
//! cuelga o se cae, el sonido del motor sigue. El motor le pasa la dirección
//! con el token por la entrada estándar (no por la línea de comandos, que otros
//! programas pueden leer) y vigila el proceso:
//! - termina con 0: el usuario cerró la ventana → el programa termina;
//! - termina con `SIN_WEBVIEW2`: no se pudo crear WebView2 → se abre el navegador;
//! - cualquier otra cosa (caída): se vuelve a abrir la ventana; tras varias
//!   caídas seguidas, el navegador.
//!
//! Seguridad: solo navega a `http://127.0.0.1:<puerto>/` del motor; nada de
//! ventanas nuevas, descargas, herramientas de desarrollo ni atajos del
//! navegador (F5, Ctrl+P, Ctrl+F…).

use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Child, Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

/// Código de salida de la ventana cuando WebView2 no se puede usar.
pub const SIN_WEBVIEW2: u8 = 3;
/// Caídas de la ventana permitidas en `VENTANA_CAIDAS` antes de pasar al navegador.
const MAX_CAIDAS: usize = 3;
const VENTANA_CAIDAS: Duration = Duration::from_secs(60);

/// Carpeta de datos del programa: junto al exe si hay `portable.txt`
/// (versión portable), si no `%LOCALAPPDATA%\Otaconvert`.
pub fn carpeta_datos() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    if exe_dir.join("portable.txt").is_file() {
        return Some(exe_dir.join("datos"));
    }
    std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Otaconvert"))
}

/// Comprueba que la dirección sea la del motor: `http://127.0.0.1:<puerto>/?t=<hex>`.
/// Devuelve el origen (`http://127.0.0.1:<puerto>`).
fn origen_valido(url: &str) -> Option<String> {
    let resto = url.strip_prefix("http://127.0.0.1:")?;
    let (puerto, consulta) = resto.split_once("/?t=")?;
    let p: u16 = puerto.parse().ok()?;
    if p == 0 || consulta.len() != 64 || !consulta.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("http://127.0.0.1:{p}"))
}

/// Proceso de la ventana. Lee la dirección de la entrada estándar.
#[cfg(windows)]
pub fn ejecutar() -> ExitCode {
    use tao::dpi::LogicalSize;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::platform::windows::IconExtWindows;
    use tao::window::{Icon, WindowBuilder};
    use wry::{WebContext, WebViewBuilder, WebViewBuilderExtWindows};

    let mut url = String::new();
    if std::io::stdin().lock().read_line(&mut url).is_err() {
        return ExitCode::from(2);
    }
    let url = url.trim().to_string();
    let Some(origen) = origen_valido(&url) else {
        eprintln!("ventana: dirección no válida");
        return ExitCode::from(2);
    };

    let event_loop = EventLoop::new();
    let icono = Icon::from_resource(1, None).ok();
    let window = match WindowBuilder::new()
        .with_title("Otaconvert")
        .with_window_icon(icono)
        .with_inner_size(LogicalSize::new(1280.0, 760.0))
        .with_min_inner_size(LogicalSize::new(720.0, 480.0))
        .with_maximized(true)
        .build(&event_loop)
    {
        Ok(w) => w,
        Err(e) => {
            eprintln!("ventana: no se pudo crear la ventana: {e}");
            return ExitCode::from(SIN_WEBVIEW2);
        }
    };

    // Los datos de WebView2 (caché) van a nuestra carpeta de datos, nunca
    // junto al exe (que puede estar en Archivos de programa).
    let mut contexto = WebContext::new(carpeta_datos().map(|d| d.join("WebView2")));
    let permitido = format!("{origen}/");
    let webview = WebViewBuilder::new_with_web_context(&mut contexto)
        .with_url(&url)
        .with_background_color((0x0E, 0x11, 0x17, 0xFF))
        .with_devtools(cfg!(debug_assertions))
        .with_navigation_handler(move |destino| destino.starts_with(&permitido))
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .with_download_started_handler(|_, _| false)
        // Cámara, micrófono, ubicación, notificaciones…: nada.
        .with_permission_handler(|_| wry::PermissionResponse::Deny)
        .with_browser_accelerator_keys(false)
        .with_default_context_menus(false)
        .with_focused(true)
        .build(&window);
    let webview = match webview {
        Ok(w) => w,
        Err(e) => {
            eprintln!("ventana: WebView2 no está disponible: {e}");
            return ExitCode::from(SIN_WEBVIEW2);
        }
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _ = &webview;
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::ExitWithCode(0);
        }
    })
}

#[cfg(not(windows))]
pub fn ejecutar() -> ExitCode {
    ExitCode::from(SIN_WEBVIEW2)
}

/// Qué pasó con la ventana, visto desde el motor.
pub enum Estado {
    /// Sigue abierta.
    Abierta,
    /// El usuario la cerró: hay que terminar el programa.
    Cerrada,
    /// No se puede usar la ventana propia: hay que abrir el navegador.
    UsarNavegador(String),
}

/// La ventana vista desde el motor: la abre, la vigila y la reabre si se cae.
pub struct Ventana {
    url: String,
    hijo: Child,
    caidas: Vec<Instant>,
}

fn lanzar(url: &str) -> Result<Child, String> {
    use std::io::Write;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut hijo = Command::new(exe)
        .arg("--ventana")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("no se pudo abrir la ventana: {e}"))?;
    if let Some(mut entrada) = hijo.stdin.take() {
        let _ = writeln!(entrada, "{url}");
    }
    Ok(hijo)
}

impl Ventana {
    pub fn abrir(url: &str) -> Result<Self, String> {
        Ok(Ventana { url: url.to_string(), hijo: lanzar(url)?, caidas: Vec::new() })
    }

    /// Se llama cada medio segundo desde el bucle del motor.
    pub fn revisar(&mut self) -> Estado {
        let codigo = match self.hijo.try_wait() {
            Ok(None) => return Estado::Abierta,
            Ok(Some(s)) => s.code(),
            Err(_) => None,
        };
        match codigo {
            Some(0) => Estado::Cerrada,
            Some(c) if c == SIN_WEBVIEW2 as i32 => Estado::UsarNavegador(
                "No se pudo abrir la ventana de Otaconvert (falta WebView2 o no funciona). Se abrió el navegador en su lugar.".into(),
            ),
            _ => {
                let ahora = Instant::now();
                self.caidas.retain(|t| ahora.duration_since(*t) < VENTANA_CAIDAS);
                self.caidas.push(ahora);
                if self.caidas.len() > MAX_CAIDAS {
                    return Estado::UsarNavegador(
                        "La ventana de Otaconvert se cerró sola varias veces. Se abrió el navegador en su lugar; el sonido no se interrumpió.".into(),
                    );
                }
                match lanzar(&self.url) {
                    Ok(h) => {
                        self.hijo = h;
                        Estado::Abierta
                    }
                    Err(e) => Estado::UsarNavegador(e),
                }
            }
        }
    }
}

impl Drop for Ventana {
    fn drop(&mut self) {
        if let Ok(None) = self.hijo.try_wait() {
            let _ = self.hijo.kill();
            let _ = self.hijo.wait();
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::origen_valido;

    #[test]
    fn solo_la_direccion_del_motor() {
        let t = "a".repeat(64);
        assert_eq!(origen_valido(&format!("http://127.0.0.1:5000/?t={t}")).as_deref(), Some("http://127.0.0.1:5000"));
        for malo in [
            format!("http://localhost:5000/?t={t}"),
            format!("https://127.0.0.1:5000/?t={t}"),
            format!("http://127.0.0.1:0/?t={t}"),
            format!("http://127.0.0.1:99999/?t={t}"),
            format!("http://127.0.0.1:5000/?t={}", "a".repeat(63)),
            format!("http://127.0.0.1:5000/?t={}", "z".repeat(64)),
            format!("http://127.0.0.1:5000/otra?t={t}"),
            format!("http://127.0.0.1.malo.com:5000/?t={t}"),
            String::new(),
        ] {
            assert!(origen_valido(&malo).is_none(), "{malo}");
        }
    }
}
