//! Motor de Otaconvert V2. Por ahora solo el esqueleto de la Fase 0:
//! muestra la versión y la lista de salidas de audio (WASAPI).

use cpal::traits::{DeviceTrait, HostTrait};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--version") => println!("otaconvert-motor {VERSION}"),
        Some("--listar-salidas") => listar_salidas(),
        _ => {
            println!("otaconvert-motor {VERSION}");
            println!("Uso: otaconvert-motor [--version | --listar-salidas]");
        }
    }
}

/// Lista las salidas de audio del sistema. Nunca falla: si no hay tarjeta
/// (por ejemplo en los equipos de GitHub) lo dice y termina bien.
fn listar_salidas() {
    let host = cpal::default_host();
    println!("Sistema de audio: {:?}", host.id());

    match host.default_output_device() {
        Some(d) => println!("Salida predeterminada: {d}"),
        None => println!("Salida predeterminada: ninguna"),
    }

    let dispositivos = match host.output_devices() {
        Ok(d) => d,
        Err(e) => {
            println!("No se pudieron leer las salidas: {e}");
            return;
        }
    };
    let mut cuantas = 0;
    for d in dispositivos {
        cuantas += 1;
        match d.default_output_config() {
            Ok(c) => println!("- {d}: {c:?}"),
            Err(e) => println!("- {d}: sin configuración ({e})"),
        }
    }
    if cuantas == 0 {
        println!("No hay salidas de audio.");
    }
}

#[cfg(test)]
mod pruebas {
    /// Si esto falla, el ejecutable pediría el VC++ Redistributable.
    #[test]
    #[cfg(all(windows, target_env = "msvc"))]
    fn runtime_de_c_estatico() {
        assert!(cfg!(target_feature = "crt-static"));
    }
}
