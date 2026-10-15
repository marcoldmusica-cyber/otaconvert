//! Motor de Otaconvert V2 (Fase 0): por ahora solo pruebas de audio por WASAPI.

use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use otaconvert_audio::prueba::Escenario;
use otaconvert_audio::{crear, Ajustes, Comando, Control, Detector, Mezclador, Pista};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let num = |i: usize, defecto: u64| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(defecto);
    let r = match args.first().map(String::as_str) {
        Some("--version") => {
            println!("otaconvert-motor {VERSION}");
            Ok(true)
        }
        Some("--listar-salidas") => {
            listar_salidas();
            Ok(true)
        }
        Some("--tono") => tono(num(1, 3), num(2, 50)),
        Some("--prueba-vivo") => prueba_vivo(num(1, 60), num(2, 50)),
        _ => {
            println!("otaconvert-motor {VERSION}");
            println!("Uso:");
            println!("  --version");
            println!("  --listar-salidas");
            println!("  --tono [segundos] [margen_ms]          tono de 440 Hz a -12 dBFS");
            println!("  --prueba-vivo [segundos] [margen_ms]   órdenes al azar en la salida real");
            Ok(true)
        }
    };
    match r {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            println!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Lista las salidas de audio del sistema. Si no hay tarjeta (por ejemplo
/// en los equipos de GitHub) lo dice y termina bien.
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

/// Lo que mide el hilo de audio, leído al final por el hilo principal.
#[derive(Default)]
struct Medidas {
    llamadas: AtomicU64,
    intervalo_max_us: AtomicU64,
    cortes_probables: AtomicU64,
    cuadros_min: AtomicU64,
    cuadros_max: AtomicU64,
    discontinuidades: AtomicU64,
    discontinuidad_max: AtomicU64,
    errores: AtomicU64,
}

/// Llamadas iniciales que no cuentan para los intervalos (arranque de WASAPI).
const CALENTAMIENTO: u64 = 10;

struct Salida {
    _stream: cpal::Stream,
    control: Control,
    medidas: Arc<Medidas>,
    frecuencia: u32,
    canales: u16,
    margen: u32,
}

/// Abre la salida predeterminada en modo compartido con un margen de
/// `margen_ms` y conecta el mezclador.
fn abrir(margen_ms: u64, umbral: impl FnOnce(u32) -> f32) -> Result<Salida, String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no hay salida de audio predeterminada")?;
    let soportada = device.default_output_config().map_err(|e| e.to_string())?;
    if soportada.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!("formato {:?} todavía no soportado", soportada.sample_format()));
    }
    let frecuencia = soportada.sample_rate();
    let canales = soportada.channels();
    let margen = (frecuencia as u64 * margen_ms / 1000) as u32;
    let config = cpal::StreamConfig {
        channels: canales,
        sample_rate: frecuencia,
        buffer_size: cpal::BufferSize::Fixed(margen),
    };
    println!("Salida: {device} · {frecuencia} Hz · {canales} canales · margen {margen_ms} ms");

    // Las pruebas usan todas las rampas (también al iniciar) para que el
    // detector distinga un corte de la tarjeta de un arranque de golpe a
    // propósito. En los shows manda lo que elija el usuario (`Ajustes::para`).
    let (mezclador, control) = crear(frecuencia, canales, Ajustes::suaves(frecuencia));
    let medidas = Arc::new(Medidas { cuadros_min: AtomicU64::new(u64::MAX), ..Default::default() });
    let detector = Detector::nuevo(canales as usize, umbral(frecuencia));
    let limite_us = margen_ms * 1000;
    let datos = callback(mezclador, detector, medidas.clone(), canales as usize, limite_us);
    let m_err = medidas.clone();
    let stream = device
        .build_output_stream(config, datos, move |_e| {
            m_err.errores.fetch_add(1, Relaxed);
        }, None)
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(Salida { _stream: stream, control, medidas, frecuencia, canales, margen })
}

fn callback(
    mut mezclador: Mezclador,
    mut detector: Detector,
    m: Arc<Medidas>,
    canales: usize,
    limite_us: u64,
) -> impl FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static {
    let mut anterior: Option<Instant> = None;
    move |data: &mut [f32], _| {
        let ahora = Instant::now();
        let n = m.llamadas.fetch_add(1, Relaxed);
        if let Some(a) = anterior {
            if n > CALENTAMIENTO {
                let us = ahora.duration_since(a).as_micros() as u64;
                m.intervalo_max_us.fetch_max(us, Relaxed);
                if us > limite_us {
                    m.cortes_probables.fetch_add(1, Relaxed);
                }
            }
        }
        anterior = Some(ahora);
        let cuadros = (data.len() / canales) as u64;
        m.cuadros_min.fetch_min(cuadros, Relaxed);
        m.cuadros_max.fetch_max(cuadros, Relaxed);

        mezclador.procesar(data);
        detector.analizar(data);
        m.discontinuidades.store(detector.discontinuidades, Relaxed);
        m.discontinuidad_max.store(detector.maxima.to_bits() as u64, Relaxed);
    }
}

fn esperar_cuadros(cuadros: u32, frecuencia: u32) {
    std::thread::sleep(Duration::from_secs_f64(cuadros as f64 / frecuencia as f64));
}

fn tono(segundos: u64, margen_ms: u64) -> Result<bool, String> {
    let amplitud = 0.25; // -12 dBFS
    let mut s = abrir(margen_ms, |fs| Detector::umbral_para_tonos(amplitud, 440.0, fs))?;
    let cuadros = s.frecuencia as usize * segundos as usize;
    let pista = Arc::new(Pista::tono(440.0, amplitud, cuadros, 1, s.frecuencia));
    s.control
        .enviar(Comando::Iniciar { id: 1, pista, volumen: 1.0, desde: 0, bucle: false, fundido: 0 })
        .map_err(|_| "cola llena")?;
    println!("Sonando {segundos} s…");
    std::thread::sleep(Duration::from_secs(segundos) + Duration::from_millis(300));
    s.control.recoger_basura();
    Ok(informe(&s))
}

fn prueba_vivo(segundos: u64, margen_ms: u64) -> Result<bool, String> {
    let semilla = 1;
    let mut esc = None;
    let mut s = abrir(margen_ms, |fs| {
        let e = Escenario::nuevo(semilla, fs);
        let u = e.umbral();
        esc = Some(e);
        u
    })?;
    let mut esc = esc.expect("escenario creado al abrir");
    println!("Prueba de {segundos} s con órdenes al azar…");
    let fin = Instant::now() + Duration::from_secs(segundos);
    while Instant::now() < fin {
        let espera = esc.paso(&mut s.control);
        esperar_cuadros(espera, s.frecuencia);
    }
    s.control.enviar(Comando::PararTodo { fundido: 0 }).ok();
    std::thread::sleep(Duration::from_millis(300));
    s.control.recoger_basura();
    println!("Órdenes enviadas: {}", esc.ordenes);
    Ok(informe(&s))
}

/// Imprime el resultado. Devuelve `true` si no hubo ningún problema.
fn informe(s: &Salida) -> bool {
    let m = &s.medidas;
    let e = &s.control.estadisticas;
    let periodo_ms = |c: u64| c as f64 * 1000.0 / s.frecuencia as f64;
    let cortes = m.cortes_probables.load(Relaxed);
    let disc = m.discontinuidades.load(Relaxed);
    let recortes = e.recortes.load(Relaxed);
    let errores = m.errores.load(Relaxed);
    let basura = e.basura_llena.load(Relaxed);
    println!("— Informe —");
    println!("Llamadas de audio: {}", m.llamadas.load(Relaxed));
    println!(
        "Cuadros por llamada: {}–{} ({:.1}–{:.1} ms)",
        m.cuadros_min.load(Relaxed),
        m.cuadros_max.load(Relaxed),
        periodo_ms(m.cuadros_min.load(Relaxed)),
        periodo_ms(m.cuadros_max.load(Relaxed))
    );
    println!("Margen pedido: {} cuadros ({:.1} ms) · canales {}", s.margen, periodo_ms(s.margen as u64), s.canales);
    println!("Mayor intervalo entre llamadas: {:.1} ms", m.intervalo_max_us.load(Relaxed) as f64 / 1000.0);
    println!("Cortes probables (intervalo > margen): {cortes}");
    println!(
        "Discontinuidades en la mezcla: {disc} (máxima {:.5})",
        f32::from_bits(m.discontinuidad_max.load(Relaxed) as u32)
    );
    println!("Recortes: {recortes} · errores de la tarjeta: {errores} · memoria liberada en audio: {basura}");
    let ok = cortes == 0 && disc == 0 && recortes == 0 && errores == 0 && basura == 0;
    println!("{}", if ok { "RESULTADO: BIEN" } else { "RESULTADO: HAY PROBLEMAS" });
    ok
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
