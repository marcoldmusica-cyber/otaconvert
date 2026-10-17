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

/// Hace en segundo plano las fotos chicas que faltan (una a la vez, en un
/// proceso aparte con prioridad baja y sin ventana, con límite de tiempo):
/// un archivo dañado o un cuelgue de mpv nunca llega al motor.
#[cfg(windows)]
fn generar_miniaturas(sesion: Arc<Mutex<Sesion>>) {
    use std::os::windows::process::CommandExt;
    const PRIORIDAD_BAJA: u32 = 0x0000_4000; // BELOW_NORMAL_PRIORITY_CLASS
    const SIN_VENTANA: u32 = 0x0800_0000; // CREATE_NO_WINDOW
    let Ok(exe) = std::env::current_exe() else { return };
    loop {
        let pendientes = sesion.lock().map(|mut s| s.miniaturas_pendientes()).unwrap_or_default();
        if pendientes.is_empty() {
            std::thread::sleep(std::time::Duration::from_secs(2));
            continue;
        }
        for (i, archivo, destino, inicio) in pendientes {
            let ok = destino.parent().is_some_and(|d| std::fs::create_dir_all(d).is_ok())
                && std::process::Command::new(&exe)
                    .arg("--miniatura")
                    .arg(&archivo)
                    .arg(&destino)
                    .arg(format!("{inicio:.3}"))
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .creation_flags(PRIORIDAD_BAJA | SIN_VENTANA)
                    .spawn()
                    .ok()
                    .and_then(|mut h| {
                        let fin = std::time::Instant::now() + std::time::Duration::from_secs(20);
                        loop {
                            match h.try_wait() {
                                Ok(Some(e)) => return Some(e.success()),
                                Ok(None) if std::time::Instant::now() < fin => std::thread::sleep(std::time::Duration::from_millis(50)),
                                _ => {
                                    let _ = h.kill();
                                    let _ = h.wait();
                                    return Some(false);
                                }
                            }
                        }
                    })
                    .unwrap_or(false)
                && destino.is_file();
            if let Ok(mut s) = sesion.lock() {
                if ok {
                    s.miniatura_hecha(i, &destino);
                } else {
                    s.miniatura_fallida(i);
                }
            }
        }
    }
}

/// Tipo de cue según la extensión del archivo (para armar un show desde una carpeta).
fn tipo_por_extension(p: &Path) -> Option<TipoCue> {
    let ext = p.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "wav" | "mp3" | "flac" | "ogg" | "oga" | "m4a" | "aac" | "aif" | "aiff" | "aifc" | "caf" | "mka" => Some(TipoCue::Audio),
        "mp4" | "m4v" | "mov" | "mkv" | "webm" | "avi" => Some(TipoCue::Video),
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" => Some(TipoCue::Imagen),
        _ => None,
    }
}

/// Carpeta sin archivo `.otashow`: arma un show con un cue por cada audio,
/// vídeo e imagen que tenga (también en sus subcarpetas, hasta 3 niveles), en
/// orden alfabético, y lo guarda en la carpeta. Para probar rápido con medios
/// propios (demo portable); los cues se ajustan después en Edición.
pub fn crear_desde_carpeta(carpeta: &Path) -> Result<std::path::PathBuf, String> {
    fn recorrer(base: &Path, dir: &Path, nivel: usize, salida: &mut Vec<String>) {
        let Ok(leidos) = std::fs::read_dir(dir) else { return };
        let mut entradas: Vec<_> = leidos.flatten().map(|e| e.path()).collect();
        entradas.sort();
        for p in entradas {
            // Las carpetas propias del programa no se recorren.
            let nombre = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if nombre.starts_with('.') || nombre == "registros" {
                continue;
            }
            if p.is_dir() && nivel < 3 {
                recorrer(base, &p, nivel + 1, salida);
            } else if tipo_por_extension(&p).is_some() {
                if let Ok(rel) = p.strip_prefix(base) {
                    let r = rel.to_string_lossy().replace('\\', "/");
                    // Solo rutas que el archivo del show acepta (sin nombres raros).
                    if otaconvert_show::rutas::validar(&r).is_ok() {
                        salida.push(r);
                    }
                }
            }
        }
    }
    let mut archivos = Vec::new();
    recorrer(carpeta, carpeta, 0, &mut archivos);
    // Orden por el nombre del archivo (no por la carpeta): "01 Luna.mp4" va
    // primero aunque esté dentro de una subcarpeta.
    archivos.sort_by_key(|a| (a.rsplit('/').next().unwrap_or(a).to_lowercase(), a.to_lowercase()));
    if archivos.is_empty() {
        return Err(format!(
            "la carpeta {} no tiene un show (.otashow) ni audios, vídeos o imágenes",
            carpeta.display()
        ));
    }
    // Mismo límite que el archivo del show (10.000 cues).
    archivos.truncate(10_000);
    let nombre = carpeta.file_name().and_then(|n| n.to_str()).unwrap_or("Show").to_string();
    let cues = archivos
        .iter()
        .enumerate()
        .map(|(n, a)| {
            let p = Path::new(a);
            Cue {
                id: n as u64 + 1,
                numero: (n + 1).to_string(),
                nombre: p.file_stem().and_then(|s| s.to_str()).unwrap_or(a).to_string(),
                tipo: tipo_por_extension(p).unwrap_or(TipoCue::Audio),
                archivo: a.clone(),
                ..Default::default()
            }
        })
        .collect();
    let show = Show { nombre: nombre.clone(), cues, ..Default::default() };
    let limpio: String = nombre.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).collect();
    let ruta = carpeta.join(format!("{}.otashow", limpio.trim()));
    guardar(&show, &ruta, &marca_de_tiempo()).map_err(|e| e.to_string())?;
    println!("Se armó el show «{nombre}» con {} cues (uno por archivo): {}", show.cues.len(), ruta.display());
    Ok(ruta)
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
/// Abre la proyección: en la pantalla pedida (`--pantalla`), o en la primera
/// que no es la principal (el proyector) a pantalla completa. Con una sola
/// pantalla, o con `--pantalla vista`, una ventana chica de vista previa
/// arriba a la derecha que no tapa los botones del programa.
#[cfg(windows)]
pub(crate) fn abrir_proyeccion(pantalla: Option<&str>) -> Result<crate::proyector_cliente::ProyectorProceso, String> {
    use crate::proyector_cliente::ProyectorProceso;
    let lista = crate::pantallas::listar();
    let elegida = match pantalla {
        Some("vista") => None,
        Some(p) => Some(crate::prueba_video::elegir_pantalla(Some(p))?),
        None => lista.iter().find(|p| !p.principal).cloned(),
    };
    let p = match elegida {
        Some(p) => {
            println!("Proyección: {} · {} × {} a pantalla completa", p.nombre, p.ancho, p.alto);
            ProyectorProceso::abrir(p.x, p.y, p.ancho, p.alto, false)?
        }
        None => {
            let base = lista.iter().find(|p| p.principal).or(lista.first()).ok_or("no hay pantallas")?;
            let w = (base.ancho / 4).max(320);
            let h = w * 9 / 16 + 32;
            println!("Proyección: vista previa en una ventana (hay una sola pantalla)");
            ProyectorProceso::abrir(base.x + base.ancho - w - 24, base.y + 140, w, h, true)?
        }
    };
    if !p.esperar_hola(std::time::Duration::from_secs(10)) {
        return Err("la proyección no arrancó (¿falta libmpv-2.dll junto al programa?)".into());
    }
    Ok(p)
}

#[cfg(not(windows))]
pub(crate) fn abrir_proyeccion(_pantalla: Option<&str>) -> Result<crate::proyector_cliente::ProyectorProceso, String> {
    Err("la proyección solo funciona en Windows".into())
}

pub fn abrir(
    ruta: &Path,
    interfaz: Interfaz,
    salida: Option<&str>,
    auriculares: Option<&str>,
    margen_ms: u64,
    pantalla: Option<&str>,
) -> Result<bool, String> {
    // Una carpeta sin show (p. ej. arrastrada sobre el programa): se arma uno con sus medios.
    let tiene_show = ruta.is_dir()
        && std::fs::read_dir(ruta).is_ok_and(|l| l.flatten().any(|e| e.path().extension().is_some_and(|x| x == otaconvert_show::EXTENSION)));
    let creado;
    let ruta = if ruta.is_dir() && !tiene_show {
        creado = crear_desde_carpeta(ruta)?;
        creado.as_path()
    } else {
        ruta
    };
    let mut sesion = Sesion::abrir(ruta).map_err(|e| e.to_string())?;
    println!("Show: {} ({} cues) · {}", sesion.show.nombre, sesion.show.cues.len(), sesion.ruta.display());
    // Con el show abierto, Windows no suspende el equipo ni apaga la pantalla
    // (este hilo vive hasta que se cierra el programa).
    if crate::despierto::mantener() {
        sesion.anotar("ENERGIA", "El equipo no se suspenderá ni apagará la pantalla mientras el show esté abierto");
    } else {
        println!("Aviso: no se pudo pedir a Windows que no suspenda el equipo.");
        sesion.anotar("AVISO", "No se pudo pedir a Windows que no suspenda el equipo");
    }
    // El protector de pantalla no se puede frenar así (lo dice Microsoft): se avisa.
    if let Some((nombre, minutos)) = crate::despierto::protector_de_pantalla() {
        let es = format!(
            "Este equipo tiene el protector de pantalla «{nombre}» a los {minutos} min sin tocar nada: puede tapar el proyector en plena función. Desactívalo en Configuración → Personalización → Pantalla de bloqueo → Protector de pantalla."
        );
        let en = format!(
            "This computer has the \"{nombre}\" screen saver after {minutes} min idle: it can cover the projector mid-show. Turn it off in Settings → Personalization → Lock screen → Screen saver.",
            minutes = minutos
        );
        println!("Aviso: {es}");
        sesion.anotar("AVISO", &es);
        sesion.aviso = Some(otaconvert_servidor::Texto::nuevo(es, en));
    }
    // Las salidas viven aquí, en el hilo principal, mientras dure el programa.
    let principal = crate::sonido_cpal::abrir_salida(salida, margen_ms)?;
    println!("Principal: {} · {} Hz · margen {margen_ms} ms", principal.nombre, principal.frecuencia);
    let mut salidas = Vec::new();
    let mut vigilar_errores = vec![("Principal", principal.errores.clone())];
    let mut vigilar_tardias = vec![("Principal", principal.tardias.clone(), principal.intervalo_max_us.clone())];
    let mut control_auriculares = None;
    if let Some(a) = auriculares {
        let s = crate::sonido_cpal::abrir_salida(Some(a), margen_ms)?;
        if s.frecuencia == principal.frecuencia {
            println!("Auriculares: {} · {} Hz", s.nombre, s.frecuencia);
            control_auriculares = Some(s.control);
            vigilar_errores.push(("Auriculares", s.errores.clone()));
            vigilar_tardias.push(("Auriculares", s.tardias.clone(), s.intervalo_max_us.clone()));
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
    // La proyección (vídeo e imagen), en un proceso aparte. Si no se puede
    // abrir, el sonido funciona igual y se avisa.
    match abrir_proyeccion(pantalla) {
        Ok(p) => sesion.conectar_proyector(Box::new(p)),
        Err(e) => {
            println!("Aviso: sin proyección: {e}");
            sesion.anotar("ERROR", &format!("proyección: {e}"));
            sesion.aviso = Some(otaconvert_servidor::Texto::nuevo(
                format!("No se pudo abrir la proyección: {e}. El sonido funciona igual."),
                format!("The projection could not be opened: {e}. Sound works as usual."),
            ));
        }
    }
    let sesion = Arc::new(Mutex::new(sesion));
    #[cfg(windows)]
    {
        let s2 = sesion.clone();
        let _ = std::thread::Builder::new().name("miniaturas".into()).spawn(move || generar_miniaturas(s2));
    }
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
    let mut tardias_vistas = vec![0u64; vigilar_tardias.len()];
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        // La tarjeta pidió sonido más tarde que el margen: pudo oírse un corte.
        // Se anota en el registro (para revisar después de la función) sin molestar.
        for (k, (nombre, tardias, maximo)) in vigilar_tardias.iter().enumerate() {
            let n = tardias.load(std::sync::atomic::Ordering::Relaxed);
            if n > tardias_vistas[k] {
                tardias_vistas[k] = n;
                let ms = maximo.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1000.0;
                let texto = format!(
                    "Posible corte de sonido en {nombre}: la tarjeta tardó más que el margen de {margen_ms} ms (mayor espera {ms:.0} ms; van {n}). Si se repite, sube el margen o revisa el equipo con la Prueba de hardware."
                );
                println!("{texto}");
                if let Ok(mut s) = sesion.lock() {
                    s.anotar("AVISO", &texto);
                }
            }
        }
        for (k, (nombre, contador)) in vigilar_errores.iter().enumerate() {
            let n = contador.load(std::sync::atomic::Ordering::Relaxed);
            if n > vistos[k] {
                vistos[k] = n;
                let texto = format!("¡Atención! La salida {nombre} dio un error: puede haberse desconectado la tarjeta de sonido. Revisa el cable.");
                let nombre_en = if *nombre == "Auriculares" { "Headphones" } else { "Main" };
                let texto_en = format!("Warning! The {nombre_en} output reported an error: the sound card may have been disconnected. Check the cable.");
                println!("{texto}");
                if let Ok(mut s) = sesion.lock() {
                    s.anotar_error(&texto);
                    s.aviso = Some(otaconvert_servidor::Texto::nuevo(texto, texto_en));
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
fn usar_navegador(sesion: &Arc<Mutex<Sesion>>, url: &str, motivo: &otaconvert_servidor::Texto) {
    println!("{}", motivo.es);
    if let Ok(mut s) = sesion.lock() {
        s.anotar("AVISO", &motivo.es);
        s.aviso = Some(motivo.clone());
    }
    abrir_navegador(url);
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn una_carpeta_con_medios_se_vuelve_un_show() {
        let d = std::env::temp_dir().join(format!("otaconvert-prueba-carpeta-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("Videos")).unwrap();
        std::fs::create_dir_all(d.join(".oculta")).unwrap();
        for f in ["02 Lluvia.wav", "Videos/01 Luna.mp4", "mapa.PNG", "notas.txt", ".oculta/x.wav"] {
            std::fs::write(d.join(f), b"x").unwrap();
        }
        let ruta = crear_desde_carpeta(&d).unwrap();
        let show = otaconvert_show::abrir(&ruta).unwrap();
        let resumen: Vec<(String, String, TipoCue)> = show.cues.iter().map(|c| (c.numero.clone(), c.archivo.clone(), c.tipo)).collect();
        assert_eq!(
            resumen,
            vec![
                ("1".into(), "Videos/01 Luna.mp4".into(), TipoCue::Video),
                ("2".into(), "02 Lluvia.wav".into(), TipoCue::Audio),
                ("3".into(), "mapa.PNG".into(), TipoCue::Imagen),
            ],
            "un cue por medio, en orden por nombre de archivo, sin .txt ni carpetas ocultas"
        );
        assert_eq!(show.cues[0].nombre, "01 Luna");
        // Una carpeta sin medios: error claro, sin crear nada.
        let vacia = d.join("vacia");
        std::fs::create_dir_all(&vacia).unwrap();
        assert!(crear_desde_carpeta(&vacia).unwrap_err().contains("no tiene un show"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
