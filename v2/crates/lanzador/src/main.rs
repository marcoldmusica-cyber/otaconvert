//! `Otaconvert V2.exe` (etapa 6): lo que toca el usuario. Abre el motor
//! (`otaconvert-motor.exe`, junto a este exe) **sin ventana de consola** y
//! guarda sus mensajes en `registro-motor.txt` de la carpeta de datos.
//!
//! - Doble clic sin nada: abre la carpeta «Show de ejemplo» que está al lado.
//! - Arrastrar una carpeta o un `.otashow` encima (o doble clic en un
//!   `.otashow` si está asociado): abre ese show.
//! - Si falta Microsoft Edge WebView2 (la ventana del programa) y su instalador
//!   viene incluido en `componentes\`, ofrece instalarlo (sin internet ni
//!   administrador). Nunca instala nada sin que el usuario diga que sí.
//! - Si el motor termina con error, lo muestra en una ventana normal con las
//!   últimas líneas de su registro (antes se veía en la consola negra).

// Sin ventana de consola (también en las pruebas locales: es su razón de ser).
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

/// El instalador independiente de WebView2 que se incluye en el instalador y
/// la portable (decisión del usuario, 15/10/2026).
const INSTALADOR_WEBVIEW2: &str = "componentes/MicrosoftEdgeWebView2RuntimeInstallerX64.exe";

/// Carpeta de datos: junto al exe con `portable.txt`; si no, `%LOCALAPPDATA%\Otaconvert`.
fn carpeta_datos(exe_dir: &Path) -> PathBuf {
    if exe_dir.join("portable.txt").is_file() {
        return exe_dir.join("datos");
    }
    std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Otaconvert")).unwrap_or_else(|| exe_dir.join("datos"))
}

/// Los argumentos para el motor: `--abrir-show <ruta>` (lo arrastrado, o el
/// «Show de ejemplo» de al lado) o, si son opciones, tal cual.
fn argumentos(exe_dir: &Path, recibidos: Vec<String>) -> Option<Vec<String>> {
    match recibidos.as_slice() {
        [] => {
            let ejemplo = exe_dir.join("Show de ejemplo");
            ejemplo.is_dir().then(|| vec!["--abrir-show".to_string(), ejemplo.to_string_lossy().into_owned()])
        }
        [uno] if !uno.starts_with("--") => Some(vec!["--abrir-show".to_string(), uno.clone()]),
        _ => Some(recibidos),
    }
}

/// Las últimas `n` líneas de un texto.
fn ultimas_lineas(texto: &str, n: usize) -> String {
    let l: Vec<&str> = texto.lines().collect();
    l[l.len().saturating_sub(n)..].join("\n")
}

#[cfg(windows)]
mod win {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MB_YESNO};

    fn ancho(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    pub enum Tipo {
        Info,
        Aviso,
        Error,
    }

    pub fn mensaje(titulo: &str, texto: &str, tipo: Tipo) {
        let icono = match tipo {
            Tipo::Info => MB_ICONINFORMATION,
            Tipo::Aviso => MB_ICONWARNING,
            Tipo::Error => MB_ICONERROR,
        };
        let (t, x) = (ancho(titulo), ancho(texto));
        // SAFETY: cadenas terminadas en 0, vivas durante la llamada.
        unsafe { MessageBoxW(std::ptr::null_mut(), x.as_ptr(), t.as_ptr(), MB_OK | icono) };
    }

    pub fn pregunta(titulo: &str, texto: &str) -> bool {
        let (t, x) = (ancho(titulo), ancho(texto));
        // SAFETY: cadenas terminadas en 0, vivas durante la llamada.
        unsafe { MessageBoxW(std::ptr::null_mut(), x.as_ptr(), t.as_ptr(), MB_YESNO | MB_ICONWARNING) == IDYES }
    }

    fn registro(raiz: HKEY, clave: &str, valor: &str) -> Option<String> {
        let (c, v) = (ancho(clave), ancho(valor));
        let mut buf = vec![0u16; 256];
        let mut tam = (buf.len() * 2) as u32;
        // SAFETY: búfer con su tamaño en bytes; cadenas terminadas en 0.
        let r = unsafe { RegGetValueW(raiz, c.as_ptr(), v.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut tam) };
        if r != 0 {
            return None;
        }
        let n = (tam as usize / 2).saturating_sub(1);
        Some(String::from_utf16_lossy(&buf[..n.min(buf.len())]))
    }

    /// ¿Está WebView2? Claves documentadas por Microsoft ("Detect if a WebView2
    /// Runtime is already installed"): `pv` vacío o 0.0.0.0 = no está.
    pub fn hay_webview2() -> bool {
        let id = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
        [
            (HKEY_LOCAL_MACHINE, format!(r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{id}")),
            (HKEY_LOCAL_MACHINE, format!(r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{id}")),
            (HKEY_CURRENT_USER, format!(r"Software\Microsoft\EdgeUpdate\Clients\{id}")),
        ]
        .iter()
        .any(|(raiz, clave)| registro(*raiz, clave, "pv").is_some_and(|v| !v.is_empty() && v != "0.0.0.0"))
    }

    /// ¿Windows está en español? (para los mensajes de este exe).
    pub fn en_espanol() -> bool {
        // SAFETY: sin argumentos.
        let lang = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
        lang & 0x3FF == 0x0A
    }
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    use std::os::windows::process::CommandExt;
    use win::{mensaje, Tipo};
    // DLL solo de la carpeta del programa y de System32 (como el motor).
    // SAFETY: llamada sin punteros; solo cambia la política del proceso.
    unsafe { windows_sys::Win32::System::LibraryLoader::SetDefaultDllDirectories(windows_sys::Win32::System::LibraryLoader::LOAD_LIBRARY_SEARCH_DEFAULT_DIRS) };
    let es = win::en_espanol();
    let tr = |a: &str, b: &str| if es { a.to_string() } else { b.to_string() };
    let titulo = "Otaconvert V2";
    let Some(exe_dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) else {
        return std::process::ExitCode::FAILURE;
    };
    let motor = exe_dir.join("otaconvert-motor.exe");
    if !motor.is_file() {
        mensaje(titulo, &tr(
            "Falta otaconvert-motor.exe junto a Otaconvert V2.exe. Vuelve a instalar Otaconvert V2 o descomprime de nuevo la versión portable.",
            "otaconvert-motor.exe is missing next to Otaconvert V2.exe. Reinstall Otaconvert V2 or unzip the portable version again.",
        ), Tipo::Error);
        return std::process::ExitCode::FAILURE;
    }
    // WebView2: si falta y viene su instalador, se ofrece (un solo botón, nunca sin preguntar).
    if !win::hay_webview2() {
        let instalador = exe_dir.join(INSTALADOR_WEBVIEW2);
        if instalador.is_file() {
            let si = win::pregunta(titulo, &tr(
                "Falta un componente de Windows:\n\n• Microsoft Edge WebView2: muestra la ventana del programa.\n\nViene incluido con Otaconvert V2: se instala sin internet y sin pedir administrador (tarda un minuto).\n\n¿Instalarlo ahora?\n(Si eliges No, el programa se abre en el navegador.)",
                "A Windows component is missing:\n\n• Microsoft Edge WebView2: shows the program window.\n\nIt comes with Otaconvert V2: it installs without internet and without administrator rights (takes a minute).\n\nInstall it now?\n(If you choose No, the program opens in the browser.)",
            ));
            if si {
                let ok = std::process::Command::new(&instalador).args(["/silent", "/install"]).status().is_ok_and(|s| s.success());
                if !ok || !win::hay_webview2() {
                    mensaje(titulo, &tr(
                        "No se pudo instalar WebView2. El programa se abre igual en el navegador.",
                        "WebView2 could not be installed. The program opens in the browser anyway.",
                    ), Tipo::Aviso);
                }
            }
        }
    }
    let Some(args) = argumentos(&exe_dir, std::env::args().skip(1).collect()) else {
        mensaje(titulo, &tr(
            "Para abrir un show, arrastra su carpeta (o su archivo .otashow) sobre el icono de Otaconvert V2, o haz doble clic en el archivo .otashow.\n\nUna carpeta con audios, vídeos e imágenes sin show se convierte en uno sola.",
            "To open a show, drag its folder (or its .otashow file) onto the Otaconvert V2 icon, or double-click the .otashow file.\n\nA folder with audio, video and images but no show becomes one by itself.",
        ), Tipo::Info);
        return std::process::ExitCode::SUCCESS;
    };
    // Mensajes del motor a un archivo (el anterior queda como .anterior.txt).
    let datos = carpeta_datos(&exe_dir);
    let _ = std::fs::create_dir_all(&datos);
    let log = datos.join("registro-motor.txt");
    let _ = std::fs::rename(&log, datos.join("registro-motor.anterior.txt"));
    let salida = std::fs::File::create(&log).ok();
    let mut cmd = std::process::Command::new(&motor);
    cmd.args(&args).current_dir(&exe_dir).stdin(std::process::Stdio::null()).creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    if let Some(f) = salida {
        if let Ok(f2) = f.try_clone() {
            cmd.stdout(f).stderr(f2);
        }
    }
    let estado = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            mensaje(titulo, &tr(&format!("No se pudo abrir el motor: {e}"), &format!("The engine could not be started: {e}")), Tipo::Error);
            return std::process::ExitCode::FAILURE;
        }
    };
    if !estado.success() {
        let texto = std::fs::read_to_string(&log).unwrap_or_default();
        mensaje(titulo, &format!(
            "{}\n\n{}\n\n{} {}",
            tr("Otaconvert V2 se cerró con un error:", "Otaconvert V2 closed with an error:"),
            ultimas_lineas(&texto, 15),
            tr("Registro completo:", "Full log:"),
            log.display()
        ), Tipo::Error);
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

#[cfg(not(windows))]
fn main() {
    let _ = (argumentos(Path::new("."), Vec::new()), ultimas_lineas("", 1), carpeta_datos(Path::new(".")), INSTALADOR_WEBVIEW2);
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn que_abre_segun_lo_que_recibe() {
        let d = std::env::temp_dir().join(format!("otaconvert-prueba-lanzador-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        // Sin nada y sin «Show de ejemplo»: no hay qué abrir (se explica cómo).
        assert_eq!(argumentos(&d, vec![]), None);
        std::fs::create_dir_all(d.join("Show de ejemplo")).unwrap();
        let a = argumentos(&d, vec![]).unwrap();
        assert_eq!(a[0], "--abrir-show");
        assert!(a[1].ends_with("Show de ejemplo"));
        // Algo arrastrado encima: se abre eso.
        assert_eq!(argumentos(&d, vec![r"C:\Shows\Bodas.otashow".into()]).unwrap(), vec!["--abrir-show", r"C:\Shows\Bodas.otashow"]);
        // Opciones: tal cual.
        assert_eq!(argumentos(&d, vec!["--comprobar".into()]).unwrap(), vec!["--comprobar"]);
        // Portable: datos junto al exe.
        assert!(carpeta_datos(&d).ends_with("Otaconvert") || carpeta_datos(&d).ends_with("datos"));
        std::fs::write(d.join("portable.txt"), "").unwrap();
        assert_eq!(carpeta_datos(&d), d.join("datos"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn ultimas_lineas_del_registro() {
        assert_eq!(ultimas_lineas("a\nb\nc\nd", 2), "c\nd");
        assert_eq!(ultimas_lineas("a", 5), "a");
        assert_eq!(ultimas_lineas("", 3), "");
    }
}
