//! Crear un show de ejemplo y abrir un show con sonido y su interfaz: la
//! ventana propia (WebView2, en proceso aparte) o, si no se puede, el navegador.

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

/// Dónde se muestra la interfaz.
#[derive(Clone, Copy, PartialEq)]
pub enum Interfaz {
    /// Ventana propia con WebView2 (por defecto); si falla, el navegador.
    Ventana,
    Navegador,
    /// No se abre nada (pruebas): se imprime la dirección.
    Ninguna,
}

/// Abre el show con sonido y sirve la interfaz hasta que se cierre el programa.
pub fn abrir(ruta: &Path, interfaz: Interfaz, salida: Option<&str>, auriculares: Option<&str>, margen_ms: u64) -> Result<bool, String> {
    let mut sesion = Sesion::abrir(ruta).map_err(|e| e.to_string())?;
    println!("Show: {} ({} cues) · {}", sesion.show.nombre, sesion.show.cues.len(), sesion.ruta.display());
    // Las salidas viven aquí, en el hilo principal, mientras dure el programa.
    let principal = crate::sonido_cpal::abrir_salida(salida, margen_ms)?;
    println!("Principal: {} · {} Hz · margen {margen_ms} ms", principal.nombre, principal.frecuencia);
    let mut salidas = Vec::new();
    let mut vigilar_errores = vec![("Principal", principal.errores.clone())];
    let mut control_auriculares = None;
    if let Some(a) = auriculares {
        let s = crate::sonido_cpal::abrir_salida(Some(a), margen_ms)?;
        if s.frecuencia == principal.frecuencia {
            println!("Auriculares: {} · {} Hz", s.nombre, s.frecuencia);
            control_auriculares = Some(s.control);
            vigilar_errores.push(("Auriculares", s.errores.clone()));
            salidas.push(s.stream);
        } else {
            println!(
                "Aviso: Auriculares ({}) trabaja a {} Hz y Principal a {} Hz; por ahora deben ser iguales. Auriculares queda sin usar.",
                s.nombre, s.frecuencia, principal.frecuencia
            );
        }
    }
    let frecuencia = principal.frecuencia;
    salidas.push(principal.stream);
    sesion.conectar_sonido(Box::new(crate::sonido_cpal::SonidoCpal {
        principal: principal.control,
        frecuencia,
        auriculares: control_auriculares,
    }));
    let sesion = Arc::new(Mutex::new(sesion));
    let srv = Servidor::iniciar(sesion.clone()).map_err(|e| e.to_string())?;
    println!("Interfaz en {}", srv.url().split("?t=").next().unwrap_or(""));
    println!("(La dirección completa lleva una clave secreta; ábrela solo desde aquí.)");
    let mut ventana = None;
    match interfaz {
        Interfaz::Ventana => match crate::ventana_ui::Ventana::abrir(&srv.url()) {
            Ok(v) => {
                ventana = Some(v);
                println!("Se abrió la ventana de Otaconvert. Al cerrarla, el programa termina.");
            }
            Err(e) => usar_navegador(&sesion, &srv.url(), &e),
        },
        Interfaz::Navegador => {
            abrir_navegador(&srv.url());
            println!("Se abrió el navegador. Para terminar, cierra esta ventana.");
        }
        Interfaz::Ninguna => println!("{}", srv.url()),
    }
    // Si una tarjeta da errores (por ejemplo, se desconectó), avisar fuerte:
    // nunca se cambia en silencio a otra salida.
    let mut vistos = vec![0u64; vigilar_errores.len()];
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        for (k, (nombre, contador)) in vigilar_errores.iter().enumerate() {
            let n = contador.load(std::sync::atomic::Ordering::Relaxed);
            if n > vistos[k] {
                vistos[k] = n;
                let texto = format!("¡Atención! La salida {nombre} dio un error: puede haberse desconectado la tarjeta de sonido. Revisa el cable.");
                println!("{texto}");
                if let Ok(mut s) = sesion.lock() {
                    s.anotar_error(&texto);
                    s.aviso = Some(texto);
                }
            }
        }
        if let Some(v) = ventana.as_mut() {
            match v.revisar() {
                crate::ventana_ui::Estado::Abierta => {}
                crate::ventana_ui::Estado::Cerrada => {
                    if let Ok(mut s) = sesion.lock() {
                        // Antes de salir se guarda lo que haya cambiado.
                        s.guardar_si_hace_falta();
                        s.anotar("CERRAR", "Ventana cerrada por el usuario");
                    }
                    drop(salidas);
                    return Ok(true);
                }
                crate::ventana_ui::Estado::UsarNavegador(motivo) => {
                    ventana = None;
                    usar_navegador(&sesion, &srv.url(), &motivo);
                }
            }
        }
    }
}

/// La ventana propia no se pudo usar: se avisa (consola, ventana y registro)
/// y se abre la misma interfaz en el navegador. El sonido no se toca.
fn usar_navegador(sesion: &Arc<Mutex<Sesion>>, url: &str, motivo: &str) {
    println!("{motivo}");
    if let Ok(mut s) = sesion.lock() {
        s.anotar("AVISO", motivo);
        s.aviso = Some(motivo.to_string());
    }
    abrir_navegador(url);
}
