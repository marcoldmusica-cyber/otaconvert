//! libmpv cargada a mano (LoadLibraryExW + GetProcAddress).
//!
//! No se enlaza con una biblioteca de importación: el exe solo busca sus
//! importaciones en System32 (`/DEPENDENTLOADFLAG:0x800`), así que
//! `libmpv-2.dll` se abre con su ruta completa desde la carpeta del programa.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use windows_sys::Win32::Foundation::{GetLastError, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    LOAD_LIBRARY_SEARCH_SYSTEM32,
};

pub const DLL: &str = "libmpv-2.dll";

type Handle = *mut c_void;

#[repr(C)]
struct EventoC {
    event_id: c_int,
    error: c_int,
    reply_userdata: u64,
    data: *mut c_void,
}

struct Api {
    client_api_version: unsafe extern "C" fn() -> u32,
    error_string: unsafe extern "C" fn(c_int) -> *const c_char,
    free: unsafe extern "C" fn(*mut c_void),
    create: unsafe extern "C" fn() -> Handle,
    initialize: unsafe extern "C" fn(Handle) -> c_int,
    terminate_destroy: unsafe extern "C" fn(Handle),
    set_option_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    set_property_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    get_property_string: unsafe extern "C" fn(Handle, *const c_char) -> *mut c_char,
    command: unsafe extern "C" fn(Handle, *mut *const c_char) -> c_int,
    wait_event: unsafe extern "C" fn(Handle, f64) -> *mut EventoC,
    request_log_messages: unsafe extern "C" fn(Handle, *const c_char) -> c_int,
}

/// La biblioteca cargada. Se comparte entre reproductores y no se descarga.
pub struct Libmpv {
    api: Api,
    pub ruta: PathBuf,
    pub version: (u32, u32),
}

// SAFETY: solo contiene punteros a funciones de una DLL que no se descarga.
unsafe impl Send for Libmpv {}
unsafe impl Sync for Libmpv {}

/// Ruta de `libmpv-2.dll` junto al ejecutable.
pub fn ruta_junto_al_exe() -> Result<PathBuf, String> {
    let mut buf = vec![0u16; 32768];
    // SAFETY: buffer con su largo; módulo nulo = el exe.
    let n = unsafe { GetModuleFileNameW(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if n == 0 {
        return Err("no se pudo saber la carpeta del programa".into());
    }
    let exe = PathBuf::from(String::from_utf16_lossy(&buf[..n]));
    Ok(exe.with_file_name(DLL))
}

fn mensaje(api: &Api, codigo: c_int) -> String {
    // SAFETY: mpv devuelve una cadena estática para cualquier código.
    unsafe { CStr::from_ptr((api.error_string)(codigo)) }.to_string_lossy().into_owned()
}

impl Libmpv {
    pub fn cargar(ruta: &Path) -> Result<Arc<Self>, String> {
        if !ruta.is_absolute() {
            return Err(format!("{} debe cargarse con ruta completa", DLL));
        }
        let ancha: Vec<u16> = ruta.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        // SAFETY: ruta terminada en 0. Las dependencias de la DLL solo se buscan
        // en su propia carpeta y en System32.
        let modulo: HMODULE = unsafe {
            LoadLibraryExW(
                ancha.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        if modulo.is_null() {
            // SAFETY: sin argumentos.
            let e = unsafe { GetLastError() };
            return Err(format!("no se pudo cargar {} (error de Windows {e})", ruta.display()));
        }
        macro_rules! funcion {
            ($nombre:literal) => {{
                // SAFETY: nombre terminado en 0; la firma coincide con mpv/client.h (API 2.x).
                match unsafe { GetProcAddress(modulo, concat!($nombre, "\0").as_ptr()) } {
                    Some(p) => unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, _>(p) },
                    None => return Err(format!("{} no tiene {}", DLL, $nombre)),
                }
            }};
        }
        let api = Api {
            client_api_version: funcion!("mpv_client_api_version"),
            error_string: funcion!("mpv_error_string"),
            free: funcion!("mpv_free"),
            create: funcion!("mpv_create"),
            initialize: funcion!("mpv_initialize"),
            terminate_destroy: funcion!("mpv_terminate_destroy"),
            set_option_string: funcion!("mpv_set_option_string"),
            set_property_string: funcion!("mpv_set_property_string"),
            get_property_string: funcion!("mpv_get_property_string"),
            command: funcion!("mpv_command"),
            wait_event: funcion!("mpv_wait_event"),
            request_log_messages: funcion!("mpv_request_log_messages"),
        };
        // SAFETY: función sin argumentos.
        let v = unsafe { (api.client_api_version)() };
        let version = (v >> 16, v & 0xFFFF);
        if version.0 != 2 {
            return Err(format!("{} tiene la API {}.{}; se necesita la 2.x", DLL, version.0, version.1));
        }
        Ok(Arc::new(Self { api, ruta: ruta.to_path_buf(), version }))
    }
}

/// Lo que interesa de los eventos de mpv.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evento {
    Nada,
    Apagado,
    InicioArchivo,
    /// `true` si terminó por un error (no por cambiar de archivo o parar).
    FinArchivo(bool),
    ArchivoCargado,
    /// Reproducción lista tras cargar o saltar: el primer cuadro ya está.
    Listo,
    /// Un mensaje del registro de mpv (pedido con `pedir_mensajes`); se lee con `tomar_mensajes`.
    Mensaje,
    Otro(i32),
}

#[repr(C)]
struct MensajeC {
    prefix: *const c_char,
    level: *const c_char,
    text: *const c_char,
    log_level: c_int,
}

/// Un reproductor mpv dibujando en una ventana (una capa).
pub struct Reproductor {
    lib: Arc<Libmpv>,
    h: Handle,
    /// Mensajes del registro de mpv que llegaron (los últimos 400).
    mensajes: std::sync::Mutex<Vec<String>>,
}

// SAFETY: la API de cliente de mpv es segura entre hilos.
unsafe impl Send for Reproductor {}

fn c(s: &str) -> Result<CString, String> {
    CString::new(s).map_err(|_| format!("texto con un 0 en medio: {s:?}"))
}

/// Opciones comunes: solo archivos locales, nada de scripts, configuración
/// del usuario, internet, teclado ni cursor de mpv. El audio lo hace el motor.
pub const OPCIONES_SEGURAS: &[(&str, &str)] = &[
    ("config", "no"),
    ("load-scripts", "no"),
    ("ytdl", "no"),
    ("osc", "no"),
    ("input-default-bindings", "no"),
    ("input-vo-keyboard", "no"),
    ("input-cursor", "no"),
    ("cursor-autohide", "always"),
    ("terminal", "no"),
    ("osd-level", "0"),
    ("aid", "no"),
    ("sid", "no"),
    ("idle", "yes"),
    ("force-window", "yes"),
    ("keep-open", "always"),
    ("pause", "yes"),
    ("vo", "gpu-next"),
    ("gpu-context", "d3d11"),
    // Decodificación por la tarjeta gráfica; si no puede, mpv usa la CPU.
    ("hwdec", "d3d11va"),
];

impl Reproductor {
    /// Crea un reproductor que dibuja dentro de la ventana `wid`.
    pub fn nuevo(lib: Arc<Libmpv>, wid: isize, extra: &[(&str, &str)]) -> Result<Self, String> {
        // SAFETY: sin argumentos; se comprueba el nulo.
        let h = unsafe { (lib.api.create)() };
        if h.is_null() {
            return Err("mpv no pudo crear el reproductor".into());
        }
        let r = Self { lib, h, mensajes: std::sync::Mutex::new(Vec::new()) };
        // mpv pide el HWND como entero sin signo de 32 bits (manual de mpv, --wid).
        r.opcion("wid", &(wid as u32).to_string())?;
        for (k, v) in OPCIONES_SEGURAS.iter().chain(extra) {
            r.opcion(k, v)?;
        }
        // Fondo negro donde el vídeo no llena la pantalla (opción de versiones recientes).
        let _ = r.opcion("background-color", "#000000");
        // SAFETY: handle válido.
        let e = unsafe { (r.lib.api.initialize)(r.h) };
        if e < 0 {
            return Err(format!("mpv no pudo iniciar: {}", mensaje(&r.lib.api, e)));
        }
        Ok(r)
    }

    /// Un reproductor sin ventana (para sacar una foto de un vídeo o imagen):
    /// las mismas protecciones (sin configuración del usuario, scripts,
    /// internet, teclado ni terminal) y sin sonido; `extra` elige la salida.
    pub fn sin_ventana(lib: Arc<Libmpv>, extra: &[(&str, &str)]) -> Result<Self, String> {
        // SAFETY: sin argumentos; se comprueba el nulo.
        let h = unsafe { (lib.api.create)() };
        if h.is_null() {
            return Err("mpv no pudo crear el reproductor".into());
        }
        let r = Self { lib, h, mensajes: std::sync::Mutex::new(Vec::new()) };
        let base: &[(&str, &str)] = &[
            ("config", "no"),
            ("load-scripts", "no"),
            ("ytdl", "no"),
            ("osc", "no"),
            ("input-default-bindings", "no"),
            ("terminal", "no"),
            ("aid", "no"),
            ("sid", "no"),
            ("ao", "null"),
            ("idle", "no"),
        ];
        for (k, v) in base.iter().chain(extra) {
            r.opcion(k, v)?;
        }
        // SAFETY: handle válido.
        let e = unsafe { (r.lib.api.initialize)(r.h) };
        if e < 0 {
            return Err(format!("mpv no pudo iniciar: {}", mensaje(&r.lib.api, e)));
        }
        Ok(r)
    }

    pub fn opcion(&self, nombre: &str, valor: &str) -> Result<(), String> {
        let (n, v) = (c(nombre)?, c(valor)?);
        // SAFETY: handle válido y cadenas terminadas en 0.
        let e = unsafe { (self.lib.api.set_option_string)(self.h, n.as_ptr(), v.as_ptr()) };
        if e < 0 {
            return Err(format!("opción {nombre}={valor}: {}", mensaje(&self.lib.api, e)));
        }
        Ok(())
    }

    pub fn propiedad(&self, nombre: &str, valor: &str) -> Result<(), String> {
        let (n, v) = (c(nombre)?, c(valor)?);
        // SAFETY: como en `opcion`.
        let e = unsafe { (self.lib.api.set_property_string)(self.h, n.as_ptr(), v.as_ptr()) };
        if e < 0 {
            return Err(format!("propiedad {nombre}={valor}: {}", mensaje(&self.lib.api, e)));
        }
        Ok(())
    }

    pub fn leer(&self, nombre: &str) -> Option<String> {
        let n = c(nombre).ok()?;
        // SAFETY: mpv devuelve una copia que se libera con mpv_free.
        unsafe {
            let p = (self.lib.api.get_property_string)(self.h, n.as_ptr());
            if p.is_null() {
                return None;
            }
            let s = CStr::from_ptr(p).to_string_lossy().into_owned();
            (self.lib.api.free)(p as *mut c_void);
            Some(s)
        }
    }

    pub fn comando(&self, args: &[&str]) -> Result<(), String> {
        let cs: Vec<CString> = args.iter().map(|a| c(a)).collect::<Result<_, _>>()?;
        let mut ptrs: Vec<*const c_char> = cs.iter().map(|s| s.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        // SAFETY: lista de cadenas terminada en nulo, viva durante la llamada.
        let e = unsafe { (self.lib.api.command)(self.h, ptrs.as_mut_ptr()) };
        if e < 0 {
            return Err(format!("comando {args:?}: {}", mensaje(&self.lib.api, e)));
        }
        Ok(())
    }

    /// Espera el próximo evento hasta `segundos` (0 = no esperar).
    pub fn evento(&self, segundos: f64) -> Evento {
        // SAFETY: mpv devuelve un puntero válido hasta la próxima llamada.
        let e = unsafe { &*(self.lib.api.wait_event)(self.h, segundos) };
        match e.event_id {
            0 => Evento::Nada,
            1 => Evento::Apagado,
            6 => Evento::InicioArchivo,
            // mpv_event_end_file empieza con `int reason`; 4 = MPV_END_FILE_REASON_ERROR.
            // SAFETY: para el evento 7, `data` apunta a un mpv_event_end_file.
            7 => Evento::FinArchivo(!e.data.is_null() && unsafe { *(e.data as *const c_int) } == 4),
            8 => Evento::ArchivoCargado,
            21 => Evento::Listo,
            2 => {
                // SAFETY: para el evento 2, `data` apunta a un mpv_event_log_message
                // con cadenas válidas hasta la próxima llamada.
                if !e.data.is_null() {
                    let m = unsafe { &*(e.data as *const MensajeC) };
                    let texto = |p: *const c_char| if p.is_null() { String::new() } else { unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned() };
                    let linea = format!("[{}] {}: {}", texto(m.prefix), texto(m.level), texto(m.text).trim_end());
                    if let Ok(mut v) = self.mensajes.lock() {
                        if v.len() >= 400 {
                            v.remove(0);
                        }
                        v.push(linea);
                    }
                }
                Evento::Mensaje
            }
            n => Evento::Otro(n),
        }
    }

    /// Pide a mpv los mensajes de su registro desde `nivel` ("error", "warn", "info"…).
    pub fn pedir_mensajes(&self, nivel: &str) -> Result<(), String> {
        let n = c(nivel)?;
        // SAFETY: handle válido y cadena terminada en 0.
        let e = unsafe { (self.lib.api.request_log_messages)(self.h, n.as_ptr()) };
        if e < 0 {
            return Err(format!("mensajes de mpv: {}", mensaje(&self.lib.api, e)));
        }
        Ok(())
    }

    /// Los mensajes que llegaron desde la última vez.
    pub fn tomar_mensajes(&self) -> Vec<String> {
        self.mensajes.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
    }

    /// Descarta los eventos pendientes.
    pub fn vaciar_eventos(&self) {
        while self.evento(0.0) != Evento::Nada {}
    }

    /// Espera hasta que el archivo cargado tenga su primer cuadro listo.
    pub fn esperar_listo(&self, limite: std::time::Duration) -> Result<(), String> {
        let fin = std::time::Instant::now() + limite;
        loop {
            let resta = fin.saturating_duration_since(std::time::Instant::now());
            if resta.is_zero() {
                return Err("el vídeo no quedó listo a tiempo".into());
            }
            match self.evento(resta.as_secs_f64()) {
                Evento::Listo => return Ok(()),
                Evento::FinArchivo(true) => {
                    return Err(format!("mpv no pudo abrir el archivo ({})", self.leer("path").unwrap_or_default()))
                }
                Evento::Apagado => return Err("mpv se cerró".into()),
                _ => {}
            }
        }
    }

    /// Carga `archivo` en pausa y espera su primer cuadro (precarga).
    pub fn precargar(&self, archivo: &str, limite: std::time::Duration) -> Result<(), String> {
        self.vaciar_eventos();
        self.propiedad("pause", "yes")?;
        self.comando(&["loadfile", archivo, "replace"])?;
        self.esperar_listo(limite)
    }
}

impl Drop for Reproductor {
    fn drop(&mut self) {
        // SAFETY: handle válido, no se usa después.
        unsafe { (self.lib.api.terminate_destroy)(self.h) };
    }
}
