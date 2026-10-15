//! Prueba de estrés: 10 minutos de órdenes al azar con bloques de tamaño
//! variable. Si aparece un solo chasquido, recorte o valor inválido, falla.

use std::sync::atomic::Ordering::Relaxed;

use otaconvert_audio::prueba::{Azar, Escenario};
use otaconvert_audio::{crear, Ajustes, Detector};

const FS: u32 = 48_000;

struct Resultado {
    ordenes: u64,
    detector: Detector,
    recortes: u64,
    rechazados: u64,
    basura_llena: u64,
}

fn correr(ajustes: Ajustes, segundos: u64, semilla: u64) -> Resultado {
    let (mut m, mut c) = crear(FS, 2, ajustes);
    let mut esc = Escenario::nuevo(semilla, FS);
    let mut det = Detector::nuevo(2, esc.umbral());
    let mut azar = Azar::nuevo(semilla ^ 0xABCD);
    let mut bloque = vec![0.0f32; 2 * 1024];

    let total = segundos * FS as u64;
    let mut hecho = 0u64;
    let mut proxima_orden = 0u64;
    while hecho < total {
        if hecho >= proxima_orden {
            proxima_orden = hecho + esc.paso(&mut c) as u64;
        }
        // Bloques de 1 a 1024 cuadros: la mezcla debe ser continua entre bloques.
        let n = (1 + azar.hasta(1024)).min(proxima_orden.saturating_sub(hecho).max(1)) as usize;
        let b = &mut bloque[..n * 2];
        m.procesar(b);
        det.analizar(b);
        hecho += n as u64;
    }
    c.recoger_basura();
    let e = &c.estadisticas;
    Resultado {
        ordenes: esc.ordenes,
        detector: det,
        recortes: e.recortes.load(Relaxed),
        rechazados: e.rechazados.load(Relaxed),
        basura_llena: e.basura_llena.load(Relaxed),
    }
}

#[test]
fn diez_minutos_sin_chasquidos() {
    for semilla in [1, 2, 3] {
        let r = correr(Ajustes::para(FS), 600, semilla);
        let d = &r.detector;
        println!(
            "semilla {semilla}: {} órdenes, {} cuadros, discontinuidad máxima {:.5} (umbral {:.5})",
            r.ordenes,
            d.cuadros(),
            d.maxima,
            Escenario::nuevo(semilla, FS).umbral()
        );
        assert!(r.ordenes >= 1000, "pocas órdenes: {}", r.ordenes);
        assert_eq!(d.discontinuidades, 0, "chasquido en {:?}", d.primera);
        assert_eq!(d.recortes + r.recortes, 0, "recortes");
        assert_eq!(d.no_numeros, 0, "valores inválidos");
        assert_eq!(r.rechazados, 0, "órdenes rechazadas");
        assert_eq!(r.basura_llena, 0, "memoria liberada en el hilo de audio");
    }
}

/// Comprueba que el detector sirve: sin rampas debe encontrar muchos chasquidos.
#[test]
fn sin_rampas_el_detector_avisa() {
    let ajustes = Ajustes { rampa_inicio: 0, rampa_parada: 0, ..Ajustes::para(FS) };
    let r = correr(ajustes, 120, 1);
    println!("sin rampas: {} discontinuidades en {} órdenes", r.detector.discontinuidades, r.ordenes);
    assert!(r.detector.discontinuidades > 50);
}
