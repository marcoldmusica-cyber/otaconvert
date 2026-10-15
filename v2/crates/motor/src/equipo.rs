//! Datos del equipo para el informe de la Prueba de hardware: Windows,
//! procesador, memoria, tarjetas gráficas y si está enchufado. Nada personal
//! salvo el nombre del equipo, que sirve para distinguir los informes.

#[derive(Debug, Clone, Default)]
pub struct Equipo {
    pub nombre: String,
    pub windows: String,
    pub procesador: String,
    pub nucleos: usize,
    pub memoria_gb: f64,
    pub tarjetas_graficas: Vec<String>,
    /// `Some(true)` enchufado, `Some(false)` a batería, `None` desconocido.
    pub enchufado: Option<bool>,
}

#[cfg(windows)]
pub fn leer() -> Equipo {
    use crate::componentes::win::hklm;
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let nt = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let windows = [
        hklm(nt, "ProductName"),
        hklm(nt, "DisplayVersion"),
        hklm(nt, "CurrentBuildNumber").map(|b| format!("(compilación {b})")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");

    let procesador = hklm(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0", "ProcessorNameString")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "desconocido".into());

    // SAFETY: estructura con su tamaño en dwLength.
    let memoria_gb = unsafe {
        let mut m: MEMORYSTATUSEX = std::mem::zeroed();
        m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut m) != 0 {
            m.ullTotalPhys as f64 / (1024.0 * 1024.0 * 1024.0)
        } else {
            0.0
        }
    };

    // SAFETY: estructura de salida simple.
    let enchufado = unsafe {
        let mut p: SYSTEM_POWER_STATUS = std::mem::zeroed();
        if GetSystemPowerStatus(&mut p) != 0 {
            match p.ACLineStatus {
                1 => Some(true),
                0 => Some(false),
                _ => None,
            }
        } else {
            None
        }
    };

    let mut tarjetas: Vec<String> = crate::pantallas::win::gpus().into_values().collect();
    tarjetas.sort();
    tarjetas.dedup();

    Equipo {
        nombre: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "equipo".into()),
        windows,
        procesador,
        nucleos: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
        memoria_gb,
        tarjetas_graficas: tarjetas,
        enchufado,
    }
}

#[cfg(not(windows))]
pub fn leer() -> Equipo {
    Equipo::default()
}

pub fn imprimir() {
    let e = leer();
    println!("Equipo: {}", e.nombre);
    println!("Windows: {}", e.windows);
    println!("Procesador: {} ({} hilos)", e.procesador, e.nucleos);
    println!("Memoria: {:.1} GB", e.memoria_gb);
    if e.tarjetas_graficas.is_empty() {
        println!("Tarjeta gráfica: desconocida");
    }
    for t in &e.tarjetas_graficas {
        println!("Tarjeta gráfica: {t}");
    }
    println!(
        "Corriente: {}",
        match e.enchufado {
            Some(true) => "enchufado",
            Some(false) => "a batería (para la función, enchufarlo)",
            None => "desconocido",
        }
    );
}
