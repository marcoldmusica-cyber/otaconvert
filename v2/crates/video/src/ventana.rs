//! Ventana de proyección: sin bordes, negra, sin cursor, ocupando la pantalla
//! elegida, con una ventana hija por capa. Vive en su propio hilo con su bucle
//! de mensajes; desde fuera solo se cambia el orden de las capas.

use std::sync::mpsc;
use std::thread::JoinHandle;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{GetDC, GetPixel, GetStockObject, ReleaseDC, BLACK_BRUSH, HBRUSH};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostMessageW, PostQuitMessage,
    RegisterClassExW, SetCursor, SetWindowPos, TranslateMessage, CS_HREDRAW, CS_VREDRAW, HWND_TOP,
    HWND_TOPMOST, MSG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WM_CLOSE, WM_DESTROY, WM_SETCURSOR,
    WNDCLASSEXW, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
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

pub struct Proyeccion {
    principal: isize,
    capas: Vec<isize>,
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
        WM_DESTROY => {
            // SAFETY: termina el bucle de mensajes de este hilo.
            unsafe { PostQuitMessage(0) };
            0
        }
        // SAFETY: comportamiento normal de Windows.
        _ => unsafe { DefWindowProcW(h, msg, w, l) },
    }
}

impl Proyeccion {
    /// Abre la ventana en `rect` con `n_capas` capas (la última creada queda arriba).
    pub fn abrir(rect: Rect, n_capas: usize) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<Result<(isize, Vec<isize>), String>>();
        let hilo = std::thread::Builder::new()
            .name("proyeccion".into())
            .spawn(move || {
                let r = crear(rect, n_capas);
                let ok = r.is_ok();
                let _ = tx.send(r);
                if ok {
                    bucle();
                }
            })
            .map_err(|e| e.to_string())?;
        let (principal, capas) = rx.recv().map_err(|_| "el hilo de la proyección terminó".to_string())??;
        Ok(Self { principal, capas, rect, hilo: Some(hilo) })
    }

    /// HWND de la capa `i` como entero, para `--wid` de mpv.
    pub fn capa(&self, i: usize) -> isize {
        self.capas[i]
    }

    /// Pone la capa `i` encima de las demás, sin activar la ventana.
    pub fn arriba(&self, i: usize) {
        // SAFETY: HWND válido mientras exista la proyección.
        unsafe {
            SetWindowPos(
                self.capas[i] as HWND,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    /// Color del píxel central de la pantalla, tal como se ve (0x00BBGGRR).
    pub fn color_centro(&self) -> u32 {
        let (x, y) = (self.rect.x + self.rect.ancho / 2, self.rect.y + self.rect.alto / 2);
        // SAFETY: DC de toda la pantalla, liberado enseguida.
        unsafe {
            let dc = GetDC(std::ptr::null_mut());
            let c = GetPixel(dc, x, y);
            ReleaseDC(std::ptr::null_mut(), dc);
            c
        }
    }
}

impl Drop for Proyeccion {
    fn drop(&mut self) {
        // SAFETY: pide cerrar; el hilo de la ventana la destruye y sale del bucle.
        unsafe { PostMessageW(self.principal as HWND, WM_CLOSE, 0, 0) };
        if let Some(h) = self.hilo.take() {
            let _ = h.join();
        }
    }
}

fn crear(r: Rect, n_capas: usize) -> Result<(isize, Vec<isize>), String> {
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
        let titulo = ancho("Otaconvert · Proyector");
        let principal = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            clase.as_ptr(),
            titulo.as_ptr(),
            WS_POPUP | WS_VISIBLE | WS_CLIPCHILDREN,
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
        SetWindowPos(principal, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        let mut capas = Vec::with_capacity(n_capas);
        for _ in 0..n_capas {
            let capa = CreateWindowExW(
                0,
                clase.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                0,
                0,
                r.ancho,
                r.alto,
                principal,
                std::ptr::null_mut(),
                instancia,
                std::ptr::null(),
            );
            if capa.is_null() {
                return Err("no se pudo crear una capa".into());
            }
            capas.push(capa as isize);
        }
        Ok((principal as isize, capas))
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
