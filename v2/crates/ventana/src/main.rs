//! `otaconvert-ventana.exe`: la ventana propia del programa (WebView2 con
//! wry + tao). La abre y la vigila el motor (`crates/motor/src/ventana_ui.rs`).
//!
//! Es un exe aparte: si se cuelga o se cae, el sonido del motor sigue; y
//! contiene el cargador de WebView2 de Microsoft sin mezclarlo con piezas GPL
//! de terceros (ver `LICENSE-EXCEPTION-WEBVIEW2.txt`).
//!
//! Lee la dirección con el token por la entrada estándar (no por la línea de
//! comandos, que otros programas pueden leer). Códigos de salida: 0 = el
//! usuario la cerró; 2 = dirección no válida; 3 = WebView2 no se puede usar.
//!
//! Seguridad: solo navega a `http://127.0.0.1:<puerto>/` del motor; nada de
//! ventanas nuevas, descargas, permisos, herramientas de desarrollo ni atajos
//! del navegador (F5, Ctrl+P, Ctrl+F…).

// Sin ventana de consola en la versión final.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

/// Código de salida cuando WebView2 no se puede usar (lo conoce el motor).
const SIN_WEBVIEW2: u8 = 3;
const DIRECCION_MALA: u8 = 2;

/// Comprueba que la dirección sea la del motor: `http://127.0.0.1:<puerto>/?t=<64 hex>`.
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

/// Carpeta de datos del programa: junto al exe si hay `portable.txt`
/// (versión portable), si no `%LOCALAPPDATA%\Otaconvert`.
fn carpeta_datos() -> Option<std::path::PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    if exe_dir.join("portable.txt").is_file() {
        return Some(exe_dir.join("datos"));
    }
    std::env::var_os("LOCALAPPDATA").map(|d| std::path::PathBuf::from(d).join("Otaconvert"))
}

#[cfg(windows)]
fn main() -> ExitCode {
    use std::io::BufRead;
    use tao::dpi::LogicalSize;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tao::platform::windows::IconExtWindows;
    use tao::window::{Icon, WindowBuilder};
    use windows_sys::Win32::System::LibraryLoader::{SetDefaultDllDirectories, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS};
    use wry::{WebContext, WebViewBuilder, WebViewBuilderExtWindows};

    // DLL cargadas después de arrancar: solo de la carpeta del programa y de
    // System32 (igual que el motor). WebView2 carga su runtime con ruta completa.
    // SAFETY: llamada sin punteros; solo cambia la política del proceso.
    unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS) };

    let mut url = String::new();
    if std::io::stdin().lock().read_line(&mut url).is_err() {
        return ExitCode::from(DIRECCION_MALA);
    }
    let url = url.trim().to_string();
    let Some(origen) = origen_valido(&url) else {
        eprintln!("ventana: dirección no válida");
        return ExitCode::from(DIRECCION_MALA);
    };

    // Eventos propios: el nuevo título cuando la página lo cambia (nombre del
    // show) y los archivos que se arrastran desde el Explorador (etapa 5A).
    let event_loop = EventLoopBuilder::<Propio>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let proxy_soltar = event_loop.create_proxy();
    let window = match WindowBuilder::new()
        .with_title("Otaconvert")
        .with_window_icon(Icon::from_resource(1, None).ok())
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
        .with_document_title_changed_handler(move |t| {
            let t: String = t.chars().filter(|c| !c.is_control()).take(200).collect();
            let _ = proxy.send_event(Propio::Titulo(t));
        })
        // Archivos arrastrados desde el Explorador: la página los manda al motor
        // (que solo acepta audios, vídeos e imágenes de este equipo). `true` =
        // WebView2 no abre el archivo por su cuenta.
        .with_drag_drop_handler(move |ev| {
            let _ = match ev {
                wry::DragDropEvent::Enter { .. } => proxy_soltar.send_event(Propio::Arrastrando(true)),
                wry::DragDropEvent::Leave => proxy_soltar.send_event(Propio::Arrastrando(false)),
                wry::DragDropEvent::Drop { paths, .. } => proxy_soltar.send_event(Propio::Soltar(paths)),
                _ => Ok(()),
            };
            true
        })
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
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::ExitWithCode(0),
            Event::UserEvent(Propio::Titulo(titulo)) if !titulo.is_empty() => window.set_title(&titulo),
            Event::UserEvent(Propio::Arrastrando(si)) => {
                let _ = webview.evaluate_script(&format!("window.otaArrastre && window.otaArrastre({si})"));
            }
            Event::UserEvent(Propio::Soltar(rutas)) => {
                let lista: Vec<String> = rutas.iter().take(1000).map(|r| texto_json(&r.to_string_lossy())).collect();
                let _ = webview.evaluate_script(&format!("window.otaSoltar && window.otaSoltar([{}])", lista.join(",")));
            }
            _ => {}
        }
    })
}

/// Eventos propios del bucle de la ventana.
#[cfg(windows)]
enum Propio {
    Titulo(String),
    Arrastrando(bool),
    Soltar(Vec<std::path::PathBuf>),
}

/// Texto como cadena de JavaScript/JSON, con todo lo especial escapado
/// (un nombre de archivo nunca puede cortar el código que se ejecuta).
fn texto_json(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' || c == '<' || c == '>' => {
                o.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

#[cfg(not(windows))]
fn main() -> ExitCode {
    let _ = texto_json("");
    let _ = (origen_valido(""), carpeta_datos());
    ExitCode::from(SIN_WEBVIEW2)
}

#[cfg(test)]
mod pruebas {
    use super::{origen_valido, texto_json};

    #[test]
    fn un_nombre_de_archivo_no_corta_el_codigo() {
        assert_eq!(texto_json(r#"C:\Música\a"b.wav"#), r#""C:\\Música\\a\"b.wav""#);
        assert_eq!(texto_json("x\n</script>\u{2028}"), r#""x\u000a\u003c/script\u003e\u2028""#);
    }

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
