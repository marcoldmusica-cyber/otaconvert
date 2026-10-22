//! "Buscar archivos…" (etapa 5A): el diálogo de Windows para elegir audios,
//! vídeos e imágenes (varios a la vez). Lo pide la ventana; los elegidos se
//! traen al show (se copian a `Medios/` si están afuera de la carpeta).

use std::path::PathBuf;

fn ancho(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// Abre el diálogo y devuelve los archivos elegidos (vacío si se canceló).
/// `ingles`: textos del diálogo en inglés.
#[cfg(windows)]
pub fn elegir_archivos(ingles: bool) -> Vec<PathBuf> {
    use windows_sys::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows_sys::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, OFN_ALLOWMULTISELECT, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let exts = "*.wav;*.mp3;*.flac;*.ogg;*.oga;*.m4a;*.aac;*.aif;*.aiff;*.aifc;*.caf;*.mka;*.mp4;*.m4v;*.mov;*.mkv;*.webm;*.avi;*.png;*.jpg;*.jpeg;*.bmp;*.gif;*.webp";
    let (tipo, todos, titulo) = if ingles {
        ("Audio, video and images", "All files", "Add to the show")
    } else {
        ("Audios, vídeos e imágenes", "Todos los archivos", "Agregar al show")
    };
    // Filtro: pares "nombre\0patrón\0", terminado en "\0\0".
    let mut filtro = ancho(&format!("{tipo}\0{exts}\0{todos}\0*.*\0"));
    filtro.push(0);
    let mut titulo_w = ancho(titulo);
    titulo_w.push(0);
    let mut buf = vec![0u16; 64 * 1024];
    // SAFETY: estructura con su tamaño; búferes vivos durante la llamada; el
    // diálogo es modal y se maneja en este hilo (COM de un solo hilo).
    let elegido = unsafe {
        CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        // Encima de la ventana de Otaconvert (la que se tocó para pedirlo).
        ofn.hwndOwner = GetForegroundWindow();
        ofn.lpstrFilter = filtro.as_ptr();
        ofn.lpstrFile = buf.as_mut_ptr();
        ofn.nMaxFile = buf.len() as u32;
        ofn.lpstrTitle = titulo_w.as_ptr();
        ofn.Flags = OFN_ALLOWMULTISELECT | OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
        let r = GetOpenFileNameW(&mut ofn);
        CoUninitialize();
        r != 0
    };
    if !elegido {
        return Vec::new();
    }
    // Un archivo: "ruta completa\0\0". Varios: "carpeta\0nombre1\0nombre2\0\0".
    let partes: Vec<String> = buf.split(|&c| c == 0).take_while(|p| !p.is_empty()).map(String::from_utf16_lossy).collect();
    match partes.as_slice() {
        [] => Vec::new(),
        [uno] => vec![PathBuf::from(uno)],
        [carpeta, nombres @ ..] => nombres.iter().map(|n| PathBuf::from(carpeta).join(n)).collect(),
    }
}

#[cfg(not(windows))]
pub fn elegir_archivos(_ingles: bool) -> Vec<PathBuf> {
    Vec::new()
}

/// Archivo → Abrir… (etapa 6): elegir un archivo de show (`.otashow`).
#[cfg(windows)]
pub fn elegir_show(ingles: bool) -> Option<PathBuf> {
    use windows_sys::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows_sys::Win32::UI::Controls::Dialogs::{GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let (tipo, titulo) = if ingles { ("Otaconvert V2 shows", "Open a show") } else { ("Shows de Otaconvert V2", "Abrir un show") };
    let mut filtro = ancho(&format!("{tipo}\0*.otashow\0"));
    filtro.push(0);
    let mut titulo_w = ancho(titulo);
    titulo_w.push(0);
    let mut buf = vec![0u16; 4096];
    // SAFETY: estructura con su tamaño; búferes vivos durante la llamada; diálogo modal en este hilo.
    let elegido = unsafe {
        CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = GetForegroundWindow();
        ofn.lpstrFilter = filtro.as_ptr();
        ofn.lpstrFile = buf.as_mut_ptr();
        ofn.nMaxFile = buf.len() as u32;
        ofn.lpstrTitle = titulo_w.as_ptr();
        ofn.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
        let r = GetOpenFileNameW(&mut ofn);
        CoUninitialize();
        r != 0
    };
    if !elegido {
        return None;
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(PathBuf::from(String::from_utf16_lossy(&buf[..n])))
}

#[cfg(not(windows))]
pub fn elegir_show(_ingles: bool) -> Option<PathBuf> {
    None
}

/// Archivo → Nuevo show…: el diálogo «Guardar» de Windows para elegir el nombre
/// del show y su carpeta (desde ahí se puede crear una carpeta nueva).
#[cfg(windows)]
pub fn nuevo_show(ingles: bool) -> Option<PathBuf> {
    use windows_sys::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows_sys::Win32::UI::Controls::Dialogs::{GetSaveFileNameW, OFN_EXPLORER, OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let (tipo, titulo, nombre) = if ingles {
        ("Otaconvert V2 shows", "New show: choose its name and folder (an empty folder is best)", "New show")
    } else {
        ("Shows de Otaconvert V2", "Show nuevo: elige su nombre y su carpeta (mejor una carpeta vacía)", "Show nuevo")
    };
    let mut filtro = ancho(&format!("{tipo}\0*.otashow\0"));
    filtro.push(0);
    let mut titulo_w = ancho(titulo);
    titulo_w.push(0);
    let mut ext = ancho("otashow");
    ext.push(0);
    let mut buf = vec![0u16; 4096];
    for (k, c) in nombre.encode_utf16().enumerate() {
        buf[k] = c;
    }
    // SAFETY: estructura con su tamaño; búferes vivos durante la llamada; diálogo modal en este hilo.
    let elegido = unsafe {
        CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = GetForegroundWindow();
        ofn.lpstrFilter = filtro.as_ptr();
        ofn.lpstrFile = buf.as_mut_ptr();
        ofn.nMaxFile = buf.len() as u32;
        ofn.lpstrTitle = titulo_w.as_ptr();
        ofn.lpstrDefExt = ext.as_ptr();
        ofn.Flags = OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT | OFN_NOCHANGEDIR;
        let r = GetSaveFileNameW(&mut ofn);
        CoUninitialize();
        r != 0
    };
    if !elegido {
        return None;
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(PathBuf::from(String::from_utf16_lossy(&buf[..n])))
}

#[cfg(not(windows))]
pub fn nuevo_show(_ingles: bool) -> Option<PathBuf> {
    None
}
