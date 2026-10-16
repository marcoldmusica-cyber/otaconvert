//! Preparar un audio para un cue: leerlo entero, pasarlo a float 32 y a la
//! frecuencia de la tarjeta, y medir su pico. Se hace **fuera** del hilo de
//! audio, en segundo plano, cuando el cue queda en espera (CLAUDE.md:
//! "sin remuestreo en vivo").
//!
//! - Lectura: Symphonia 0.6.1 (MP3, MP1/2, AAC, ALAC, FLAC, Vorbis, ADPCM,
//!   PCM; contenedores WAV, AIFF, CAF, OGG, MKV/WebM, MP4/M4A).
//! - Cambio de frecuencia: rubato 5 (`Fft`, síncrono, alta calidad).
//! - Más de 2 canales: se pasa a estéreo (izq. = 1º + 0,707·3º; der. = 2º +
//!   0,707·3º, suponiendo el orden habitual FL, FR, FC…). Multicanal: Fase 3.

use std::fs::File;
use std::path::Path;

use rubato::audioadapter::Adapter;
use rubato::audioadapter_buffers::owned::InterleavedOwned;
use rubato::{Fft, FixedSync, Resampler};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as ErrorSym;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::Pista;

/// Audios más largos que esto no se cargan en memoria (unos 1,3 GB en estéreo a 48 kHz).
pub const DURACION_MAXIMA_S: u64 = 60 * 60;
const ERRORES_TOLERADOS: usize = 50;

#[derive(Debug)]
pub enum ErrorPreparar {
    NoSePudoAbrir(String),
    FormatoNoSoportado(String),
    SinAudio,
    Danado(String),
    DemasiadoLargo(u64),
}

impl std::fmt::Display for ErrorPreparar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSePudoAbrir(e) => write!(f, "no se pudo abrir el audio: {e}"),
            Self::FormatoNoSoportado(e) => write!(
                f,
                "formato de audio no soportado ({e}). Conviértelo a WAV, FLAC o MP3 en la pestaña Convertir"
            ),
            Self::SinAudio => write!(f, "el archivo no tiene sonido"),
            Self::Danado(e) => write!(f, "el audio está dañado: {e}"),
            Self::DemasiadoLargo(s) => write!(f, "el audio dura {} minutos; el máximo es {} minutos", s / 60, DURACION_MAXIMA_S / 60),
        }
    }
}

impl std::error::Error for ErrorPreparar {}

/// Un audio listo para sonar.
#[derive(Debug)]
pub struct Preparado {
    pub pista: Pista,
    /// Pico absoluto tras preparar (1,0 = 0 dBFS; la comprobación previa marca > 1,0).
    pub pico: f32,
    pub frecuencia_original: u32,
    pub canales_originales: usize,
    /// Paquetes dañados que se saltaron (0 = archivo sano).
    pub paquetes_danados: usize,
}

impl Preparado {
    pub fn duracion_s(&self) -> f64 {
        self.pista.cuadros() as f64 / self.pista.frecuencia() as f64
    }
}

/// Decodifica todo el archivo a float 32 intercalado. Devuelve (muestras, canales, frecuencia, dañados).
fn decodificar(ruta: &Path) -> Result<(Vec<f32>, usize, u32, usize), ErrorPreparar> {
    let archivo = File::open(ruta).map_err(|e| ErrorPreparar::NoSePudoAbrir(e.to_string()))?;
    let mss = MediaSourceStream::new(Box::new(archivo), Default::default());
    let mut pista_hint = Hint::new();
    if let Some(ext) = ruta.extension().and_then(|e| e.to_str()) {
        pista_hint.with_extension(ext);
    }
    let mut formato = symphonia::default::get_probe()
        .probe(&pista_hint, mss, FormatOptions::default(), MetadataOptions::default())
        .map_err(|e| match e {
            ErrorSym::Unsupported(m) => ErrorPreparar::FormatoNoSoportado(m.to_string()),
            ErrorSym::IoError(e) => ErrorPreparar::NoSePudoAbrir(e.to_string()),
            otro => ErrorPreparar::Danado(otro.to_string()),
        })?;
    let (id, params) = {
        let t = formato.default_track(TrackType::Audio).ok_or(ErrorPreparar::SinAudio)?;
        (t.id, t.codec_params.clone().ok_or(ErrorPreparar::SinAudio)?)
    };
    let audio = params.audio().ok_or(ErrorPreparar::SinAudio)?;
    let mut decodificador = symphonia::default::get_codecs()
        .make_audio_decoder(audio, &AudioDecoderOptions::default())
        .map_err(|e| ErrorPreparar::FormatoNoSoportado(e.to_string()))?;

    let mut muestras: Vec<f32> = Vec::new();
    let mut tmp: Vec<f32> = Vec::new();
    let (mut canales, mut frecuencia, mut danados) = (0usize, 0u32, 0usize);
    loop {
        let paquete = match formato.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(ErrorSym::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(ErrorSym::ResetRequired) => break,
            Err(e) => return Err(ErrorPreparar::Danado(e.to_string())),
        };
        if paquete.track_id != id {
            continue;
        }
        match decodificador.decode(&paquete) {
            Ok(buf) => {
                let spec = buf.spec();
                let (c, f) = (spec.channels().count(), spec.rate());
                if canales == 0 {
                    canales = c;
                    frecuencia = f;
                } else if c != canales || f != frecuencia {
                    return Err(ErrorPreparar::Danado("cambia de canales o de frecuencia a mitad del archivo".into()));
                }
                tmp.resize(buf.samples_interleaved(), 0.0);
                buf.copy_to_slice_interleaved(&mut tmp[..]);
                muestras.extend_from_slice(&tmp);
                if canales > 0 && frecuencia > 0 && (muestras.len() / canales) as u64 > DURACION_MAXIMA_S * frecuencia as u64 {
                    return Err(ErrorPreparar::DemasiadoLargo((muestras.len() / canales) as u64 / frecuencia as u64));
                }
            }
            Err(ErrorSym::DecodeError(_)) => {
                danados += 1;
                if danados > ERRORES_TOLERADOS {
                    return Err(ErrorPreparar::Danado(format!("más de {ERRORES_TOLERADOS} partes ilegibles")));
                }
            }
            Err(e) => return Err(ErrorPreparar::Danado(e.to_string())),
        }
    }
    if canales == 0 || frecuencia == 0 || muestras.is_empty() {
        return Err(ErrorPreparar::SinAudio);
    }
    Ok((muestras, canales, frecuencia, danados))
}

/// Más de dos canales → estéreo.
fn a_estereo(m: &[f32], canales: usize) -> Vec<f32> {
    let k = std::f32::consts::FRAC_1_SQRT_2;
    let mut out = Vec::with_capacity(m.len() / canales * 2);
    for c in m.chunks_exact(canales) {
        let centro = if canales >= 3 { k * c[2] } else { 0.0 };
        out.push(c[0] + centro);
        out.push(c[1] + centro);
    }
    out
}

/// Prepara `ruta` para sonar a `frecuencia_salida` (la de la tarjeta).
pub fn preparar(ruta: &Path, frecuencia_salida: u32) -> Result<Preparado, ErrorPreparar> {
    let (mut muestras, canales_originales, frecuencia_original, paquetes_danados) = decodificar(ruta)?;
    let mut canales = canales_originales;
    if canales > 2 {
        muestras = a_estereo(&muestras, canales);
        canales = 2;
    }
    if frecuencia_original != frecuencia_salida {
        let cuadros = muestras.len() / canales;
        let entrada = InterleavedOwned::new_from(muestras, canales, cuadros)
            .map_err(|e| ErrorPreparar::Danado(e.to_string()))?;
        let mut r = Fft::<f32>::new(frecuencia_original as usize, frecuencia_salida as usize, 1024, canales, FixedSync::Both)
            .map_err(|e| ErrorPreparar::FormatoNoSoportado(format!("no se puede convertir de {frecuencia_original} Hz: {e}")))?;
        let salida = r.process_all(&entrada, cuadros, None).map_err(|e| ErrorPreparar::Danado(e.to_string()))?;
        let n = salida.frames();
        muestras = salida.take_data();
        muestras.truncate(n * canales);
    }
    let pico = muestras.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    Ok(Preparado {
        pista: Pista::nueva(muestras, canales as u16, frecuencia_salida),
        pico,
        frecuencia_original,
        canales_originales,
        paquetes_danados,
    })
}
