//! Función simulada, lo más parecida posible a una función real:
//! `otaconvert-motor --prueba-funcion [segundos] --carpeta <vídeos> [--pantalla N | vista] [--salida X]`.
//!
//! Arma un show de verdad en una carpeta con tildes y espacios (como las de
//! muchos equipos de teatro): un ambiente en bucle, vídeos con sonido (los
//! `RRGGBB.mp4` de la carpeta: color liso + tono de 440 Hz), una imagen y
//! tonos cortos. Lo abre con la misma sesión, el mismo sonido (WASAPI) y la
//! misma proyección (proceso aparte, a pantalla completa) que el programa, y
//! durante `segundos` da GO, Pausa, Fundir todo, Parar todo, Mayús+F, elige
//! cues a mano y "salir desde aquí", al azar pero siempre igual (semilla fija).
//!
//! Mientras tanto mide:
//! - **La imagen:** el centro (y las esquinas, a pantalla completa) de la
//!   proyección unas 50 veces por segundo, comparando con lo que tendría que
//!   verse: un negro o un color que no corresponde es un fallo.
//! - **El sonido que sale de verdad:** graba la salida (modo "loopback" de
//!   WASAPI) y busca clics y cortes con el mismo detector de las pruebas del
//!   mezclador.
//! - **Pantalla completa:** que tape toda la pantalla y la barra de tareas,
//!   que no se vea el ratón y que un clic sobre la proyección no le robe el
//!   teclado al programa.
//! - **Memoria** del motor y de la proyección (que no crezca sin parar).
//! - **El registro** de la función (sin errores).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, StreamTrait};
use otaconvert_servidor::Sesion;
use otaconvert_show::{Cue, Show, TipoCue};
use serde_json::Value;

const TOL: i32 = 48;
/// Tras cada cambio, durante este tiempo vale lo de antes o lo nuevo.
const VENTANA_CAMBIO: Duration = Duration::from_millis(300);
const HZ: f32 = 440.0;
const AMP: f32 = 0.12;
/// Ambiente + vídeo + dos tonos cortos a la vez, como mucho.
const AMP_TOTAL: f32 = 0.6;
const IMAGEN: u32 = 0xC8C8C8;
const CLASE_PROYECCION: &str = "OtaconvertProyeccion";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Esperado {
    Negro,
    Color(u32),
    Fundiendo(u32),
}

type Rgb = (i32, i32, i32);

fn partes(c: u32) -> Rgb {
    (((c >> 16) & 0xFF) as i32, ((c >> 8) & 0xFF) as i32, (c & 0xFF) as i32)
}

fn negro(p: Rgb) -> bool {
    p.0 < 32 && p.1 < 32 && p.2 < 32
}

fn vale(p: Rgb, e: Esperado) -> bool {
    match e {
        Esperado::Negro => negro(p),
        Esperado::Color(c) => {
            let q = partes(c);
            (p.0 - q.0).abs() <= TOL && (p.1 - q.1).abs() <= TOL && (p.2 - q.2).abs() <= TOL
        }
        // Mientras se funde, el color baja hasta negro (nunca otro color ni más claro).
        Esperado::Fundiendo(c) => {
            let q = partes(c);
            negro(p) || (p.0 <= q.0 + TOL && p.1 <= q.1 + TOL && p.2 <= q.2 + TOL)
        }
    }
}

/// Semilla fija: la misma función simulada cada vez.
struct Azar(u64);
impl Azar {
    fn siguiente(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn hasta(&mut self, n: u64) -> u64 {
        self.siguiente() % n.max(1)
    }
    fn entre_ms(&mut self, a: u64, b: u64) -> Duration {
        Duration::from_millis(a + self.hasta(b - a + 1))
    }
}

fn escribir_wav(ruta: &Path, segundos: f32, fs: u32) -> Result<(), String> {
    let n = (segundos * fs as f32) as u32;
    let bytes = n * 4;
    let mut d = Vec::with_capacity(44 + bytes as usize);
    d.extend_from_slice(b"RIFF");
    d.extend_from_slice(&(36 + bytes).to_le_bytes());
    d.extend_from_slice(b"WAVEfmt ");
    d.extend_from_slice(&16u32.to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&2u16.to_le_bytes());
    d.extend_from_slice(&fs.to_le_bytes());
    d.extend_from_slice(&(fs * 4).to_le_bytes());
    d.extend_from_slice(&4u16.to_le_bytes());
    d.extend_from_slice(&16u16.to_le_bytes());
    d.extend_from_slice(b"data");
    d.extend_from_slice(&bytes.to_le_bytes());
    for i in 0..n {
        let v = (AMP * (2.0 * std::f32::consts::PI * HZ * i as f32 / fs as f32).sin() * 32767.0) as i16;
        d.extend_from_slice(&v.to_le_bytes());
        d.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(ruta, d).map_err(|e| format!("{}: {e}", ruta.display()))
}

fn guardar_wav_f32(ruta: &Path, m: &[f32], canales: u16, fs: u32) -> std::io::Result<()> {
    let bytes = (m.len() * 4) as u32;
    let mut d = Vec::with_capacity(44 + bytes as usize);
    d.extend_from_slice(b"RIFF");
    d.extend_from_slice(&(36 + bytes).to_le_bytes());
    d.extend_from_slice(b"WAVEfmt ");
    d.extend_from_slice(&16u32.to_le_bytes());
    d.extend_from_slice(&3u16.to_le_bytes());
    d.extend_from_slice(&canales.to_le_bytes());
    d.extend_from_slice(&fs.to_le_bytes());
    d.extend_from_slice(&(fs * 4 * canales as u32).to_le_bytes());
    d.extend_from_slice(&(4 * canales).to_le_bytes());
    d.extend_from_slice(&32u16.to_le_bytes());
    d.extend_from_slice(b"data");
    d.extend_from_slice(&bytes.to_le_bytes());
    for x in m {
        d.extend_from_slice(&x.to_le_bytes());
    }
    std::fs::write(ruta, d)
}

fn escribir_bmp(ruta: &Path, color: u32) -> Result<(), String> {
    let (w, h) = (1920u32, 1080u32);
    let fila = w * 3;
    let tam = fila * h;
    let mut d = Vec::with_capacity(54 + tam as usize);
    d.extend_from_slice(b"BM");
    d.extend_from_slice(&(54 + tam).to_le_bytes());
    d.extend_from_slice(&0u32.to_le_bytes());
    d.extend_from_slice(&54u32.to_le_bytes());
    d.extend_from_slice(&40u32.to_le_bytes());
    d.extend_from_slice(&(w as i32).to_le_bytes());
    d.extend_from_slice(&(h as i32).to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&24u16.to_le_bytes());
    d.extend_from_slice(&[0u8; 24]);
    let (r, g, b) = partes(color);
    for _ in 0..(w * h) {
        d.extend_from_slice(&[b as u8, g as u8, r as u8]);
    }
    std::fs::write(ruta, d).map_err(|e| format!("{}: {e}", ruta.display()))
}

/// Arma el show de prueba. Devuelve (ruta del .otashow, color de cada cue).
fn armar_show(carpeta_videos: &Path) -> Result<(PathBuf, Vec<Option<u32>>), String> {
    let mut videos: Vec<(u32, PathBuf)> = std::fs::read_dir(carpeta_videos)
        .map_err(|e| format!("{}: {e}", carpeta_videos.display()))?
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let nombre = p.file_stem()?.to_str()?.to_string();
            let ext = p.extension()?.to_str()?.to_lowercase();
            (ext == "mp4" && nombre.len() == 6).then(|| u32::from_str_radix(&nombre, 16).ok().map(|c| (c, p.clone())))?
        })
        .collect();
    videos.sort();
    if videos.len() < 3 {
        return Err(format!("hacen falta al menos 3 vídeos RRGGBB.mp4 en {}", carpeta_videos.display()));
    }
    videos.truncate(4);
    // Carpeta con espacios, tildes y eñe, como "C:\Users\Técnico\Escritorio".
    let show_dir = std::env::temp_dir().join("Otaconvert prueba función ñandú");
    let _ = std::fs::remove_dir_all(&show_dir);
    let medios = show_dir.join("Medios del show");
    std::fs::create_dir_all(&medios).map_err(|e| e.to_string())?;
    let mut nombres_video = Vec::new();
    for (c, p) in &videos {
        let nombre = format!("Vídeo {c:06X}.mp4");
        std::fs::copy(p, medios.join(&nombre)).map_err(|e| e.to_string())?;
        nombres_video.push((*c, nombre));
    }
    // Audios a 44,1 kHz (la tarjeta suele estar a 48 kHz): se adaptan al preparar.
    escribir_wav(&medios.join("tono largo.wav"), 4.0, 44_100)?;
    escribir_wav(&medios.join("tono corto.wav"), 1.5, 44_100)?;
    escribir_bmp(&medios.join("imagen fija.bmp"), IMAGEN)?;

    let mut cues = Vec::new();
    let mut colores = Vec::new();
    let mut agregar = |nombre: &str, tipo: TipoCue, archivo: String, bucle: bool, color: Option<u32>| {
        let n = cues.len() as u64 + 1;
        cues.push(Cue {
            id: n,
            numero: n.to_string(),
            nombre: nombre.into(),
            tipo,
            archivo: format!("Medios del show/{archivo}"),
            fundido_entrada_ms: 30,
            fundido_salida_ms: 30,
            bucle,
            ..Default::default()
        });
        colores.push(color);
    };
    agregar("Ambiente", TipoCue::Audio, "tono largo.wav".into(), true, None);
    let v = |k: usize| nombres_video[k % nombres_video.len()].clone();
    let orden = ["v0", "img", "v1", "corto", "v2", "v3", "img", "v0", "corto", "v1", "v2"];
    for o in orden {
        match o {
            "img" => agregar("Imagen fija", TipoCue::Imagen, "imagen fija.bmp".into(), false, Some(IMAGEN)),
            "corto" => agregar("Tono corto", TipoCue::Audio, "tono corto.wav".into(), false, None),
            _ => {
                let (c, n) = v(o[1..].parse::<usize>().unwrap_or(0));
                agregar(&format!("Vídeo {c:06X}"), TipoCue::Video, n, true, Some(c));
            }
        }
    }
    let mut show = Show { nombre: "Función simulada".into(), cues, ..Default::default() };
    show.ajustes.fundir_todo_ms = 1500;
    let ruta = show_dir.join("Función simulada.otashow");
    otaconvert_show::guardar(&show, &ruta, &otaconvert_show::marca_de_tiempo()).map_err(|e| e.to_string())?;
    Ok((ruta, colores))
}

fn estado(s: &Arc<Mutex<Sesion>>) -> Value {
    s.lock().ok().and_then(|s| serde_json::from_str(&s.estado_json()).ok()).unwrap_or(Value::Null)
}

/// Lo que tendría que verse según la sesión.
fn esperado(e: &Value, colores: &[Option<u32>]) -> Esperado {
    let visual = e["sonando"].as_array().and_then(|l| l.iter().rev().find(|s| s["visual"].as_bool() == Some(true)).cloned());
    match visual {
        None => Esperado::Negro,
        Some(s) => {
            let c = s["cue"].as_u64().and_then(|i| colores.get(i as usize).copied().flatten()).unwrap_or(0);
            if s["fundiendo"].as_bool() == Some(true) {
                Esperado::Fundiendo(c)
            } else {
                Esperado::Color(c)
            }
        }
    }
}

#[cfg(windows)]
mod win {
    use windows_sys::Win32::Foundation::{CloseHandle, POINT, RECT};
    use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetAncestor, GetClassNameW, GetCursorInfo, GetCursorPos, GetForegroundWindow, GetWindowRect, SetCursorPos,
        WindowFromPoint, CURSORINFO, CURSOR_SHOWING, GA_ROOT,
    };

    pub fn clase_raiz_de(h: windows_sys::Win32::Foundation::HWND) -> String {
        // SAFETY: solo lee el nombre de clase de una ventana (si no existe, devuelve 0).
        unsafe {
            if h.is_null() {
                return String::new();
            }
            let r = GetAncestor(h, GA_ROOT);
            let mut b = [0u16; 256];
            let n = GetClassNameW(r, b.as_mut_ptr(), 256);
            String::from_utf16_lossy(&b[..n.max(0) as usize])
        }
    }

    /// Clase de la ventana de arriba de todo en ese punto de la pantalla.
    pub fn clase_en(x: i32, y: i32) -> String {
        // SAFETY: consulta sin efectos.
        clase_raiz_de(unsafe { WindowFromPoint(POINT { x, y }) })
    }

    pub fn clase_al_frente() -> String {
        // SAFETY: consulta sin efectos.
        clase_raiz_de(unsafe { GetForegroundWindow() })
    }

    /// Centro de la barra de tareas principal, si está.
    pub fn centro_barra() -> Option<(i32, i32)> {
        let clase: Vec<u16> = "Shell_TrayWnd\0".encode_utf16().collect();
        // SAFETY: busca una ventana por clase y lee su rectángulo.
        unsafe {
            let h = FindWindowW(clase.as_ptr(), std::ptr::null());
            if h.is_null() {
                return None;
            }
            let mut r: RECT = std::mem::zeroed();
            GetWindowRect(h, &mut r);
            (r.right > r.left && r.bottom > r.top).then_some(((r.left + r.right) / 2, (r.top + r.bottom) / 2))
        }
    }

    pub fn raton() -> (i32, i32) {
        let mut p = POINT { x: 0, y: 0 };
        // SAFETY: escribe en `p`.
        unsafe { GetCursorPos(&mut p) };
        (p.x, p.y)
    }

    pub fn mover_raton(x: i32, y: i32) {
        // SAFETY: mueve el puntero.
        unsafe { SetCursorPos(x, y) };
    }

    /// ¿Se ve el puntero? (con el puntero sobre la proyección no tendría que verse).
    pub fn raton_visible() -> bool {
        // SAFETY: estructura con su tamaño.
        unsafe {
            let mut c: CURSORINFO = std::mem::zeroed();
            c.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
            if GetCursorInfo(&mut c) == 0 {
                return true;
            }
            c.flags & CURSOR_SHOWING != 0 && !c.hCursor.is_null()
        }
    }

    pub fn clic() {
        let ev = |f| INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: f, time: 0, dwExtraInfo: 0 } },
        };
        let entradas = [ev(MOUSEEVENTF_LEFTDOWN), ev(MOUSEEVENTF_LEFTUP)];
        // SAFETY: dos eventos del ratón en el lugar donde está.
        unsafe { SendInput(2, entradas.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    }

    /// Memoria privada (MB) de un proceso; `None` = este proceso.
    pub fn memoria_mb(pid: Option<u32>) -> Option<f64> {
        // SAFETY: abre el proceso solo para leer su memoria y lo cierra.
        unsafe {
            let (h, propio) = match pid {
                Some(p) => (OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, 0, p), false),
                None => (GetCurrentProcess(), true),
            };
            if h.is_null() {
                return None;
            }
            let mut m: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            m.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            let ok = GetProcessMemoryInfo(h, &mut m, m.cb);
            if !propio {
                CloseHandle(h);
            }
            (ok != 0).then(|| m.PagefileUsage as f64 / 1_048_576.0)
        }
    }

    pub struct Pantalla(windows_sys::Win32::Graphics::Gdi::HDC);
    impl Pantalla {
        pub fn nueva() -> Self {
            // SAFETY: DC de toda la pantalla; se libera en Drop.
            Self(unsafe { windows_sys::Win32::Graphics::Gdi::GetDC(std::ptr::null_mut()) })
        }
        pub fn pixel(&self, x: i32, y: i32) -> super::Rgb {
            // SAFETY: lee un píxel con el DC propio.
            let c = unsafe { windows_sys::Win32::Graphics::Gdi::GetPixel(self.0, x, y) };
            ((c & 0xFF) as i32, ((c >> 8) & 0xFF) as i32, ((c >> 16) & 0xFF) as i32)
        }
    }
    impl Drop for Pantalla {
        fn drop(&mut self) {
            // SAFETY: libera el DC pedido en `nueva`.
            unsafe { windows_sys::Win32::Graphics::Gdi::ReleaseDC(std::ptr::null_mut(), self.0) };
        }
    }
}

struct Muestra {
    t: Instant,
    centro: Rgb,
    esquinas: Option<[Rgb; 4]>,
}

/// Máxima segunda diferencia del contenido de un medio (sin el principio ni
/// el final, que en el show se suavizan con los fundidos).
fn aspereza_del_contenido(ruta: &Path, fs: u32) -> Result<f32, String> {
    let p = otaconvert_audio::preparar::preparar(ruta, fs).map_err(|e| format!("{}: {}", ruta.display(), e.texto(otaconvert_audio::consejos::Idioma::Es)))?;
    let ch = p.pista.canales() as usize;
    let m = p.pista.muestras();
    let borde = (fs as usize / 20) * ch;
    let mut maxima = 0f32;
    if m.len() > 2 * borde + 3 * ch {
        for i in (borde + 2 * ch)..(m.len() - borde) {
            let d = (m[i] - 2.0 * m[i - ch] + m[i - 2 * ch]).abs();
            maxima = maxima.max(d);
        }
    }
    Ok(maxima)
}

pub fn ejecutar(segundos: u64, carpeta: Option<&Path>, pantalla: Option<&str>, salida: Option<&str>) -> Result<bool, String> {
    let carpeta = carpeta.ok_or("falta --carpeta <carpeta con vídeos RRGGBB.mp4 con sonido>")?;
    let lista = crate::pantallas::listar();
    // Sin elegir: el proyector si hay; si no, a pantalla completa en la única pantalla.
    let pantalla_usada = match pantalla {
        Some(p) => p.to_string(),
        None => match lista.iter().position(|p| !p.principal) {
            Some(i) => (i + 1).to_string(),
            None => "1".into(),
        },
    };
    let completa = pantalla_usada != "vista";
    let rect = if completa {
        let n: usize = pantalla_usada.parse().map_err(|_| "--pantalla: usa un número o \"vista\"")?;
        let p = lista.get(n.wrapping_sub(1)).ok_or("esa pantalla no existe")?;
        Some((p.x, p.y, p.ancho, p.alto))
    } else {
        None
    };

    println!("Función simulada de {segundos} s. Se verán colores en la pantalla y sonarán tonos suaves de 440 Hz.");
    let (ruta_show, colores) = armar_show(carpeta)?;
    println!("Show de prueba: {}", ruta_show.display());
    let mut sesion = Sesion::abrir(&ruta_show).map_err(|e| e.to_string())?;

    // Sonido: la misma salida que usa el programa, y una grabación de lo que sale.
    let principal = crate::sonido_cpal::abrir_salida(salida, 50)?;
    let fs = principal.frecuencia;
    println!("Sonido: {} · {fs} Hz", principal.nombre);
    let dispositivo = crate::elegir_salida(salida)?;
    let cfg = dispositivo.default_output_config().map_err(|e| e.to_string())?;
    let canales = cfg.channels() as usize;
    let grabado = Arc::new(Mutex::new(Vec::<f32>::with_capacity((segundos as usize + 20) * fs as usize * canales)));
    let inicio_grabacion: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    // Cuándo avisó la grabadora (segundos desde que empezó a grabar).
    let errores_grabacion: Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));
    let (g2, i2, e2, i3) = (grabado.clone(), inicio_grabacion.clone(), errores_grabacion.clone(), inicio_grabacion.clone());
    let grabacion = dispositivo
        .build_input_stream(
            // Búfer grande para la grabación: si la grabación pierde un trozo, no es
            // un corte de lo que sale (eso lo cuenta aparte la salida: "llamadas tardías").
            cpal::StreamConfig {
                channels: cfg.channels(),
                sample_rate: cfg.sample_rate(),
                buffer_size: cpal::BufferSize::Fixed(cfg.sample_rate() / 5),
            },
            move |d: &[f32], _: &cpal::InputCallbackInfo| {
                if let Ok(mut g) = g2.lock() {
                    if g.is_empty() {
                        if let Ok(mut t) = i2.lock() {
                            *t = Some(Instant::now());
                        }
                    }
                    g.extend_from_slice(d);
                }
            },
            move |e| {
                let s = i3.lock().ok().and_then(|t| *t).map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
                if let Ok(mut v) = e2.lock() {
                    v.push(s);
                }
                println!("Aviso de la grabación a los {s:.2} s: {e}");
            },
            None,
        )
        .map_err(|e| format!("no se pudo grabar la salida: {e}"))?;
    grabacion.play().map_err(|e| e.to_string())?;
    let (tardias, intervalo_max_us) = (principal.tardias.clone(), principal.intervalo_max_us.clone());
    sesion.conectar_sonido(Box::new(crate::sonido_cpal::SonidoCpal { principal: principal.control, frecuencia: fs, auriculares: None }));

    let proyector = crate::show_cmd::abrir_proyeccion(Some(&pantalla_usada))?;
    let pid_proyector = proyector.pid();
    sesion.conectar_proyector(Box::new(proyector));
    let sesion = Arc::new(Mutex::new(sesion));
    let parar = Arc::new(AtomicBool::new(false));

    // Los mismos hilos que el servidor: preparar en segundo plano y refrescar.
    let (s1, p1) = (sesion.clone(), parar.clone());
    let preparador = std::thread::spawn(move || {
        while !p1.load(Relaxed) {
            let tarea = s1.lock().ok().and_then(|mut s| s.siguiente_a_preparar());
            match tarea {
                Some(t) => {
                    let r = t.hacer();
                    if let Ok(mut s) = s1.lock() {
                        s.resultado_preparar(t.cue, r);
                    }
                }
                None => std::thread::sleep(Duration::from_millis(100)),
            }
        }
    });
    let (s2, p2) = (sesion.clone(), parar.clone());
    let refresco = std::thread::spawn(move || {
        while !p2.load(Relaxed) {
            std::thread::sleep(Duration::from_millis(100));
            if let Ok(mut s) = s2.lock() {
                s.limpiar();
            }
        }
    });

    let mut informe: Vec<String> = Vec::new();
    let mut problemas = 0u32;
    let mut bien = |informe: &mut Vec<String>, ok: bool, texto: String| {
        informe.push(format!("{}  {texto}", if ok { "BIEN    " } else { "PROBLEMA" }));
        if !ok {
            problemas += 1;
        }
    };

    // Como el programa: con el show abierto, el equipo no se suspende ni apaga la pantalla.
    let pedido = crate::despierto::mantener();
    #[cfg(windows)]
    {
        let estado = crate::despierto::estado_del_sistema();
        bien(
            &mut informe,
            pedido && estado == Some((true, true)),
            format!(
                "Suspensión y pantalla: pedido a Windows {} · Windows lo tiene registrado: {}",
                if pedido { "aceptado" } else { "NO aceptado" },
                match estado {
                    Some((s, p)) => format!("no suspender {} · no apagar la pantalla {}", if s { "sí" } else { "NO" }, if p { "sí" } else { "NO" }),
                    None => "no se pudo consultar".into(),
                }
            ),
        );
        match crate::despierto::protector_de_pantalla() {
            Some((n, m)) => informe.push(format!("DATO      Protector de pantalla «{n}» a los {m} min: el programa avisa al abrir el show")),
            None => informe.push("DATO      Sin protector de pantalla configurado".into()),
        }
    }

    // Lo que hay que comparar: dónde mirar.
    let (cx, cy, puntos) = match rect {
        Some((x, y, w, h)) => (x + w / 2, y + h / 2, Some([(x + 4, y + 4), (x + w - 5, y + 4), (x + 4, y + h - 5), (x + w - 5, y + h - 5)])),
        None => {
            // Vista previa: el centro de su ventana (arriba a la derecha de la pantalla principal).
            let base = lista.iter().find(|p| p.principal).or(lista.first()).ok_or("no hay pantallas")?;
            let w = (base.ancho / 4).max(320);
            let h = w * 9 / 16 + 32;
            (base.x + base.ancho - w - 24 + w / 2, base.y + 140 + h / 2 + 12, None)
        }
    };

    // Esperar que todo esté listo (la proyección precarga, el sonido se prepara).
    let fin_espera = Instant::now() + Duration::from_secs(20);
    while Instant::now() < fin_espera {
        let e = estado(&sesion);
        if e["cues"][0]["preparacion"] == "listo" && e["cues"][1]["preparacion"] == "listo" {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Pantalla completa: tapa todo, sin ratón, sin robar el teclado.
    #[cfg(windows)]
    if let (Some((x, y, w, h)), Some(pts)) = (rect, puntos) {
        let mut tapan = vec![(cx, cy)];
        tapan.extend_from_slice(&pts);
        let barra = win::centro_barra().filter(|&(bx, by)| bx >= x && bx < x + w && by >= y && by < y + h);
        if let Some(b) = barra {
            tapan.push(b);
        }
        let fuera: Vec<String> =
            tapan.iter().map(|&(a, b)| (a, b, win::clase_en(a, b))).filter(|(_, _, c)| c != CLASE_PROYECCION).map(|(a, b, c)| format!("({a},{b}) {c}")).collect();
        bien(
            &mut informe,
            fuera.is_empty(),
            format!(
                "Pantalla completa: tapa el centro, las 4 esquinas{} {}",
                if barra.is_some() { " y la barra de tareas" } else { "" },
                if fuera.is_empty() { "".into() } else { format!("· asoma: {}", fuera.join(", ")) }
            ),
        );
        if fuera.is_empty() {
            let (rx, ry) = win::raton();
            let antes = win::clase_al_frente();
            win::mover_raton(cx, cy);
            std::thread::sleep(Duration::from_millis(400));
            let visible = win::raton_visible();
            bien(&mut informe, !visible, format!("Ratón sobre la proyección: {}", if visible { "SE VE" } else { "no se ve" }));
            // Solo se hace clic si lo que está debajo es la proyección (nunca otra ventana del equipo).
            if win::clase_en(cx, cy) == CLASE_PROYECCION {
                win::clic();
                std::thread::sleep(Duration::from_millis(400));
                let despues = win::clase_al_frente();
                bien(
                    &mut informe,
                    despues != CLASE_PROYECCION,
                    format!("Clic sobre la proyección: el teclado sigue en «{despues}» (antes «{antes}»)"),
                );
            }
            win::mover_raton(rx, ry);
        }
    }
    #[cfg(not(windows))]
    let _ = puntos;

    // Quien mira la pantalla, todo el tiempo.
    let muestras = Arc::new(Mutex::new(Vec::<Muestra>::with_capacity(segundos as usize * 80 + 1000)));
    let barra_tapada_fallos = Arc::new(AtomicU64::new(0));
    let (m3, p3, b3) = (muestras.clone(), parar.clone(), barra_tapada_fallos.clone());
    let barra = if completa {
        #[cfg(windows)]
        {
            win::centro_barra().filter(|&(bx, by)| rect.is_some_and(|(x, y, w, h)| bx >= x && bx < x + w && by >= y && by < y + h))
        }
        #[cfg(not(windows))]
        {
            None
        }
    } else {
        None
    };
    let mirador = std::thread::spawn(move || {
        #[cfg(windows)]
        {
            let pantalla = win::Pantalla::nueva();
            let mut n = 0u64;
            while !p3.load(Relaxed) {
                let centro = pantalla.pixel(cx, cy);
                // Las esquinas, una vez de cada 8 (cada lectura espera a la pantalla).
                let esquinas = match puntos {
                    Some(pts) if n % 8 == 0 => Some(pts.map(|(a, b)| pantalla.pixel(a, b))),
                    _ => None,
                };
                if let (Some((bx, by)), true) = (barra, n % 50 == 0) {
                    if win::clase_en(bx, by) != CLASE_PROYECCION {
                        b3.fetch_add(1, Relaxed);
                    }
                }
                if let Ok(mut m) = m3.lock() {
                    m.push(Muestra { t: Instant::now(), centro, esquinas });
                }
                n += 1;
            }
        }
        #[cfg(not(windows))]
        let _ = (m3, p3, b3, barra);
    });

    // La función: acciones al azar, siempre las mismas.
    let mut azar = Azar(0x5EED_0CA1_7EA7_0001);
    let mut linea: Vec<(Instant, Esperado)> = vec![(Instant::now(), esperado(&estado(&sesion), &colores))];
    let mut acciones: Vec<(Instant, String)> = Vec::new();
    let mut gos = 0u32;
    let mut go_ignorados = 0u32;
    let mut no_preparados = 0u32;
    let mut avisos: Vec<String> = Vec::new();
    let inicio = Instant::now();
    let mut memoria_inicial = None;
    // (segundo, motor MB, proyección MB) cada 30 s: para ver si crece sin parar o se estabiliza.
    let mut serie_memoria: Vec<(f64, f64, f64)> = Vec::new();
    let mirar = |linea: &mut Vec<(Instant, Esperado)>, avisos: &mut Vec<String>, durante: Duration| {
        let fin = Instant::now() + durante;
        loop {
            let e = estado(&sesion);
            let ahora = esperado(&e, &colores);
            if linea.last().map(|l| l.1) != Some(ahora) {
                linea.push((Instant::now(), ahora));
            }
            if let Some(a) = e["aviso"].as_str() {
                if !avisos.iter().any(|x| x == a) {
                    avisos.push(a.to_string());
                }
            }
            if Instant::now() >= fin {
                break;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
    };
    while inicio.elapsed() < Duration::from_secs(segundos) {
        #[cfg(windows)]
        if inicio.elapsed().as_secs_f64() >= serie_memoria.len() as f64 * 30.0 {
            serie_memoria.push((
                inicio.elapsed().as_secs_f64(),
                win::memoria_mb(None).unwrap_or(0.0),
                pid_proyector.and_then(|p| win::memoria_mb(Some(p))).unwrap_or(0.0),
            ));
        }
        if memoria_inicial.is_none() && inicio.elapsed() > Duration::from_secs(15) {
            #[cfg(windows)]
            {
                memoria_inicial = Some((win::memoria_mb(None), pid_proyector.and_then(|p| win::memoria_mb(Some(p)))));
            }
            #[cfg(not(windows))]
            {
                memoria_inicial = Some((None::<f64>, None::<f64>));
            }
        }
        let e = estado(&sesion);
        let n_cues = colores.len();
        let espera = e["en_espera"].as_u64().unwrap_or(0) as usize;
        let ambiente_suena = e["sonando"].as_array().is_some_and(|l| l.iter().any(|s| s["cue"].as_u64() == Some(0)));
        // Al final de la lista se vuelve arriba; el ambiente solo si no suena.
        if espera >= n_cues {
            let a = if ambiente_suena { 1 } else { 0 };
            sesion.lock().map_err(|_| "sesión")?.ir_a(a);
            acciones.push((Instant::now(), format!("ELEGIR {}", a + 1)));
            continue;
        }
        let tirada = azar.hasta(100);
        let accion = match tirada {
            0..=54 => "go",
            55..=62 => "pausa",
            63..=67 => "fundir_todo",
            68..=75 => "parar",
            76..=83 => "fundir_ultimo",
            84..=91 => "elegir",
            _ => "desde",
        };
        match accion {
            "go" => {
                // Como un operador: espera a que el cue esté listo (no da GO a ciegas).
                let limite = Instant::now() + Duration::from_secs(8);
                let mut listo = false;
                while Instant::now() < limite {
                    let e = estado(&sesion);
                    if e["cues"][espera]["preparacion"] == "listo" {
                        listo = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                if !listo {
                    no_preparados += 1;
                }
                let disparo = sesion.lock().map_err(|_| "sesión")?.go();
                gos += 1;
                if !disparo {
                    go_ignorados += 1;
                }
                acciones.push((Instant::now(), format!("GO {}{}", espera + 1, if disparo { "" } else { " (ignorado)" })));
            }
            "pausa" => {
                sesion.lock().map_err(|_| "sesión")?.pausa();
                acciones.push((Instant::now(), "PAUSA".into()));
                mirar(&mut linea, &mut avisos, azar.entre_ms(400, 1500));
                sesion.lock().map_err(|_| "sesión")?.pausa();
                acciones.push((Instant::now(), "SIGUE".into()));
            }
            "fundir_todo" => {
                sesion.lock().map_err(|_| "sesión")?.fundir_todo();
                acciones.push((Instant::now(), "FUNDIR TODO".into()));
                mirar(&mut linea, &mut avisos, Duration::from_millis(1900));
            }
            "parar" => {
                sesion.lock().map_err(|_| "sesión")?.parar_todo();
                acciones.push((Instant::now(), "PARAR TODO".into()));
            }
            "fundir_ultimo" => {
                sesion.lock().map_err(|_| "sesión")?.fundir_ultimo();
                acciones.push((Instant::now(), "MAYUS+F".into()));
            }
            "elegir" => {
                // La mitad de las veces, el vídeo o imagen que está al aire (para
                // repetirlo con el próximo GO, como en la Vista Estudio).
                let al_aire = e["sonando"]
                    .as_array()
                    .and_then(|l| l.iter().rev().find(|s| s["visual"].as_bool() == Some(true)))
                    .and_then(|s| s["cue"].as_u64())
                    .map(|c| c as usize);
                let a = match al_aire {
                    Some(c) if azar.hasta(2) == 0 => c,
                    _ => 1 + azar.hasta(n_cues as u64 - 1) as usize,
                };
                sesion.lock().map_err(|_| "sesión")?.ir_a(a);
                acciones.push((Instant::now(), format!("ELEGIR {}{}", a + 1, if al_aire == Some(a) { " (el que está al aire)" } else { "" })));
            }
            _ => {
                if colores.get(espera).copied().flatten().is_some() && e["cues"][espera]["tipo"] == "video" {
                    let s = 0.5 + azar.hasta(450) as f64 / 100.0;
                    let r = sesion.lock().map_err(|_| "sesión")?.elegir_desde(espera, Some(s));
                    acciones.push((Instant::now(), format!("DESDE {} {s:.2} s{}", espera + 1, if r.is_ok() { "" } else { " (rechazado)" })));
                }
            }
        }
        mirar(&mut linea, &mut avisos, azar.entre_ms(450, 2500));
    }

    // Final: Parar todo y negro.
    sesion.lock().map_err(|_| "sesión")?.parar_todo();
    acciones.push((Instant::now(), "PARAR TODO (final)".into()));
    mirar(&mut linea, &mut avisos, Duration::from_millis(1500));
    #[cfg(windows)]
    let memoria_final = (win::memoria_mb(None), pid_proyector.and_then(|p| win::memoria_mb(Some(p))));
    #[cfg(not(windows))]
    let memoria_final = (None::<f64>, None::<f64>);
    parar.store(true, Relaxed);
    let _ = mirador.join();
    drop(grabacion);
    let _ = preparador.join();
    let _ = refresco.join();

    // — La imagen —
    let muestras = std::mem::take(&mut *muestras.lock().map_err(|_| "muestras")?);
    let (mut negros, mut raros, mut esquinas_mal, mut n_esq) = (0u64, 0u64, 0u64, 0u64);
    let mut ejemplos: Vec<String> = Vec::new();
    let t_cero = inicio;
    for m in &muestras {
        let k = linea.partition_point(|(t, _)| *t <= m.t);
        if k == 0 {
            continue;
        }
        let (t_cambio, ahora) = linea[k - 1];
        let antes = if k >= 2 { Some(linea[k - 2].1) } else { None };
        let en_cambio = m.t.duration_since(t_cambio) < VENTANA_CAMBIO;
        let ok = |p: Rgb| {
            vale(p, ahora)
                || (en_cambio && antes.is_some_and(|a| vale(p, a)))
                // Al salir del negro (primer GO o tras Parar todo) el negro vale un momento.
                || (en_cambio && antes == Some(Esperado::Negro) && negro(p))
        };
        if !ok(m.centro) {
            if negro(m.centro) {
                negros += 1;
            } else {
                raros += 1;
            }
            if ejemplos.len() < 8 {
                ejemplos.push(format!(
                    "a los {:.2} s: se vio {:?}, tocaba {:?}",
                    m.t.duration_since(t_cero).as_secs_f64(),
                    m.centro,
                    ahora
                ));
            }
        }
        if let Some(es) = m.esquinas {
            n_esq += 1;
            if es.iter().any(|&p| !ok(p)) {
                esquinas_mal += 1;
            }
        }
    }
    let duracion_mirada = muestras.last().zip(muestras.first()).map(|(a, b)| a.t.duration_since(b.t).as_secs_f64()).unwrap_or(1.0);
    let cambios_visuales = linea.len().saturating_sub(1);
    bien(
        &mut informe,
        negros == 0 && raros == 0,
        format!(
            "Imagen: {} muestras ({:.0} por segundo) en {} cambios · negros que no tocaban {negros} · colores que no tocaban {raros}",
            muestras.len(),
            muestras.len() as f64 / duracion_mirada.max(0.001),
            cambios_visuales
        ),
    );
    for e in &ejemplos {
        informe.push(format!("          {e}"));
    }
    if completa {
        bien(&mut informe, esquinas_mal == 0, format!("Esquinas de la pantalla: {n_esq} muestras · mal {esquinas_mal}"));
        let b = barra_tapada_fallos.load(Relaxed);
        if barra.is_some() {
            bien(&mut informe, b == 0, format!("Barra de tareas siempre tapada durante la función (veces que asomó: {b})"));
        }
    }

    // — El sonido que salió de verdad —
    let grabado = std::mem::take(&mut *grabado.lock().map_err(|_| "grabación")?);
    let t_grab = inicio_grabacion.lock().ok().and_then(|t| *t);
    let cuadros = grabado.len() / canales.max(1);
    let teorico = |a: f32| otaconvert_audio::Detector::umbral_para_tonos(a, HZ, fs);
    let mut aspereza = 0f32;
    let carpeta_show = ruta_show.parent().map(Path::to_path_buf).unwrap_or_default();
    for c in colores.iter().enumerate().filter_map(|(i, _)| sesion.lock().ok().and_then(|s| s.show.cues.get(i).map(|c| c.archivo.clone()))) {
        if let Ok(a) = aspereza_del_contenido(&carpeta_show.join(&c), fs) {
            aspereza = aspereza.max(a);
        }
    }
    let umbral = teorico(AMP_TOTAL) + 4.0 * (aspereza - teorico(AMP)).max(0.0);
    let maximo = grabado.iter().fold(0f32, |m, x| m.max(x.abs()));
    if cuadros < fs as usize || maximo < 0.01 {
        informe.push(format!(
            "NO MEDIDO  Sonido grabado: {cuadros} cuadros, pico {maximo:.3} (¿salida en silencio o volumen de Windows en 0?)"
        ));
    } else {
        let mut eventos: Vec<usize> = Vec::new();
        let mut max_d = 0f32;
        for f in 2..cuadros {
            let mut peor = 0f32;
            for c in 0..canales {
                let x = |k: usize| grabado[k * canales + c];
                peor = peor.max((x(f) - 2.0 * x(f - 1) + x(f - 2)).abs());
            }
            max_d = max_d.max(peor);
            if peor > umbral && eventos.last().is_none_or(|&u| f - u > fs as usize / 100) {
                eventos.push(f);
            }
        }
        let esperados_s = segundos as f64 + 2.0;
        let grabados_s = cuadros as f64 / fs as f64;
        bien(
            &mut informe,
            eventos.is_empty(),
            format!(
                "Sonido: {grabados_s:.1} s grabados de la salida · clics o cortes {} · máxima {max_d:.4} (umbral {umbral:.4}; contenido {aspereza:.4}) · pico {maximo:.3}",
                eventos.len()
            ),
        );
        for &f in eventos.iter().take(8) {
            let t = f as f64 / fs as f64;
            let cerca = t_grab
                .map(|tg| {
                    let instante = tg + Duration::from_secs_f64(t);
                    acciones
                        .iter()
                        .min_by_key(|(ta, _)| if *ta > instante { *ta - instante } else { instante - *ta })
                        .map(|(ta, a)| {
                            let dif = ta.duration_since(tg).as_secs_f64() - t;
                            format!("acción más cercana: {a} ({:+.0} ms)", dif * 1000.0)
                        })
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            informe.push(format!("          a los {t:.3} s de grabación · {cerca}"));
        }
        // Para revisar a mano: la grabación y la lista de acciones con su hora.
        if let Some(tg) = t_grab {
            let mut txt = String::new();
            for (ta, a) in &acciones {
                let s = if *ta >= tg { ta.duration_since(tg).as_secs_f64() } else { -tg.duration_since(*ta).as_secs_f64() };
                txt.push_str(&format!("{s:9.3}  {a}\n"));
            }
            txt.push_str("\nClics o cortes (s de grabación):\n");
            for &f in &eventos {
                txt.push_str(&format!("{:9.3}\n", f as f64 / fs as f64));
            }
            let _ = std::fs::write(carpeta_show.join("acciones.txt"), txt);
            let _ = guardar_wav_f32(&carpeta_show.join("grabacion.wav"), &grabado, canales as u16, fs);
            informe.push(format!("DATO      Grabación y acciones guardadas en {}", carpeta_show.display()));
        }
        // Windows marca el primer paquete de una grabación nueva como "discontinuo":
        // es el arranque de la grabadora, no un corte. Solo cuentan los de después.
        let avisos_grabacion = errores_grabacion.lock().map(|v| v.clone()).unwrap_or_default();
        let al_arrancar = avisos_grabacion.iter().filter(|&&s| s < 1.0).count();
        let despues: Vec<String> = avisos_grabacion.iter().filter(|&&s| s >= 1.0).map(|s| format!("{s:.2} s")).collect();
        bien(
            &mut informe,
            despues.is_empty() && grabados_s > esperados_s - 3.0,
            format!(
                "Grabación continua: {grabados_s:.1} s de {esperados_s:.0} s esperados · avisos de la grabadora al arrancar {al_arrancar}, después {}",
                if despues.is_empty() { "0".into() } else { despues.join(", ") }
            ),
        );
    }

    let t = tardias.load(Relaxed);
    bien(
        &mut informe,
        t == 0,
        format!(
            "Salida de sonido: llamadas tardías de la tarjeta {t} · mayor espera entre llamadas {:.1} ms (margen 50 ms)",
            intervalo_max_us.load(Relaxed) as f64 / 1000.0
        ),
    );

    // — Lo demás —
    bien(
        &mut informe,
        go_ignorados == 0 && no_preparados == 0,
        format!("GO: {gos} · ignorados {go_ignorados} · cues que no estaban listos a tiempo {no_preparados}"),
    );
    if let (Some((Some(m0), p0)), (Some(m1), p1)) = (memoria_inicial, memoria_final) {
        let minutos = (segundos as f64 - 15.0).max(1.0) / 60.0;
        let crece = (m1 - m0) / minutos;
        let texto_p = match (p0, p1) {
            (Some(a), Some(b)) => format!(" · proyección {a:.0} → {b:.0} MB"),
            _ => String::new(),
        };
        let crece_p = match (p0, p1) {
            (Some(a), Some(b)) => (b - a) / minutos,
            _ => 0.0,
        };
        // Con varios minutos medidos, manda la segunda mitad: al principio la
        // memoria sube mientras se llenan las reservas de los decodificadores;
        // lo que importa es que después se quede quieta.
        let (ok, detalle) = if serie_memoria.len() >= 8 {
            let medio = serie_memoria[serie_memoria.len() / 2];
            let ultimo = serie_memoria[serie_memoria.len() - 1];
            let min = ((ultimo.0 - medio.0) / 60.0).max(0.5);
            let (cm, cp) = ((ultimo.1 - medio.1) / min, (ultimo.2 - medio.2) / min);
            (cm < 5.0 && cp < 5.0, format!(" · en la segunda mitad crece {cm:.1} y {cp:.1} MB por minuto"))
        } else {
            (true, String::new())
        };
        let texto = format!("Memoria: motor {m0:.0} → {m1:.0} MB{texto_p} (crece {crece:.1} y {crece_p:.1} MB por minuto){detalle}");
        if serie_memoria.len() >= 8 {
            bien(&mut informe, ok, texto);
        } else {
            // En pocos minutos la memoria todavía se está llenando (sube y baja):
            // no alcanza para juzgar. Con 15 min se vio que se estabiliza.
            informe.push(format!("DATO      {texto} · para juzgarla hacen falta 4 minutos o más"));
        }
        if !serie_memoria.is_empty() {
            let s: Vec<String> = serie_memoria.iter().map(|(t, _, p)| format!("{:.0}′{:.0}", t / 60.0, p)).collect();
            informe.push(format!("DATO      Memoria de la proyección cada 30 s (minuto′MB): {}", s.join(" ")));
        }
    }
    let registro = std::fs::read_dir(carpeta_show.join("registros"))
        .ok()
        .and_then(|l| l.flatten().next())
        .and_then(|e| std::fs::read_to_string(e.path()).ok())
        .unwrap_or_default();
    let errores_registro: Vec<&str> = registro.lines().filter(|l| l.contains("ERROR")).collect();
    bien(
        &mut informe,
        errores_registro.is_empty(),
        format!("Registro de la función: {} líneas · errores {}", registro.lines().count(), errores_registro.len()),
    );
    for l in errores_registro.iter().take(5) {
        informe.push(format!("          {l}"));
    }
    if !avisos.is_empty() {
        informe.push(format!("DATO      Avisos que mostró el programa: {}", avisos.join(" | ")));
    }

    println!("— Informe —");
    for l in &informe {
        println!("{l}");
    }
    println!("{}", if problemas == 0 { "RESULTADO: BIEN" } else { "RESULTADO: HAY PROBLEMAS" });
    Ok(problemas == 0)
}
