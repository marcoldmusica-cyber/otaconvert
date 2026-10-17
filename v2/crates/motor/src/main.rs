//! Motor de Otaconvert V2 (Fase 0): por ahora solo pruebas de audio por WASAPI.

use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use otaconvert_audio::prueba::Escenario;
use otaconvert_audio::{crear, Ajustes, Comando, Control, Detector, Mezclador, Pista};

mod componentes;
mod despierto;
mod dialogo;
mod equipo;
mod pantallas;
mod show_cmd;
#[cfg(windows)]
mod proyector;
mod proyector_cliente;
#[cfg(windows)]
mod prueba_proyector;
#[cfg(windows)]
mod prueba_funcion;
#[cfg(windows)]
mod prueba_esquinas;
mod ventana_ui;
#[cfg(windows)]
mod miniatura;
mod sonido_cpal;
mod prueba_hardware;
mod vigilante;
#[cfg(windows)]
mod prueba_video;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Seguridad: las DLL que se carguen después de arrancar (por nombre, sin
/// ruta completa) solo se buscan en la carpeta del programa y en System32,
/// nunca en la carpeta actual ni en el PATH. Evita que una DLL falsa dejada
/// en Descargas o en la carpeta del show se cuele en el motor.
#[cfg(windows)]
fn endurecer_carga_de_dll() {
    use windows_sys::Win32::System::LibraryLoader::{
        SetDefaultDllDirectories, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
    };
    // SAFETY: llamada sin punteros; solo cambia la política del proceso.
    let ok = unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS) };
    if ok == 0 {
        println!("Aviso: no se pudo restringir la carga de DLL");
    }
}

#[cfg(not(windows))]
fn endurecer_carga_de_dll() {}

fn main() -> ExitCode {
    endurecer_carga_de_dll();
    let todos: Vec<String> = std::env::args().skip(1).collect();
    // `--salida X` y `--pantalla X` pueden ir en cualquier lugar; lo demás son el comando y sus números.
    let mut salida: Option<String> = None;
    let mut pantalla: Option<String> = None;
    let mut carpeta: Option<String> = None;
    let mut archivo: Option<String> = None;
    let mut fallo: Option<(String, u64)> = None;
    let mut auriculares: Option<String> = None;
    let mut margen: Option<String> = None;
    let mut args: Vec<String> = Vec::new();
    let mut it = todos.into_iter();
    while let Some(a) = it.next() {
        if a == "--salida" {
            salida = it.next();
        } else if a == "--pantalla" {
            pantalla = it.next();
        } else if a == "--carpeta" {
            carpeta = it.next();
        } else if a == "--archivo" {
            archivo = it.next();
        } else if a == "--auriculares" {
            auriculares = it.next();
        } else if a == "--margen" {
            margen = it.next();
        } else if a == "--fallo" {
            let tipo = it.next().unwrap_or_default();
            let en = it.next().and_then(|s| s.parse().ok()).unwrap_or(2000);
            fallo = Some((tipo, en));
        } else {
            args.push(a);
        }
    }
    let salida = salida.as_deref();
    let pantalla = pantalla.as_deref();
    // Doble clic en "Prueba de hardware.exe" (sin argumentos): corre la prueba.
    let nombre_exe = std::env::current_exe()
        .ok()
        .and_then(|e| e.file_stem().map(|s| s.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    if args.is_empty() && nombre_exe.starts_with("prueba") {
        args.push("--prueba-hardware".into());
    }
    // Uso con el ratón (demo y versión portable): arrastrar una carpeta o un
    // archivo .otashow sobre el programa lo abre; doble clic en "Otaconvert.exe"
    // abre la carpeta "Show de ejemplo" que está a su lado.
    let mut interactivo = false;
    if args.len() == 1 && !args[0].starts_with("--") && std::path::Path::new(&args[0]).exists() {
        args.insert(0, "--abrir-show".into());
        interactivo = true;
    } else if args.is_empty() && nombre_exe == "otaconvert" {
        let ejemplo = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("Show de ejemplo")));
        match ejemplo.filter(|d| d.is_dir()) {
            Some(d) => {
                args = vec!["--abrir-show".into(), d.to_string_lossy().into_owned()];
                interactivo = true;
            }
            None => {
                println!("Otaconvert {VERSION}");
                println!("Para abrir un show, arrastra su carpeta (o una carpeta con audios, vídeos e imágenes) sobre Otaconvert.exe.");
                esperar_enter();
                return ExitCode::SUCCESS;
            }
        }
    }
    let sin_pausa = args.iter().any(|a| a == "--sin-pausa");
    // Interfaz: ventana propia por defecto; --navegador la abre en el
    // navegador; --sin-navegador no abre nada (pruebas).
    let interfaz = if args.iter().any(|a| a == "--sin-navegador") {
        show_cmd::Interfaz::Ninguna
    } else if args.iter().any(|a| a == "--navegador") {
        show_cmd::Interfaz::Navegador
    } else {
        show_cmd::Interfaz::Ventana
    };
    let num = |i: usize, defecto: u64| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(defecto);
    let r = match args.first().map(String::as_str) {
        Some("--version") => {
            println!("otaconvert-motor {VERSION}");
            println!("Desarrollado por @marcold.ok (Instagram): sugerencias y errores, escríbeme.");
            Ok(true)
        }
        Some("--listar-salidas") => {
            listar_salidas();
            Ok(true)
        }
        Some("--comprobar") => Ok(componentes::imprimir()),
        Some("--show-ejemplo") => match args.get(1) {
            Some(c) => show_cmd::crear_ejemplo(std::path::Path::new(c)),
            None => Err("falta la carpeta: --show-ejemplo <carpeta>".into()),
        },
        Some("--abrir-show") => match args.get(1) {
            Some(r) => show_cmd::abrir(
                std::path::Path::new(r),
                interfaz,
                salida,
                auriculares.as_deref(),
                margen.as_deref().and_then(|m| m.parse().ok()).unwrap_or(50),
                pantalla,
            ),
            None => Err("falta el show: --abrir-show <carpeta o archivo .otashow>".into()),
        },
        // Uso interno: el proceso de proyección que abre y vigila el motor.
        #[cfg(windows)]
        Some("--proyector") => {
            vigilante::sin_ventanas_de_error();
            proyector::ejecutar(&args)
        }
        #[cfg(windows)]
        Some("--prueba-proyector") => prueba_proyector::ejecutar(num(1, 30), pantalla),
        #[cfg(windows)]
        Some("--prueba-esquinas") => prueba_esquinas::ejecutar(carpeta.as_deref().map(std::path::Path::new), pantalla),
        #[cfg(windows)]
        Some("--prueba-mapeo") => prueba_esquinas::proceso(pantalla),
        #[cfg(windows)]
        Some("--prueba-funcion") => prueba_funcion::ejecutar(num(1, 300), carpeta.as_deref().map(std::path::Path::new), pantalla, salida),
        #[cfg(windows)]
        Some("--prueba-movimiento") => match archivo.as_deref() {
            Some(a) => prueba_proyector::movimiento(std::path::Path::new(a)),
            None => Err("falta --archivo <vídeo>".into()),
        },
        // Uso interno: foto chica de un vídeo o imagen (proceso aparte).
        #[cfg(windows)]
        Some("--miniatura") => match (args.get(1), args.get(2)) {
            (Some(a), Some(s)) => {
                vigilante::sin_ventanas_de_error();
                let inicio = args.get(3).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0);
                miniatura::ejecutar(std::path::Path::new(a), std::path::Path::new(s), inicio)
            }
            _ => Err("uso: --miniatura <archivo> <salida.jpg> [inicio_s]".into()),
        },
        Some("--info-equipo") => {
            equipo::imprimir();
            Ok(true)
        }
        Some("--prueba-hardware") => prueba_hardware::ejecutar(!sin_pausa),
        Some("--listar-pantallas") => {
            pantallas::imprimir();
            Ok(true)
        }
        Some("--prueba-vigilante") => vigilante::prueba(num(1, 10), salida),
        // Uso interno del vigilante: el motor hijo que se vigila.
        Some("--hijo-audio") => vigilante::hijo(salida, num(1, 0), fallo),
        Some("--tono") => tono(num(1, 3), num(2, 50), salida),
        Some("--prueba-vivo") => prueba_vivo(num(1, 60), num(2, 50), salida),
        #[cfg(windows)]
        Some("--prueba-reproduccion") => match archivo.as_deref() {
            Some(a) => prueba_video::reproduccion(num(1, 60), std::path::Path::new(a), pantalla),
            None => Err("falta --archivo <vídeo>".into()),
        },
        #[cfg(windows)]
        Some("--prueba-video") => prueba_video::ejecutar(num(1, 200), pantalla, carpeta.as_deref().map(std::path::Path::new)),
        _ => {
            println!("otaconvert-motor {VERSION}");
            println!("Uso:");
            println!("  --version");
            println!("  --prueba-hardware [--sin-pausa]        todas las pruebas en orden, con informe (también con doble clic en \"Prueba de hardware.exe\")");
            println!("  --info-equipo                          Windows, procesador, memoria, tarjetas gráficas, corriente");
            println!("  --show-ejemplo <carpeta>               crea un show de ejemplo");
            println!("  --abrir-show <carpeta | .otashow> [--salida X] [--auriculares Y] [--margen ms] [--navegador | --sin-navegador]   abre el show con sonido en su ventana");
            println!("  --comprobar                            componentes necesarios: qué hay y qué falta");
            println!("  --listar-salidas                       salidas de audio, con número e identificador");
            println!("  --listar-pantallas                     pantallas para la salida de vídeo");
            println!("  --prueba-vigilante [fallos]            caídas y cuelgues a propósito; mide el tiempo sin sonido");
            println!("  --tono [segundos] [margen_ms]          tono de 440 Hz a -12 dBFS");
            println!("  --prueba-vivo [segundos] [margen_ms]   órdenes al azar en la salida real");
            println!("  --prueba-video [cambios]               cambios de vídeo por capas, contando cuadros negros");
            println!("  --prueba-proyector [cambios]           el proceso de proyección de verdad: cambios sin negros, telón, parar, fundir y caída");
            println!("  --salida <número | nombre | id>        elige la salida de audio (si no, la predeterminada)");
            println!("  --prueba-reproduccion [segundos] --archivo <vídeo>   reproducción continua contando cuadros perdidos");
            println!("  --pantalla <número | nombre | id>      pantalla para el vídeo (si no, la que no es principal)");
            println!("  --carpeta <ruta>                       vídeos de prueba llamados RRGGBB.* (color del centro)");
            Ok(true)
        }
    };
    match r {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            println!("Error: {e}");
            // Abierto con el ratón: que se pueda leer el error antes de que se cierre la ventana.
            if interactivo {
                esperar_enter();
            }
            ExitCode::FAILURE
        }
    }
}

fn esperar_enter() {
    println!("Pulsa Enter para cerrar.");
    let mut s = String::new();
    let _ = std::io::stdin().read_line(&mut s);
}

/// Salidas de audio en el orden en que las da Windows, con su identificador estable.
fn salidas() -> Result<Vec<(cpal::Device, String)>, String> {
    let host = cpal::default_host();
    let lista = host.output_devices().map_err(|e| format!("no se pudieron leer las salidas: {e}"))?;
    Ok(lista
        .map(|d| {
            let id = d.id().map(|i| i.to_string()).unwrap_or_default();
            (d, id)
        })
        .collect())
}

/// Lista las salidas de audio del sistema. Si no hay tarjeta (por ejemplo
/// en los equipos de GitHub) lo dice y termina bien.
fn listar_salidas() {
    let host = cpal::default_host();
    println!("Sistema de audio: {:?}", host.id());
    let predeterminada = host.default_output_device().and_then(|d| d.id().ok()).map(|i| i.to_string());
    let lista = match salidas() {
        Ok(l) => l,
        Err(e) => {
            println!("{e}");
            return;
        }
    };
    if lista.is_empty() {
        println!("No hay salidas de audio.");
        return;
    }
    for (n, (d, id)) in lista.iter().enumerate() {
        let marca = if predeterminada.as_deref() == Some(id.as_str()) { " (predeterminada)" } else { "" };
        println!("{}. {d}{marca}", n + 1);
        match d.default_output_config() {
            Ok(c) => println!(
                "   {} Hz · {} canales · {:?} · búfer {:?}",
                c.sample_rate(),
                c.channels(),
                c.sample_format(),
                c.buffer_size()
            ),
            Err(e) => println!("   sin configuración ({e})"),
        }
        println!("   id: {id}");
    }
}

/// Busca la salida pedida por número de la lista, identificador exacto o parte
/// del nombre. Sin `pedida`, la predeterminada de Windows. Si la pedida no
/// está, es un error: nunca se cambia en silencio a otra salida.
fn elegir_salida(pedida: Option<&str>) -> Result<cpal::Device, String> {
    let Some(p) = pedida else {
        return cpal::default_host()
            .default_output_device()
            .ok_or_else(|| "no hay salida de audio predeterminada".to_string());
    };
    let lista = salidas()?;
    let nombres = || {
        lista.iter().enumerate().map(|(n, (d, _))| format!("{}. {d}", n + 1)).collect::<Vec<_>>().join("\n  ")
    };
    if let Ok(n) = p.parse::<usize>() {
        return match lista.get(n.wrapping_sub(1)) {
            Some((d, _)) => Ok(d.clone()),
            None => Err(format!("no existe la salida {n}. Salidas:\n  {}", nombres())),
        };
    }
    if let Some((d, _)) = lista.iter().find(|(_, id)| id == p) {
        return Ok(d.clone());
    }
    let p_min = p.to_lowercase();
    let coinciden: Vec<&cpal::Device> =
        lista.iter().filter(|(d, _)| d.to_string().to_lowercase().contains(&p_min)).map(|(d, _)| d).collect();
    match coinciden.as_slice() {
        [d] => Ok((*d).clone()),
        [] => Err(format!("no se encontró la salida \"{p}\". Salidas:\n  {}", nombres())),
        _ => Err(format!("\"{p}\" coincide con varias salidas; usa el número. Salidas:\n  {}", nombres())),
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

/// Abre la salida elegida (o la predeterminada) en modo compartido con un
/// margen de `margen_ms` y conecta el mezclador.
fn abrir(margen_ms: u64, salida: Option<&str>, umbral: impl FnOnce(u32) -> f32) -> Result<Salida, String> {
    let device = elegir_salida(salida)?;
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

fn tono(segundos: u64, margen_ms: u64, salida: Option<&str>) -> Result<bool, String> {
    let amplitud = 0.25; // -12 dBFS
    let mut s = abrir(margen_ms, salida, |fs| Detector::umbral_para_tonos(amplitud, 440.0, fs))?;
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

fn prueba_vivo(segundos: u64, margen_ms: u64, salida: Option<&str>) -> Result<bool, String> {
    let semilla = 1;
    let mut esc = None;
    let mut s = abrir(margen_ms, salida, |fs| {
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
