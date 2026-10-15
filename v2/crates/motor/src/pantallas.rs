//! Pantallas conectadas, para elegir la salida de vídeo (el Proyector).
//!
//! Por cada pantalla: nombre del monitor o proyector, tarjeta gráfica que la
//! maneja, posición, resolución en píxeles reales, frecuencia y si es la
//! principal. Todo con la API de Windows, sin permisos de administrador.

#[derive(Debug, Clone)]
pub struct Pantalla {
    /// Nombre interno de Windows, p. ej. `\\.\DISPLAY2`. Cambia si se
    /// reordenan las pantallas: el perfil del teatro guardará también el nombre.
    pub dispositivo: String,
    /// Nombre del monitor o proyector (EDID), p. ej. "C24F390".
    pub nombre: String,
    /// Tarjeta gráfica que la maneja.
    pub gpu: String,
    pub x: i32,
    pub y: i32,
    pub ancho: i32,
    pub alto: i32,
    pub hz: u32,
    pub principal: bool,
}

pub fn imprimir() {
    let lista = listar();
    if lista.is_empty() {
        println!("No hay pantallas.");
        return;
    }
    for (n, p) in lista.iter().enumerate() {
        let marca = if p.principal { " (principal)" } else { "" };
        println!("{}. {}{marca}", n + 1, p.nombre);
        println!("   {} × {} a {} Hz · posición {}, {}", p.ancho, p.alto, p.hz, p.x, p.y);
        println!("   tarjeta gráfica: {}", p.gpu);
        println!("   id: {}", p.dispositivo);
    }
    if lista.len() == 1 {
        println!("Solo hay una pantalla: para proyectar, conecta el proyector y elige \"Extender\" en Windows (Win + P).");
    }
}

#[cfg(not(windows))]
pub fn listar() -> Vec<Pantalla> {
    Vec::new()
}

#[cfg(windows)]
pub fn listar() -> Vec<Pantalla> {
    win::listar()
}

#[cfg(windows)]
pub(crate) mod win {
    use super::Pantalla;
    use std::collections::HashMap;
    use std::mem::{size_of, zeroed};
    use std::ptr::{null, null_mut};

    use windows_sys::Win32::Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
        DISPLAYCONFIG_TARGET_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
    };
    use windows_sys::Win32::Foundation::{LPARAM, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplayDevicesW, EnumDisplayMonitors, EnumDisplaySettingsW, GetMonitorInfoW, DEVMODEW,
        DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
    };
    use windows_sys::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };

    const MONITORINFOF_PRIMARY: u32 = 1;

    fn texto(w: &[u16]) -> String {
        let fin = w.iter().position(|&c| c == 0).unwrap_or(w.len());
        String::from_utf16_lossy(&w[..fin])
    }

    /// `\\.\DISPLAYn` → nombre del monitor, con la API DisplayConfig.
    fn nombres_de_monitor() -> HashMap<String, String> {
        let mut mapa = HashMap::new();
        // SAFETY: buffers del tamaño que pide Windows; estructuras con su `size`.
        unsafe {
            let (mut np, mut nm) = (0u32, 0u32);
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) != 0 {
                return mapa;
            }
            let mut caminos: Vec<DISPLAYCONFIG_PATH_INFO> = vec![zeroed(); np as usize];
            let mut modos: Vec<DISPLAYCONFIG_MODE_INFO> = vec![zeroed(); nm as usize];
            if QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut np,
                caminos.as_mut_ptr(),
                &mut nm,
                modos.as_mut_ptr(),
                null_mut(),
            ) != 0
            {
                return mapa;
            }
            for c in &caminos[..np as usize] {
                let mut origen: DISPLAYCONFIG_SOURCE_DEVICE_NAME = zeroed();
                origen.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
                origen.header.size = size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
                origen.header.adapterId = c.sourceInfo.adapterId;
                origen.header.id = c.sourceInfo.id;
                let mut destino: DISPLAYCONFIG_TARGET_DEVICE_NAME = zeroed();
                destino.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
                destino.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
                destino.header.adapterId = c.targetInfo.adapterId;
                destino.header.id = c.targetInfo.id;
                if DisplayConfigGetDeviceInfo(&mut origen.header) == 0
                    && DisplayConfigGetDeviceInfo(&mut destino.header) == 0
                {
                    let nombre = texto(&destino.monitorFriendlyDeviceName);
                    if !nombre.is_empty() {
                        mapa.insert(texto(&origen.viewGdiDeviceName), nombre);
                    }
                }
            }
        }
        mapa
    }

    /// `\\.\DISPLAYn` → tarjeta gráfica.
    pub fn gpus() -> HashMap<String, String> {
        let mut mapa = HashMap::new();
        // SAFETY: DISPLAY_DEVICEW con `cb` correcto; se para cuando Windows devuelve 0.
        unsafe {
            for i in 0.. {
                let mut dd: DISPLAY_DEVICEW = zeroed();
                dd.cb = size_of::<DISPLAY_DEVICEW>() as u32;
                if EnumDisplayDevicesW(null(), i, &mut dd, 0) == 0 {
                    break;
                }
                mapa.insert(texto(&dd.DeviceName), texto(&dd.DeviceString));
            }
        }
        mapa
    }

    unsafe extern "system" fn por_monitor(h: HMONITOR, _: HDC, _: *mut RECT, datos: LPARAM) -> i32 {
        // SAFETY: `datos` es el `&mut Vec` que pasa `listar`, vivo durante la enumeración.
        let lista = unsafe { &mut *(datos as *mut Vec<(MONITORINFOEXW, u32)>) };
        let mut mi: MONITORINFOEXW = unsafe { zeroed() };
        mi.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
        if unsafe { GetMonitorInfoW(h, &mut mi as *mut MONITORINFOEXW as *mut MONITORINFO) } != 0 {
            let mut dm: DEVMODEW = unsafe { zeroed() };
            dm.dmSize = size_of::<DEVMODEW>() as u16;
            let hz = if unsafe { EnumDisplaySettingsW(mi.szDevice.as_ptr(), ENUM_CURRENT_SETTINGS, &mut dm) } != 0 {
                dm.dmDisplayFrequency
            } else {
                0
            };
            lista.push((mi, hz));
        }
        1
    }

    pub fn listar() -> Vec<Pantalla> {
        // Píxeles reales, no escalados por el zoom de Windows (Windows 10 1703+).
        // SAFETY: sin punteros; si falla (ya fijado), se sigue igual.
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        let mut crudos: Vec<(MONITORINFOEXW, u32)> = Vec::new();
        // SAFETY: el callback solo usa `crudos` durante esta llamada.
        unsafe {
            EnumDisplayMonitors(null_mut(), null(), Some(por_monitor), &mut crudos as *mut _ as LPARAM);
        }
        let nombres = nombres_de_monitor();
        let gpus = gpus();
        let mut lista: Vec<Pantalla> = crudos
            .into_iter()
            .map(|(mi, hz)| {
                let dispositivo = texto(&mi.szDevice);
                let r = mi.monitorInfo.rcMonitor;
                Pantalla {
                    nombre: nombres.get(&dispositivo).cloned().unwrap_or_else(|| "Pantalla sin nombre".into()),
                    gpu: gpus.get(&dispositivo).cloned().unwrap_or_else(|| "desconocida".into()),
                    dispositivo,
                    x: r.left,
                    y: r.top,
                    ancho: r.right - r.left,
                    alto: r.bottom - r.top,
                    hz,
                    principal: mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                }
            })
            .collect();
        // La principal primero, después de izquierda a derecha.
        lista.sort_by_key(|p| (!p.principal, p.x, p.y));
        lista
    }
}
