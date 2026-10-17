//! El sonido real de la sesión: un mezclador por salida (Principal y,
//! si se elige, Auriculares) sobre WASAPI con cpal. Los streams quedan en el
//! hilo principal; la sesión solo tiene los `Control` de los mezcladores.

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, StreamTrait};
use otaconvert_audio::{crear, Ajustes, Comando, Control};
use otaconvert_servidor::{Destino, Sonido};

pub struct SalidaAbierta {
    pub stream: cpal::Stream,
    pub control: Control,
    pub frecuencia: u32,
    pub nombre: String,
    pub errores: Arc<AtomicU64>,
    /// Veces que la tarjeta pidió sonido más tarde que el margen: el búfer
    /// pudo vaciarse y oírse un corte (lo vigila el programa y lo anota).
    pub tardias: Arc<AtomicU64>,
    /// Mayor tiempo entre dos pedidos de la tarjeta, en microsegundos.
    pub intervalo_max_us: Arc<AtomicU64>,
}

/// Abre una salida (número, nombre o id; sin pedir nada, la predeterminada).
/// Si la pedida no está, es un error: nunca se cambia en silencio a otra.
pub fn abrir_salida(pedida: Option<&str>, margen_ms: u64) -> Result<SalidaAbierta, String> {
    let device = crate::elegir_salida(pedida)?;
    let nombre = device.to_string();
    let soportada = device.default_output_config().map_err(|e| e.to_string())?;
    if soportada.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!("{nombre}: formato {:?} todavía no soportado", soportada.sample_format()));
    }
    let frecuencia = soportada.sample_rate();
    let canales = soportada.channels();
    let config = cpal::StreamConfig {
        channels: canales,
        sample_rate: frecuencia,
        buffer_size: cpal::BufferSize::Fixed((frecuencia as u64 * margen_ms / 1000) as u32),
    };
    let (mut mezclador, control) = crear(frecuencia, canales, Ajustes::para(frecuencia));
    let errores = Arc::new(AtomicU64::new(0));
    let e2 = errores.clone();
    let tardias = Arc::new(AtomicU64::new(0));
    let intervalo_max_us = Arc::new(AtomicU64::new(0));
    let (t2, i2) = (tardias.clone(), intervalo_max_us.clone());
    let margen = std::time::Duration::from_millis(margen_ms);
    let mut anterior: Option<std::time::Instant> = None;
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Solo mide el tiempo (sin pedir memoria): seguro en el hilo de audio.
                let ahora = std::time::Instant::now();
                if let Some(a) = anterior {
                    let d = ahora.duration_since(a);
                    i2.fetch_max(d.as_micros() as u64, Relaxed);
                    if d > margen {
                        t2.fetch_add(1, Relaxed);
                    }
                }
                anterior = Some(ahora);
                mezclador.procesar(data)
            },
            move |_e| {
                e2.fetch_add(1, Relaxed);
            },
            None,
        )
        .map_err(|e| format!("{nombre}: {e}"))?;
    stream.play().map_err(|e| format!("{nombre}: {e}"))?;
    Ok(SalidaAbierta { stream, control, frecuencia, nombre, errores, tardias, intervalo_max_us })
}

pub struct SonidoCpal {
    pub principal: Control,
    pub frecuencia: u32,
    pub auriculares: Option<Control>,
}

impl Sonido for SonidoCpal {
    fn frecuencia(&self) -> u32 {
        self.frecuencia
    }
    fn tiene_auriculares(&self) -> bool {
        self.auriculares.is_some()
    }
    fn enviar(&mut self, destino: Destino, c: Comando) -> Result<(), String> {
        let control = match destino {
            Destino::Principal => &mut self.principal,
            Destino::Auriculares => self.auriculares.as_mut().ok_or("no hay salida de auriculares")?,
        };
        control.enviar(c).map_err(|_| "la cola del mezclador está llena".to_string())
    }
    fn recoger_basura(&mut self) {
        self.principal.recoger_basura();
        if let Some(a) = self.auriculares.as_mut() {
            a.recoger_basura();
        }
    }
}
