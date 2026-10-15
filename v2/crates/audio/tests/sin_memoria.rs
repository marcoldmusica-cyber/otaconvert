//! El hilo de audio no debe pedir ni liberar memoria: hacerlo puede tardar
//! y provocar un corte. Un asignador que cuenta lo comprueba, solo en el
//! hilo que llama a `procesar`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use otaconvert_audio::prueba::Escenario;
use otaconvert_audio::{crear, Ajustes};

struct Contador;

thread_local! {
    static VIGILAR: Cell<bool> = const { Cell::new(false) };
}
static PEDIDOS: AtomicUsize = AtomicUsize::new(0);

fn anotar() {
    if VIGILAR.try_with(|v| v.get()).unwrap_or(false) {
        PEDIDOS.fetch_add(1, SeqCst);
    }
}

unsafe impl GlobalAlloc for Contador {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        anotar();
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        anotar();
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Contador = Contador;

#[test]
fn procesar_no_toca_la_memoria() {
    const FS: u32 = 48_000;
    let (mut m, mut c) = crear(FS, 2, Ajustes::para(FS));
    let mut esc = Escenario::nuevo(7, FS);
    let mut bloque = vec![0.0f32; 2 * 480];
    let mut espera = 0u32;
    for _ in 0..20_000 {
        if espera == 0 {
            espera = esc.paso(&mut c);
        }
        espera = espera.saturating_sub(480);
        VIGILAR.with(|v| v.set(true));
        m.procesar(&mut bloque);
        VIGILAR.with(|v| v.set(false));
        c.recoger_basura();
    }
    assert!(esc.ordenes > 100);
    assert_eq!(PEDIDOS.load(SeqCst), 0, "el hilo de audio pidió o liberó memoria");
}
