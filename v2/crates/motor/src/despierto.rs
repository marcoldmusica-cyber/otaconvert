//! Que el equipo no se duerma en plena función.
//!
//! Mientras haya un show abierto, el motor le pide a Windows que no suspenda
//! el equipo ni apague la pantalla (`SetThreadExecutionState` con
//! `ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED`, desde el hilo
//! principal, que vive mientras el programa está abierto). Sin esto, en una
//! escena larga sin vídeo (una imagen fija, el proyector en negro) Windows
//! podía apagar la pantalla del proyector a los pocos minutos.
//!
//! Lo que esto **no** frena (documentado por Microsoft): el protector de
//! pantalla, y que el usuario cierre la tapa del portátil o apriete el botón
//! de apagado. Por eso, si hay un protector de pantalla configurado, se avisa.

/// Pide que no se suspenda el equipo ni se apague la pantalla. Llamarla desde
/// el hilo que vive mientras el show está abierto. Devuelve si Windows lo aceptó.
#[cfg(windows)]
pub fn mantener() -> bool {
    use windows_sys::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED};
    // SAFETY: solo cambia el estado de energía pedido por este hilo.
    unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED) != 0 }
}

#[cfg(not(windows))]
pub fn mantener() -> bool {
    false
}

/// Lo que Windows tiene pedido en este momento por todos los programas:
/// (no suspender, no apagar la pantalla). Para comprobarlo sin administrador.
#[cfg(windows)]
pub fn estado_del_sistema() -> Option<(bool, bool)> {
    use windows_sys::Win32::System::Power::{CallNtPowerInformation, SystemExecutionState, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED};
    let mut estado: u32 = 0;
    // SAFETY: salida de 4 bytes en `estado`.
    let r = unsafe {
        CallNtPowerInformation(SystemExecutionState, std::ptr::null(), 0, (&mut estado as *mut u32).cast(), 4)
    };
    (r == 0).then_some((estado & ES_SYSTEM_REQUIRED != 0, estado & ES_DISPLAY_REQUIRED != 0))
}

/// Protector de pantalla configurado: (nombre del archivo, minutos de espera).
/// `ScreenSaveActive` vale 1 aunque no haya ninguno elegido, así que manda
/// `SCRNSAVE.EXE` (vacío = "(Ninguno)"); una directiva de la empresa pisa al usuario.
#[cfg(windows)]
pub fn protector_de_pantalla() -> Option<(String, u32)> {
    use crate::componentes::win::hkcu;
    const POLITICA: &str = r"Software\Policies\Microsoft\Windows\Control Panel\Desktop";
    const USUARIO: &str = r"Control Panel\Desktop";
    let leer = |valor: &str| hkcu(POLITICA, valor).or_else(|| hkcu(USUARIO, valor));
    let archivo = leer("SCRNSAVE.EXE").filter(|s| !s.trim().is_empty())?;
    if leer("ScreenSaveActive").is_some_and(|v| v.trim() == "0") {
        return None;
    }
    let segundos = leer("ScreenSaveTimeOut").and_then(|s| s.trim().parse::<u32>().ok()).unwrap_or(600);
    let nombre = std::path::Path::new(&archivo).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(archivo);
    Some((nombre, segundos.div_ceil(60).max(1)))
}

#[cfg(not(windows))]
pub fn protector_de_pantalla() -> Option<(String, u32)> {
    None
}

#[cfg(all(test, windows))]
mod pruebas {
    #[test]
    fn el_pedido_se_ve_en_el_estado_del_sistema() {
        // En un hilo aparte, para no dejar el pedido puesto en el hilo de las pruebas.
        std::thread::spawn(|| {
            assert!(super::mantener(), "Windows aceptó el pedido");
            let (sistema, pantalla) = super::estado_del_sistema().expect("se puede consultar sin administrador");
            assert!(sistema && pantalla, "el sistema lo tiene pedido: {sistema} {pantalla}");
        })
        .join()
        .unwrap();
    }
}
