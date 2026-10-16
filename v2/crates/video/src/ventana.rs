//! Ventana de proyección: negra, sin cursor, con una ventana hija por capa
//! (una por reproductor) y encima de todas un **telón** negro (sin vídeo).
//! Vive en su propio hilo con su bucle de mensajes; desde fuera solo se cambia
//! el orden de las capas.
//!
//! - **Pantalla completa** (proyector): sin bordes, siempre encima, en el
//!   rectángulo de la pantalla elegida.
//! - **Vista previa** (cuando hay una sola pantalla): una ventana normal y
//!   chica, que se puede mover y agrandar, para no tapar el programa.
//!
//! Nunca toma el foco del teclado (WS_EX_NOACTIVATE): Espacio y Esc siguen
//! yendo a la ventana del programa.

use std::sync::mpsc;
use std::thread::JoinHandle;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, GetDC, GetPixel, GetStockObject, ReleaseDC, BLACK_BRUSH, HBRUSH};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW, GetWindow,
    MoveWindow, PostMessageW, PostQuitMessage, RegisterClassExW, SetCursor, SetWindowPos, ShowWindow,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, GW_CHILD, GW_HWNDNEXT, HWND_BOTTOM, HWND_TOP, HWND_TOPMOST, MSG,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNOACTIVATE, WM_APP, WM_CLOSE, WM_DESTROY, WM_SETCURSOR,
    WM_SIZE, WNDCLASSEXW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
    WS_VISIBLE,
};

/// Rectángulo de la pantalla en píxeles reales.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub ancho: i32,
    pub alto: i32,
}

/// Cómo se muestra la proyección.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modo {
    /// Sin bordes y siempre encima, ocupando `rect` (el proyector).
    Completa,
    /// Ventana normal y chica (una sola pantalla): no tapa el programa.
    VistaPrevia,
}

/// Mensaje propio para cerrar: el botón de cerrar de la vista previa no cierra
/// la proyección (se cierra con el programa).
const CERRAR: u32 = WM_APP + 1;

pub struct Proyeccion {
    principal: isize,
    capas: Vec<isize>,
    telon: isize,
    pub rect: Rect,
    hilo: Option<JoinHandle<()>>,
}

fn ancho(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn procedimiento(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        // Sin cursor sobre la proyección.
        WM_SETCURSOR => {
            // SAFETY: cursor nulo = oculto.
            unsafe { SetCursor(std::ptr::null_mut()) };
            1
        }
        // Las capas y el telón ocupan siempre todo el interior.
        WM_SIZE => {
            // SAFETY: recorre las ventanas hijas de esta ventana.
            unsafe {
                let mut r: RECT = std::mem::zeroed();
                GetClientRect(h, &mut r);
                let mut hijo = GetWindow(h, GW_CHILD);
                while !hijo.is_null() {
                    MoveWindow(hijo, 0, 0, r.right, r.bottom, 1);
                    hijo = GetWindow(hijo, GW_HWNDNEXT);
                }
            }
            0
        }
        // El botón de cerrar de la vista previa no cierra la proyección.
        WM_CLOSE => 0,
        CERRAR => {
            // SAFETY: destruye la ventana en su propio hilo.
            unsafe { DestroyWindow(h) };
            0
        }
        WM_DESTROY => {
            // SAFETY: termina el bucle de mensajes de este hilo (pedirlo varias veces no cambia nada).
            unsafe { PostQuitMessage(0) };
            0
        }
        // SAFETY: comportamiento normal de Windows.
        _ => unsafe { DefWindowProcW(h, msg, w, l) },
    }
}

impl Proyeccion {
    /// Pantalla completa en `rect` con `n_capas` capas (como en la Fase 0).
    pub fn abrir(rect: Rect, n_capas: usize) -> Result<Self, String> {
        Self::abrir_con(rect, n_capas, Modo::Completa)
    }

    /// Abre la proyección. Empieza con el telón arriba: todo negro.
    pub fn abrir_con(rect: Rect, n_capas: usize, modo: Modo) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<Result<(isize, Vec<isize>, isize), String>>();
        let hilo = std::thread::Builder::new()
            .name("proyeccion".into())
            .spawn(move || {
                let r = crear(rect, n_capas, modo);
                let ok = r.is_ok();
                let _ = tx.send(r);
                if ok {
                    bucle();
                }
            })
            .map_err(|e| e.to_string())?;
        let (principal, capas, telon) = rx.recv().map_err(|_| "el hilo de la proyección terminó".to_string())??;
        Ok(Self { principal, capas, telon, rect, hilo: Some(hilo) })
    }

    /// HWND de la capa `i` como entero, para `--wid` de mpv.
    pub fn capa(&self, i: usize) -> isize {
        self.capas[i]
    }

    pub fn n_capas(&self) -> usize {
        self.capas.len()
    }

    fn poner(&self, h: isize, donde: HWND) {
        // SAFETY: HWND válido mientras exista la proyección.
        unsafe { SetWindowPos(h as HWND, donde, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) };
    }

    /// Pone la capa `i` encima de todo (también del telón), sin activar la ventana.
    pub fn arriba(&self, i: usize) {
        self.poner(self.capas[i], HWND_TOP);
    }

    /// Manda la capa `i` al fondo (para precargar sin que se vea).
    pub fn abajo(&self, i: usize) {
        self.poner(self.capas[i], HWND_BOTTOM);
    }

    /// Pone el telón negro encima de todo: no se ve ninguna capa.
    pub fn telon_arriba(&self) {
        self.poner(self.telon, HWND_TOP);
    }

    /// Color del píxel central de la proyección, tal como se ve en la pantalla (0x00BBGGRR).
    pub fn color_centro(&self) -> u32 {
        // SAFETY: lee el interior de la ventana y un píxel de la pantalla; DC liberado enseguida.
        unsafe {
            let mut r: RECT = std::mem::zeroed();
            GetClientRect(self.principal as HWND, &mut r);
            let mut p = windows_sys::Win32::Foundation::POINT { x: r.right / 2, y: r.bottom / 2 };
            ClientToScreen(self.principal as HWND, &mut p);
            let dc = GetDC(std::ptr::null_mut());
            let c = GetPixel(dc, p.x, p.y);
            ReleaseDC(std::ptr::null_mut(), dc);
            c
        }
    }
}

impl Drop for Proyeccion {
    fn drop(&mut self) {
        // SAFETY: pide cerrar; el hilo de la ventana la destruye y sale del bucle.
        unsafe { PostMessageW(self.principal as HWND, CERRAR, 0, 0) };
        if let Some(h) = self.hilo.take() {
            let _ = h.join();
        }
    }
}

fn crear(r: Rect, n_capas: usize, modo: Modo) -> Result<(isize, Vec<isize>, isize), String> {
    let clase = ancho("OtaconvertProyeccion");
    // SAFETY: estructuras completas; la clase vive en memoria estática del proceso.
    unsafe {
        let instancia = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(procedimiento),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instancia,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: GetStockObject(BLACK_BRUSH) as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: clase.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        // Si ya estaba registrada (segunda proyección), sigue igual.
        RegisterClassExW(&wc);
        let (titulo, estilo_ex, estilo) = match modo {
            Modo::Completa => (
                ancho("Otaconvert · Proyector"),
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                WS_POPUP | WS_CLIPCHILDREN,
            ),
            Modo::VistaPrevia => (
                ancho("Otaconvert · Proyector (vista previa)"),
                WS_EX_NOACTIVATE,
                WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_CLIPCHILDREN,
            ),
        };
        let principal = CreateWindowExW(
            estilo_ex,
            clase.as_ptr(),
            titulo.as_ptr(),
            estilo,
            r.x,
            r.y,
            r.ancho,
            r.alto,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instancia,
            std::ptr::null(),
        );
        if principal.is_null() {
            return Err("no se pudo crear la ventana de proyección".into());
        }
        let mut cliente: RECT = std::mem::zeroed();
        GetClientRect(principal, &mut cliente);
        let hija = |nombre: *const u16| {
            CreateWindowExW(
                0,
                clase.as_ptr(),
                nombre,
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                0,
                0,
                cliente.right,
                cliente.bottom,
                principal,
                std::ptr::null_mut(),
                instancia,
                std::ptr::null(),
            )
        };
        let mut capas = Vec::with_capacity(n_capas);
        for _ in 0..n_capas {
            let capa = hija(std::ptr::null());
            if capa.is_null() {
                return Err("no se pudo crear una capa".into());
            }
            capas.push(capa as isize);
        }
        // El telón: una ventana negra sin vídeo, creada al final (queda arriba).
        let telon = hija(std::ptr::null());
        if telon.is_null() {
            return Err("no se pudo crear el telón".into());
        }
        SetWindowPos(telon, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        ShowWindow(principal, SW_SHOWNOACTIVATE);
        if modo == Modo::Completa {
            SetWindowPos(principal, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
        Ok((principal as isize, capas, telon as isize))
    }
}

fn bucle() {
    // SAFETY: bucle de mensajes estándar del hilo que creó las ventanas.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
