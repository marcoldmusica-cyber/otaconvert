//! Comprobación de componentes al instalar y al abrir por primera vez.
//!
//! Regla (CLAUDE.md): todo lo que se pueda se incluye con el programa; lo que
//! falte se avisa en una sola ventana con un botón "Instalar lo que falta"
//! (instaladores sin internet incluidos, por usuario, sin administrador).
//! Si no falta nada, no se muestra nada.

use std::path::Path;

/// Qué hacer si un componente falta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arreglo {
    /// Viene dentro del programa: si falta, el programa está incompleto (reinstalar).
    Reinstalar,
    /// Se instala con un instalador incluido, sin internet ni administrador.
    InstaladorIncluido,
    /// Depende de algo que no podemos instalar (driver, versión de Windows).
    Explicar,
}

#[derive(Debug, Clone)]
pub struct Componente {
    pub nombre: &'static str,
    pub para_que: &'static str,
    pub ok: bool,
    pub detalle: String,
    pub arreglo: Arreglo,
}

#[cfg(windows)]
pub(crate) mod win {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{
        LoadLibraryExW, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32,
    };
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};

    fn ancho(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Lee un texto del registro, o `None` si no está.
    pub fn registro(raiz: HKEY, clave: &str, valor: &str) -> Option<String> {
        let (k, v) = (ancho(clave), ancho(valor));
        let mut buf = vec![0u16; 512];
        let mut bytes = (buf.len() * 2) as u32;
        // SAFETY: buffer con su tamaño en bytes; cadenas terminadas en 0.
        let e = unsafe {
            RegGetValueW(
                raiz,
                k.as_ptr(),
                v.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buf.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        if e != 0 {
            return None;
        }
        let n = (bytes as usize / 2).saturating_sub(1);
        Some(String::from_utf16_lossy(&buf[..n.min(buf.len())]))
    }

    pub fn hklm(clave: &str, valor: &str) -> Option<String> {
        registro(HKEY_LOCAL_MACHINE, clave, valor)
    }

    pub fn hkcu(clave: &str, valor: &str) -> Option<String> {
        registro(HKEY_CURRENT_USER, clave, valor)
    }

    /// ¿Se puede cargar esta DLL? Por ruta completa (carpeta del programa) o desde System32.
    pub fn se_carga(ruta_o_nombre: &Path, completa: bool) -> bool {
        let w: Vec<u16> = ruta_o_nombre.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let flags = if completa { LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32 } else { LOAD_LIBRARY_SEARCH_SYSTEM32 };
        // SAFETY: ruta terminada en 0; se libera enseguida.
        unsafe {
            let h = LoadLibraryExW(w.as_ptr(), std::ptr::null_mut(), flags);
            if h.is_null() {
                return false;
            }
            FreeLibrary(h);
        }
        true
    }
}

/// WebView2 (para la interfaz): claves documentadas por Microsoft.
const WEBVIEW2: &str = r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

#[cfg(windows)]
pub fn comprobar() -> Vec<Componente> {
    let mut v = Vec::new();

    // Windows 10 1607 (compilación 14393) o posterior: lo exige la carga segura de DLL.
    let nt = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let build: u32 = win::hklm(nt, "CurrentBuildNumber").and_then(|b| b.parse().ok()).unwrap_or(0);
    let nombre_win = win::hklm(nt, "DisplayVersion").unwrap_or_default();
    v.push(Componente {
        nombre: "Windows 10 u 11 de 64 bits",
        para_que: "el programa",
        ok: build >= 14393,
        detalle: format!("compilación {build} {nombre_win}").trim().to_string(),
        arreglo: Arreglo::Explicar,
    });

    // libmpv: viene con el programa.
    let mpv = otaconvert_video::mpv::ruta_junto_al_exe();
    let mpv_ok = mpv.as_ref().map(|p| p.exists() && otaconvert_video::mpv::Libmpv::cargar(p).is_ok()).unwrap_or(false);
    v.push(Componente {
        nombre: "Reproductor de vídeo (libmpv)",
        para_que: "vídeos e imágenes",
        ok: mpv_ok,
        detalle: mpv.map(|p| p.display().to_string()).unwrap_or_default(),
        arreglo: Arreglo::Reinstalar,
    });

    // Vulkan: libmpv la importa. Viene con los drivers de la tarjeta gráfica;
    // se planea incluir el cargador junto a libmpv.
    let carpeta = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
    let vk_propio = carpeta.as_ref().map(|c| c.join("vulkan-1.dll")).filter(|p| p.exists());
    let vk_ok = match &vk_propio {
        Some(p) => win::se_carga(p, true),
        None => win::se_carga(Path::new("vulkan-1.dll"), false),
    };
    v.push(Componente {
        nombre: "Vulkan (vulkan-1.dll)",
        para_que: "que el reproductor de vídeo pueda abrirse",
        ok: vk_ok,
        detalle: if vk_propio.is_some() { "incluida con el programa".into() } else { "de los drivers de la tarjeta gráfica".into() },
        arreglo: Arreglo::Reinstalar,
    });

    // WebView2: Windows 11 y casi todos los Windows 10 la traen.
    let pv = win::hklm(&format!(r"SOFTWARE\WOW6432Node\{WEBVIEW2}"), "pv")
        .filter(|s| !s.is_empty() && s != "0.0.0.0")
        .or_else(|| win::hkcu(&format!(r"Software\{WEBVIEW2}"), "pv").filter(|s| !s.is_empty() && s != "0.0.0.0"));
    v.push(Componente {
        nombre: "Microsoft Edge WebView2",
        para_que: "mostrar la ventana del programa",
        ok: pv.is_some(),
        detalle: pv.map(|p| format!("versión {p}")).unwrap_or_else(|| "no instalado".into()),
        arreglo: Arreglo::InstaladorIncluido,
    });

    v
}

#[cfg(not(windows))]
pub fn comprobar() -> Vec<Componente> {
    Vec::new()
}

/// Imprime el resultado. Devuelve `true` si no falta nada.
pub fn imprimir() -> bool {
    let lista = comprobar();
    let faltan: Vec<&Componente> = lista.iter().filter(|c| !c.ok).collect();
    for c in &lista {
        let estado = if c.ok { "Bien " } else { "FALTA" };
        println!("{estado}  {} — para {} ({})", c.nombre, c.para_que, c.detalle);
    }
    if faltan.is_empty() {
        println!("No falta nada.");
        return true;
    }
    println!();
    for c in &faltan {
        let que_hacer = match c.arreglo {
            Arreglo::Reinstalar => "viene con Otaconvert: vuelve a instalarlo o descarga de nuevo la portable",
            Arreglo::InstaladorIncluido => "botón \"Instalar lo que falta\" (instalador incluido, sin internet ni administrador)",
            Arreglo::Explicar => "no se puede instalar desde Otaconvert; ver la explicación",
        };
        println!("Falta {}: {que_hacer}.", c.nombre);
    }
    false
}

#[cfg(test)]
mod pruebas {
    #[test]
    #[cfg(windows)]
    fn lee_la_version_de_windows() {
        let b = super::win::hklm(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "CurrentBuildNumber");
        assert!(b.and_then(|s| s.parse::<u32>().ok()).unwrap_or(0) > 10000);
    }
}
