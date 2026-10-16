//! Etapa 1 de la Fase 1: crear un show de ejemplo y abrir un show con la
//! interfaz mínima. La interfaz se abre en el navegador predeterminado; la
//! ventana propia (WebView2) llega en la etapa 3.

use std::path::Path;
use std::sync::{Arc, Mutex};

use otaconvert_servidor::{Servidor, Sesion};
use otaconvert_show::{guardar, marca_de_tiempo, BotonSonido, Cue, PaginaBotonera, Show, TipoCue};

/// Crea `carpeta/Ejemplo.otashow` con algunos cues (sin medios todavía).
pub fn crear_ejemplo(carpeta: &Path) -> Result<bool, String> {
    std::fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
    let ruta = carpeta.join("Ejemplo.otashow");
    if ruta.exists() {
        return Err(format!("ya existe {}; no se sobrescribe", ruta.display()));
    }
    let cue = |id: u64, numero: &str, nombre: &str, tipo: TipoCue, archivo: &str, nota: &str| Cue {
        id,
        numero: numero.into(),
        nombre: nombre.into(),
        tipo,
        archivo: archivo.into(),
        nota: nota.into(),
        ..Default::default()
    };
    let show = Show {
        nombre: "Show de ejemplo".into(),
        cues: vec![
            Cue { bucle: true, ..cue(1, "1", "Música de sala", TipoCue::Audio, "audio/sala.mp3", "Mientras entra el público") },
            cue(2, "2", "Luna sobre el telón", TipoCue::Video, "video/luna.mp4", "Cuando se apaga la sala"),
            cue(3, "2.5", "Viento nocturno", TipoCue::Audio, "audio/viento.wav", ""),
            cue(4, "3", "Campanas de boda", TipoCue::Audio, "audio/campanas.flac", "Entrada de la novia"),
            cue(5, "3A", "Mapa del pueblo", TipoCue::Imagen, "imagen/mapa.png", ""),
            cue(6, "4", "Final y aplausos", TipoCue::Audio, "audio/final.wav", "Saludo final"),
        ],
        botonera: vec![PaginaBotonera {
            nombre: "Improvisación".into(),
            botones: vec![
                BotonSonido { nombre: "Aplausos".into(), archivo: "audio/aplausos.wav".into(), ..Default::default() },
                BotonSonido { nombre: "Trueno".into(), archivo: "audio/trueno.wav".into(), color: "#F2C46B".into(), ..Default::default() },
            ],
        }],
        ..Default::default()
    };
    guardar(&show, &ruta, &marca_de_tiempo()).map_err(|e| e.to_string())?;
    println!("Show de ejemplo creado: {}", ruta.display());
    Ok(true)
}

#[cfg(windows)]
fn abrir_navegador(url: &str) {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let w = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (verbo, destino) = (w("open"), w(url));
    // SAFETY: cadenas terminadas en 0, vivas durante la llamada.
    unsafe { ShellExecuteW(std::ptr::null_mut(), verbo.as_ptr(), destino.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL) };
}

#[cfg(not(windows))]
fn abrir_navegador(_url: &str) {}

/// Abre el show y sirve la interfaz hasta que se cierre el programa.
pub fn abrir(ruta: &Path, navegador: bool) -> Result<bool, String> {
    let sesion = Sesion::abrir(ruta).map_err(|e| e.to_string())?;
    println!("Show: {} ({} cues) · {}", sesion.show.nombre, sesion.show.cues.len(), sesion.ruta.display());
    let sesion = Arc::new(Mutex::new(sesion));
    let srv = Servidor::iniciar(sesion).map_err(|e| e.to_string())?;
    println!("Interfaz en {}", srv.url().split("?t=").next().unwrap_or(""));
    println!("(La dirección completa lleva una clave secreta; ábrela solo desde aquí.)");
    if navegador {
        abrir_navegador(&srv.url());
        println!("Se abrió el navegador. Para terminar, cierra esta ventana.");
    } else {
        println!("{}", srv.url());
    }
    loop {
        std::thread::park();
    }
}
