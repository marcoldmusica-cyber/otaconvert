use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

use rtrb::{Consumer, Producer, PushError, RingBuffer};

use crate::{Pista, Rampa};

/// Rampas cortas del mezclador, en cuadros. Todas las elige el usuario;
/// 0 = de golpe. Son mínimos: el fundido de cada cue, si es más largo, manda.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ajustes {
    /// Subida al iniciar un cue. Por defecto 0: arranca de golpe.
    pub rampa_inicio: u32,
    /// Subida al reanudar tras una pausa.
    pub rampa_reanudar: u32,
    /// Bajada al pausar.
    pub rampa_pausa: u32,
    /// Bajada al parar (también Parar todo).
    pub rampa_parada: u32,
    /// Bajada antes de saltar a otro punto y subida después.
    pub rampa_salto: u32,
    /// Duración mínima de un cambio de volumen (del cue o maestro).
    pub rampa_volumen: u32,
    /// Bajada sobre los últimos cuadros de un archivo que no está en bucle.
    pub rampa_fin: u32,
    pub max_voces: usize,
}

/// Cuadros que dura `ms` milisegundos a `frecuencia`.
pub fn ms(frecuencia: u32, ms: f32) -> u32 {
    (frecuencia as f32 * ms / 1000.0).round() as u32
}

impl Ajustes {
    /// Valores por defecto: inicio de golpe; 5 ms al reanudar, saltar y
    /// cambiar volumen; 10 ms al pausar, parar y al final del archivo.
    pub fn para(frecuencia: u32) -> Self {
        Self { rampa_inicio: 0, ..Self::suaves(frecuencia) }
    }

    /// Todas las rampas activas, también al iniciar.
    pub fn suaves(frecuencia: u32) -> Self {
        Self {
            rampa_inicio: ms(frecuencia, 5.0),
            rampa_reanudar: ms(frecuencia, 5.0),
            rampa_pausa: ms(frecuencia, 10.0),
            rampa_parada: ms(frecuencia, 10.0),
            rampa_salto: ms(frecuencia, 5.0),
            rampa_volumen: ms(frecuencia, 5.0),
            rampa_fin: ms(frecuencia, 10.0),
            max_voces: 64,
        }
    }

    /// Todo de golpe, sin ninguna rampa.
    pub fn sin_rampas(frecuencia: u32) -> Self {
        Self {
            rampa_inicio: 0,
            rampa_reanudar: 0,
            rampa_pausa: 0,
            rampa_parada: 0,
            rampa_salto: 0,
            rampa_volumen: 0,
            rampa_fin: 0,
            ..Self::suaves(frecuencia)
        }
    }
}

/// Órdenes del hilo de control al hilo de audio. Los tiempos van en cuadros.
#[derive(Debug)]
pub enum Comando {
    /// Cambia las rampas en vivo, sin cortar lo que suena. `max_voces` no cambia.
    CambiarAjustes(Ajustes),
    Iniciar { id: u32, pista: Arc<Pista>, volumen: f32, desde: usize, bucle: bool, fundido: u32 },
    Parar { id: u32, fundido: u32 },
    Pausar { id: u32 },
    Reanudar { id: u32 },
    Volumen { id: u32, volumen: f32, cuadros: u32 },
    Saltar { id: u32, cuadro: usize },
    PararTodo { fundido: u32 },
    VolumenMaestro { volumen: f32, cuadros: u32 },
}

/// Contadores que el hilo de audio publica sin bloquear.
#[derive(Debug, Default)]
pub struct Estadisticas {
    pub bloques: AtomicU64,
    pub cuadros: AtomicU64,
    pub voces_activas: AtomicU32,
    /// Muestras de salida que habrían pasado de 0 dBFS (se recortan a ±1).
    pub recortes: AtomicU64,
    pub no_numeros: AtomicU64,
    /// Comandos descartados: sin voces libres o frecuencia distinta.
    pub rechazados: AtomicU64,
    /// Veces que se tuvo que liberar memoria en el hilo de audio (no debería pasar).
    pub basura_llena: AtomicU64,
}

/// Lado del hilo de control: envía comandos y libera los audios que ya no suenan.
pub struct Control {
    comandos: Producer<Comando>,
    basura: Consumer<Arc<Pista>>,
    pub estadisticas: Arc<Estadisticas>,
}

impl Control {
    /// Devuelve el comando si la cola está llena.
    pub fn enviar(&mut self, comando: Comando) -> Result<(), Comando> {
        self.recoger_basura();
        self.comandos.push(comando).map_err(|PushError::Full(c)| c)
    }

    /// Libera aquí los audios que el hilo de audio ya no usa.
    pub fn recoger_basura(&mut self) -> usize {
        let mut n = 0;
        while self.basura.pop().is_ok() {
            n += 1;
        }
        n
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Despues {
    Pausar,
    Parar,
    Reanudar,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Estado {
    Sonando,
    Bajando(Despues),
    Pausada,
}

struct Voz {
    id: u32,
    pista: Arc<Pista>,
    pos: usize,
    bucle: bool,
    volumen: Rampa,
    envolvente: Rampa,
    estado: Estado,
    salto: Option<usize>,
}

/// Lado del hilo de audio.
pub struct Mezclador {
    frecuencia: u32,
    canales: usize,
    ajustes: Ajustes,
    voces: Vec<Voz>,
    maestro: Rampa,
    comandos: Consumer<Comando>,
    basura: Producer<Arc<Pista>>,
    estadisticas: Arc<Estadisticas>,
}

/// Crea el par mezclador (hilo de audio) y control (hilo de la ventana).
pub fn crear(frecuencia: u32, canales: u16, ajustes: Ajustes) -> (Mezclador, Control) {
    assert!(canales > 0);
    let (cp, cc) = RingBuffer::new(1024);
    // Caben todas las voces más todos los comandos rechazados posibles.
    let (bp, bc) = RingBuffer::new(ajustes.max_voces + 1024);
    let estadisticas = Arc::new(Estadisticas::default());
    let mezclador = Mezclador {
        frecuencia,
        canales: canales as usize,
        ajustes,
        voces: Vec::with_capacity(ajustes.max_voces),
        maestro: Rampa::nueva(1.0),
        comandos: cc,
        basura: bp,
        estadisticas: estadisticas.clone(),
    };
    (mezclador, Control { comandos: cp, basura: bc, estadisticas })
}

impl Mezclador {
    pub fn frecuencia(&self) -> u32 {
        self.frecuencia
    }

    pub fn canales(&self) -> usize {
        self.canales
    }

    /// Llena `salida` (muestras intercaladas) con la mezcla. No pide memoria.
    pub fn procesar(&mut self, salida: &mut [f32]) {
        while let Ok(c) = self.comandos.pop() {
            self.aplicar(c);
        }

        salida.fill(0.0);
        let canales = self.canales;
        let fin = self.ajustes.rampa_fin;
        let salto = self.ajustes.rampa_salto;

        let mut i = 0;
        while i < self.voces.len() {
            let terminada = mezclar_voz(&mut self.voces[i], salida, canales, fin, salto);
            if terminada {
                let v = self.voces.swap_remove(i);
                self.tirar(v.pista);
            } else {
                i += 1;
            }
        }

        let mut recortes = 0;
        let mut no_numeros = 0;
        for cuadro in salida.chunks_exact_mut(canales) {
            let g = self.maestro.siguiente();
            for s in cuadro {
                let mut x = *s * g;
                if !x.is_finite() {
                    no_numeros += 1;
                    x = 0.0;
                } else if x.abs() > 1.0 {
                    recortes += 1;
                    x = x.clamp(-1.0, 1.0);
                }
                *s = x;
            }
        }

        let e = &self.estadisticas;
        e.bloques.fetch_add(1, Relaxed);
        e.cuadros.fetch_add((salida.len() / canales) as u64, Relaxed);
        e.voces_activas.store(self.voces.len() as u32, Relaxed);
        if recortes > 0 {
            e.recortes.fetch_add(recortes, Relaxed);
        }
        if no_numeros > 0 {
            e.no_numeros.fetch_add(no_numeros, Relaxed);
        }
    }

    fn tirar(&mut self, pista: Arc<Pista>) {
        if let Err(PushError::Full(p)) = self.basura.push(pista) {
            self.estadisticas.basura_llena.fetch_add(1, Relaxed);
            drop(p);
        }
    }

    fn rechazar(&mut self, pista: Arc<Pista>) {
        self.estadisticas.rechazados.fetch_add(1, Relaxed);
        self.tirar(pista);
    }

    fn voz(&mut self, id: u32) -> Option<&mut Voz> {
        self.voces.iter_mut().find(|v| v.id == id)
    }

    fn aplicar(&mut self, c: Comando) {
        let a = self.ajustes;
        match c {
            Comando::CambiarAjustes(nuevos) => {
                self.ajustes = Ajustes { max_voces: a.max_voces, ..nuevos };
            }
            Comando::Iniciar { id, pista, volumen, desde, bucle, fundido } => {
                if pista.frecuencia() != self.frecuencia
                    || self.voces.len() >= self.ajustes.max_voces
                    || desde >= pista.cuadros()
                {
                    self.rechazar(pista);
                    return;
                }
                let mut envolvente = Rampa::nueva(0.0);
                envolvente.ir_a(1.0, fundido.max(a.rampa_inicio));
                self.voces.push(Voz {
                    id,
                    pista,
                    pos: desde,
                    bucle,
                    volumen: Rampa::nueva(volumen),
                    envolvente,
                    estado: Estado::Sonando,
                    salto: None,
                });
            }
            Comando::Parar { id, fundido } => {
                if let Some(v) = self.voz(id) {
                    parar(v, fundido.max(a.rampa_parada));
                }
            }
            Comando::Pausar { id } => {
                if let Some(v) = self.voz(id) {
                    match v.estado {
                        Estado::Sonando | Estado::Bajando(Despues::Reanudar) => {
                            v.estado = Estado::Bajando(Despues::Pausar);
                            v.envolvente.ir_a(0.0, a.rampa_pausa);
                        }
                        Estado::Bajando(_) | Estado::Pausada => {}
                    }
                }
            }
            Comando::Reanudar { id } => {
                if let Some(v) = self.voz(id) {
                    match v.estado {
                        Estado::Pausada | Estado::Bajando(Despues::Pausar) => {
                            v.estado = Estado::Sonando;
                            v.envolvente.ir_a(1.0, a.rampa_reanudar);
                        }
                        Estado::Sonando | Estado::Bajando(_) => {}
                    }
                }
            }
            Comando::Volumen { id, volumen, cuadros } => {
                if let Some(v) = self.voz(id) {
                    v.volumen.ir_a(volumen, cuadros.max(a.rampa_volumen));
                }
            }
            Comando::Saltar { id, cuadro } => {
                if let Some(v) = self.voz(id) {
                    if cuadro >= v.pista.cuadros() {
                        return;
                    }
                    match v.estado {
                        Estado::Pausada => v.pos = cuadro,
                        Estado::Sonando => {
                            v.salto = Some(cuadro);
                            v.estado = Estado::Bajando(Despues::Reanudar);
                            v.envolvente.ir_a(0.0, a.rampa_salto);
                        }
                        Estado::Bajando(_) => v.salto = Some(cuadro),
                    }
                }
            }
            Comando::PararTodo { fundido } => {
                for v in &mut self.voces {
                    parar(v, fundido.max(a.rampa_parada));
                }
            }
            Comando::VolumenMaestro { volumen, cuadros } => {
                self.maestro.ir_a(volumen, cuadros.max(a.rampa_volumen));
            }
        }
    }
}

fn parar(v: &mut Voz, cuadros: u32) {
    if v.estado == Estado::Pausada {
        // Ya está en silencio: se quita en el próximo bloque.
        v.estado = Estado::Bajando(Despues::Parar);
        v.envolvente.ir_a(0.0, 0);
        return;
    }
    v.estado = Estado::Bajando(Despues::Parar);
    v.envolvente.ir_a(0.0, cuadros);
}

/// Suma una voz a la salida. Devuelve `true` si terminó.
#[inline]
fn mezclar_voz(v: &mut Voz, salida: &mut [f32], canales: usize, fin: u32, salto: u32) -> bool {
    let total = v.pista.cuadros();
    for cuadro in salida.chunks_exact_mut(canales) {
        match v.estado {
            Estado::Pausada => return false,
            Estado::Bajando(Despues::Parar) if v.envolvente.en_reposo() && v.envolvente.valor() == 0.0 => {
                return true;
            }
            _ => {}
        }
        if v.pos >= total {
            if v.bucle {
                v.pos = 0;
            } else {
                return true;
            }
        }

        let mut g = v.envolvente.siguiente() * v.volumen.siguiente();
        if !v.bucle && fin > 0 {
            // Rampa de salida sobre los últimos cuadros del archivo.
            let quedan = (total - v.pos) as u32;
            if quedan < fin {
                g *= quedan as f32 / fin as f32;
            }
        }
        let (l, r) = v.pista.cuadro(v.pos);
        if canales == 1 {
            cuadro[0] += (l + r) * 0.5 * g;
        } else {
            cuadro[0] += l * g;
            cuadro[1] += r * g;
        }
        v.pos += 1;

        if let Estado::Bajando(despues) = v.estado {
            if v.envolvente.en_reposo() && v.envolvente.valor() == 0.0 {
                if let Some(c) = v.salto.take() {
                    v.pos = c;
                }
                match despues {
                    Despues::Pausar => {
                        v.estado = Estado::Pausada;
                        return false;
                    }
                    Despues::Parar => return true,
                    Despues::Reanudar => {
                        v.estado = Estado::Sonando;
                        v.envolvente.ir_a(1.0, salto);
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const FS: u32 = 48_000;

    fn tono() -> Arc<Pista> {
        Arc::new(Pista::tono(440.0, 0.5, FS as usize, 2, FS))
    }

    fn render(m: &mut Mezclador, cuadros: usize) -> Vec<f32> {
        let mut v = vec![0.0; cuadros * m.canales()];
        m.procesar(&mut v);
        v
    }

    #[test]
    fn parar_todo_deja_silencio_y_devuelve_memoria() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::para(FS));
        let p = tono();
        for id in 0..3 {
            c.enviar(Comando::Iniciar { id, pista: p.clone(), volumen: 1.0, desde: 0, bucle: true, fundido: 0 })
                .unwrap();
        }
        render(&mut m, 4800);
        c.enviar(Comando::PararTodo { fundido: 0 }).unwrap();
        render(&mut m, 480); // 10 ms de bajada
        let s = render(&mut m, 480);
        assert!(s.iter().all(|&x| x == 0.0));
        assert_eq!(c.estadisticas.voces_activas.load(Relaxed), 0);
        assert_eq!(c.recoger_basura(), 3);
        assert_eq!(Arc::strong_count(&p), 1);
    }

    #[test]
    fn pausa_y_reanuda_en_el_mismo_punto() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::para(FS));
        c.enviar(Comando::Iniciar { id: 1, pista: tono(), volumen: 1.0, desde: 0, bucle: false, fundido: 0 })
            .unwrap();
        render(&mut m, 1000);
        c.enviar(Comando::Pausar { id: 1 }).unwrap();
        render(&mut m, 480);
        assert!(render(&mut m, 4800).iter().all(|&x| x == 0.0));
        assert_eq!(m.voces[0].pos, 1000 + 480);
        c.enviar(Comando::Reanudar { id: 1 }).unwrap();
        assert!(render(&mut m, 480).iter().any(|&x| x != 0.0));
    }

    #[test]
    fn rechaza_otra_frecuencia() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::para(FS));
        let p = Arc::new(Pista::tono(440.0, 0.5, 44_100, 2, 44_100));
        c.enviar(Comando::Iniciar { id: 1, pista: p, volumen: 1.0, desde: 0, bucle: false, fundido: 0 })
            .unwrap();
        render(&mut m, 480);
        assert_eq!(c.estadisticas.rechazados.load(Relaxed), 1);
        assert_eq!(c.estadisticas.voces_activas.load(Relaxed), 0);
    }

    fn continua() -> Arc<Pista> {
        Arc::new(Pista::nueva(vec![0.5; 48_000], 1, FS))
    }

    fn iniciar(c: &mut Control, p: Arc<Pista>) {
        c.enviar(Comando::Iniciar { id: 1, pista: p, volumen: 1.0, desde: 0, bucle: false, fundido: 0 })
            .unwrap();
    }

    #[test]
    fn por_defecto_arranca_de_golpe() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::para(FS));
        iniciar(&mut c, continua());
        assert_eq!(render(&mut m, 1)[0], 0.5);
    }

    #[test]
    fn con_rampa_de_inicio_sube_suave() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::suaves(FS));
        iniciar(&mut c, continua());
        let s = render(&mut m, 240);
        assert_eq!(s[0], 0.0);
        assert!(s[2 * 239] < 0.5);
        assert_eq!(render(&mut m, 1)[0], 0.5);
    }

    #[test]
    fn cambiar_ajustes_en_vivo() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::para(FS));
        c.enviar(Comando::CambiarAjustes(Ajustes::suaves(FS))).unwrap();
        iniciar(&mut c, continua());
        assert_eq!(render(&mut m, 1)[0], 0.0);
    }

    #[test]
    fn sin_rampas_para_de_golpe() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::sin_rampas(FS));
        iniciar(&mut c, continua());
        render(&mut m, 100);
        c.enviar(Comando::Parar { id: 1, fundido: 0 }).unwrap();
        assert!(render(&mut m, 1).iter().all(|&x| x == 0.0));
    }

    #[test]
    fn el_fundido_del_cue_manda_sobre_la_rampa() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::para(FS));
        c.enviar(Comando::Iniciar { id: 1, pista: continua(), volumen: 1.0, desde: 0, bucle: false, fundido: 4800 })
            .unwrap();
        let s = render(&mut m, 2400);
        assert_eq!(s[0], 0.0);
        assert!((s[2 * 2399] - 0.25).abs() < 0.01);
    }

    #[test]
    fn termina_sola_al_final_del_archivo() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::para(FS));
        let p = Arc::new(Pista::tono(440.0, 0.5, 1000, 1, FS));
        c.enviar(Comando::Iniciar { id: 1, pista: p, volumen: 1.0, desde: 0, bucle: false, fundido: 0 })
            .unwrap();
        render(&mut m, 2000);
        assert_eq!(c.estadisticas.voces_activas.load(Relaxed), 0);
    }
}
