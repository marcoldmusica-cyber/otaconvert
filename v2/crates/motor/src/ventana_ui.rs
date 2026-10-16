//! Etapa 3A: el motor abre y vigila la ventana propia del programa,
//! `otaconvert-ventana.exe` (junto al motor, proceso aparte: si se cuelga o se
//! cae, el sonido sigue). Le pasa la dirección con el token por la entrada
//! estándar (no por la línea de comandos, que otros programas pueden leer):
//! - termina con 0: el usuario cerró la ventana → el programa termina;
//! - termina con `SIN_WEBVIEW2`: no se pudo crear WebView2 → se abre el navegador;
//! - cualquier otra cosa (caída): se vuelve a abrir; tras varias caídas
//!   seguidas, el navegador.

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Código de salida de la ventana cuando WebView2 no se puede usar.
const SIN_WEBVIEW2: i32 = 3;
/// Caídas de la ventana permitidas en `VENTANA_CAIDAS` antes de pasar al navegador.
const MAX_CAIDAS: usize = 3;
const VENTANA_CAIDAS: Duration = Duration::from_secs(60);

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
    let exe = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("otaconvert-ventana.exe")))
        .filter(|e| e.is_file())
        .ok_or("falta otaconvert-ventana.exe junto al programa")?;
    let mut hijo = Command::new(exe)
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
            Some(SIN_WEBVIEW2) => Estado::UsarNavegador(
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
