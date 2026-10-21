use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

use rtrb::{Consumer, Producer, PushError, RingBuffer};

use crate::limitador::Limitador;
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
    /// Limitador maestro transparente (activo por defecto).
    pub limitador: bool,
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
            limitador: true,
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
    /// Un cue completo: empieza en `desde` (≥ `inicio`), termina en `hasta` (o al
    /// final del archivo) bajando en `fundido_salida`; el bucle vuelve a `inicio`.
    IniciarCue {
        id: u32,
        pista: Arc<Pista>,
        volumen: f32,
        inicio: usize,
        desde: usize,
        hasta: Option<usize>,
        bucle: bool,
        fundido_entrada: u32,
        fundido_salida: u32,
    },
    /// Pausa general (P): todo lo que suena.
    PausarTodo,
    ReanudarTodo,
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
    /// Cuadros en los que el limitador tuvo que bajar el volumen.
    pub cuadros_limitados: AtomicU64,
    /// Medidor de nivel: el pico de lo que salió (canal izquierdo y derecho),
    /// desde la última lectura, como bits de un `f32` (0 a 1). Para números
    /// positivos el orden de los bits es el de los valores: sirve `fetch_max`.
    pub pico: [AtomicU32; 2],
}

impl Estadisticas {
    /// El pico de cada canal desde la última vez (y vuelve a empezar).
    pub fn tomar_picos(&self) -> [f32; 2] {
        [f32::from_bits(self.pico[0].swap(0, Relaxed)), f32::from_bits(self.pico[1].swap(0, Relaxed))]
    }
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
    /// Inicio del cue (a donde vuelve el bucle) y fin (exclusivo).
    inicio: usize,
    fin: usize,
    fundido_salida: u32,
}

/// Lado del hilo de audio.
pub struct Mezclador {
    frecuencia: u32,
    canales: usize,
    ajustes: Ajustes,
    voces: Vec<Voz>,
    maestro: Rampa,
    limitador: Limitador,
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
        limitador: Limitador::nuevo(frecuencia, canales as usize),
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

        let mut no_numeros = 0;
        for cuadro in salida.chunks_exact_mut(canales) {
            let g = self.maestro.siguiente();
            for s in cuadro {
                let x = *s * g;
                if x.is_finite() {
                    *s = x;
                } else {
                    no_numeros += 1;
                    *s = 0.0;
                }
            }
        }
        if self.ajustes.limitador {
            let antes = self.limitador.cuadros_limitados;
            self.limitador.procesar(salida);
            let limitados = self.limitador.cuadros_limitados - antes;
            if limitados > 0 {
                self.estadisticas.cuadros_limitados.fetch_add(limitados, Relaxed);
            }
        }
        // Último seguro: nada sale por encima de 0 dBFS (con el limitador no debería pasar).
        // En la misma pasada, el pico de cada lado para el medidor (sin pedir memoria).
        let mut recortes = 0;
        let mut pico = [0f32; 2];
        for (k, s) in salida.iter_mut().enumerate() {
            if s.abs() > 1.0 {
                recortes += 1;
                *s = s.clamp(-1.0, 1.0);
            }
            let lado = (k % canales).min(1);
            pico[lado] = pico[lado].max(s.abs());
        }
        if canales == 1 {
            pico[1] = pico[0];
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
        for (k, p) in pico.iter().enumerate() {
            // Un NaN ya se contó arriba; aquí solo valores de 0 a 1.
            if p.is_finite() {
                e.pico[k].fetch_max(p.to_bits(), Relaxed);
            }
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

    /// Arranca una voz. Rechaza (y devuelve la memoria) si la frecuencia no es
    /// la de la salida, no quedan voces o los puntos no tienen sentido.
    #[allow(clippy::too_many_arguments)]
    fn iniciar(
        &mut self,
        id: u32,
        pista: Arc<Pista>,
        volumen: f32,
        inicio: usize,
        desde: usize,
        hasta: Option<usize>,
        bucle: bool,
        fundido_entrada: u32,
        fundido_salida: u32,
    ) {
        let fin = hasta.unwrap_or(usize::MAX).min(pista.cuadros());
        if pista.frecuencia() != self.frecuencia
            || self.voces.len() >= self.ajustes.max_voces
            || inicio >= fin
            || desde < inicio
            || desde >= fin
        {
            self.rechazar(pista);
            return;
        }
        let mut envolvente = Rampa::nueva(0.0);
        envolvente.ir_a(1.0, fundido_entrada.max(self.ajustes.rampa_inicio));
        self.voces.push(Voz {
            id,
            pista,
            pos: desde,
            bucle,
            volumen: Rampa::nueva(volumen),
            envolvente,
            estado: Estado::Sonando,
            salto: None,
            inicio,
            fin,
            fundido_salida,
        });
    }

    fn aplicar(&mut self, c: Comando) {
        let a = self.ajustes;
        match c {
            Comando::CambiarAjustes(nuevos) => {
                self.ajustes = Ajustes { max_voces: a.max_voces, ..nuevos };
            }
            Comando::Iniciar { id, pista, volumen, desde, bucle, fundido } => {
                self.iniciar(id, pista, volumen, 0, desde, None, bucle, fundido, 0);
            }
            Comando::IniciarCue { id, pista, volumen, inicio, desde, hasta, bucle, fundido_entrada, fundido_salida } => {
                self.iniciar(id, pista, volumen, inicio, desde, hasta, bucle, fundido_entrada, fundido_salida);
            }
            Comando::PausarTodo => {
                for v in &mut self.voces {
                    pausar_voz(v, &a);
                }
            }
            Comando::ReanudarTodo => {
                for v in &mut self.voces {
                    reanudar_voz(v, &a);
                }
            }
            Comando::Parar { id, fundido } => {
                if let Some(v) = self.voz(id) {
                    parar(v, fundido.max(a.rampa_parada));
                }
            }
            Comando::Pausar { id } => {
                if let Some(v) = self.voz(id) {
                    pausar_voz(v, &a);
                }
            }
            Comando::Reanudar { id } => {
                if let Some(v) = self.voz(id) {
                    reanudar_voz(v, &a);
                }
            }
            Comando::Volumen { id, volumen, cuadros } => {
                if let Some(v) = self.voz(id) {
                    v.volumen.ir_a(volumen, cuadros.max(a.rampa_volumen));
                }
            }
            Comando::Saltar { id, cuadro } => {
                if let Some(v) = self.voz(id) {
                    if cuadro < v.inicio || cuadro >= v.fin {
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

fn pausar_voz(v: &mut Voz, a: &Ajustes) {
    match v.estado {
        Estado::Sonando | Estado::Bajando(Despues::Reanudar) => {
            v.estado = Estado::Bajando(Despues::Pausar);
            v.envolvente.ir_a(0.0, a.rampa_pausa);
        }
        Estado::Bajando(_) | Estado::Pausada => {}
    }
}

fn reanudar_voz(v: &mut Voz, a: &Ajustes) {
    match v.estado {
        Estado::Pausada | Estado::Bajando(Despues::Pausar) => {
            v.estado = Estado::Sonando;
            v.envolvente.ir_a(1.0, a.rampa_reanudar);
        }
        Estado::Sonando | Estado::Bajando(_) => {}
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
    let total = v.fin;
    // Bajada al final: la rampa corta o el fundido de salida del cue, la más larga.
    let fin = fin.max(v.fundido_salida);
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
                v.pos = v.inicio;
            } else {
                return true;
            }
        }

        let mut g = v.envolvente.siguiente() * v.volumen.siguiente();
        if !v.bucle && fin > 0 {
            // Rampa de salida sobre los últimos cuadros del cue.
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

    /// Sin limitador: estas pruebas miran la muestra exacta sin el retraso de 1,5 ms.
    fn sl(a: Ajustes) -> Ajustes {
        Ajustes { limitador: false, ..a }
    }

    fn tono() -> Arc<Pista> {
        Arc::new(Pista::tono(440.0, 0.5, FS as usize, 2, FS))
    }

    fn render(m: &mut Mezclador, cuadros: usize) -> Vec<f32> {
        let mut v = vec![0.0; cuadros * m.canales()];
        m.procesar(&mut v);
        v
    }

    /// Medidor de nivel: el pico de lo que sale, por lado, desde la última lectura.
    #[test]
    fn el_medidor_ve_el_pico_de_lo_que_sale() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::para(FS));
        assert_eq!(c.estadisticas.tomar_picos(), [0.0, 0.0], "en silencio, 0");
        c.enviar(Comando::Iniciar { id: 1, pista: tono(), volumen: 1.0, desde: 0, bucle: true, fundido: 0 }).unwrap();
        render(&mut m, 4800);
        let [i, d] = c.estadisticas.tomar_picos();
        assert!((i - 0.5).abs() < 0.01 && (d - 0.5).abs() < 0.01, "el tono tiene pico 0,5: {i} {d}");
        assert_eq!(c.estadisticas.tomar_picos(), [0.0, 0.0], "leer lo vuelve a 0");
        // Con el volumen a la mitad, el pico también.
        c.enviar(Comando::Volumen { id: 1, volumen: 0.5, cuadros: 0 }).unwrap();
        render(&mut m, 4800);
        render(&mut m, 4800);
        c.estadisticas.tomar_picos();
        render(&mut m, 4800);
        let [i, _] = c.estadisticas.tomar_picos();
        assert!((i - 0.25).abs() < 0.01, "{i}");
        // En mono, los dos lados muestran lo mismo.
        let (mut m1, mut c1) = crear(FS, 1, Ajustes::para(FS));
        c1.enviar(Comando::Iniciar { id: 1, pista: Arc::new(Pista::tono(440.0, 0.5, FS as usize, 1, FS)), volumen: 1.0, desde: 0, bucle: true, fundido: 0 }).unwrap();
        render(&mut m1, 4800);
        let [i, d] = c1.estadisticas.tomar_picos();
        assert!(i > 0.49 && i == d, "{i} {d}");
    }

    #[test]
    fn parar_todo_deja_silencio_y_devuelve_memoria() {
        let (mut m, mut c) = crear(FS, 2, sl(Ajustes::para(FS)));
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
        let (mut m, mut c) = crear(FS, 2, sl(Ajustes::para(FS)));
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
        let (mut m, mut c) = crear(FS, 2, sl(Ajustes::para(FS)));
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

    fn iniciar_id(c: &mut Control, id: u32, p: Arc<Pista>) {
        c.enviar(Comando::Iniciar { id, pista: p, volumen: 1.0, desde: 0, bucle: false, fundido: 0 }).unwrap();
    }

    fn iniciar(c: &mut Control, p: Arc<Pista>) {
        c.enviar(Comando::Iniciar { id: 1, pista: p, volumen: 1.0, desde: 0, bucle: false, fundido: 0 })
            .unwrap();
    }

    #[test]
    fn por_defecto_arranca_de_golpe() {
        let (mut m, mut c) = crear(FS, 2, sl(Ajustes::para(FS)));
        iniciar(&mut c, continua());
        assert_eq!(render(&mut m, 1)[0], 0.5);
    }

    #[test]
    fn con_rampa_de_inicio_sube_suave() {
        let (mut m, mut c) = crear(FS, 2, sl(Ajustes::suaves(FS)));
        iniciar(&mut c, continua());
        let s = render(&mut m, 240);
        assert_eq!(s[0], 0.0);
        assert!(s[2 * 239] < 0.5);
        assert_eq!(render(&mut m, 1)[0], 0.5);
    }

    #[test]
    fn cambiar_ajustes_en_vivo() {
        let (mut m, mut c) = crear(FS, 2, sl(Ajustes::para(FS)));
        c.enviar(Comando::CambiarAjustes(sl(Ajustes::suaves(FS)))).unwrap();
        iniciar(&mut c, continua());
        assert_eq!(render(&mut m, 1)[0], 0.0);
    }

    #[test]
    fn sin_rampas_para_de_golpe() {
        let (mut m, mut c) = crear(FS, 2, sl(Ajustes::sin_rampas(FS)));
        iniciar(&mut c, continua());
        render(&mut m, 100);
        c.enviar(Comando::Parar { id: 1, fundido: 0 }).unwrap();
        assert!(render(&mut m, 1).iter().all(|&x| x == 0.0));
    }

    #[test]
    fn el_fundido_del_cue_manda_sobre_la_rampa() {
        let (mut m, mut c) = crear(FS, 2, sl(Ajustes::para(FS)));
        c.enviar(Comando::Iniciar { id: 1, pista: continua(), volumen: 1.0, desde: 0, bucle: false, fundido: 4800 })
            .unwrap();
        let s = render(&mut m, 2400);
        assert_eq!(s[0], 0.0);
        assert!((s[2 * 2399] - 0.25).abs() < 0.01);
    }

    fn rampa_creciente(cuadros: usize) -> Arc<Pista> {
        Arc::new(Pista::nueva((0..cuadros).map(|i| i as f32 / cuadros as f32).collect(), 1, FS))
    }

    #[test]
    fn el_limitador_del_mezclador_no_deja_pasar_del_techo() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::para(FS));
        for id in 0..4 {
            iniciar_id(&mut c, id, continua());
        }
        let s = render(&mut m, 4800);
        let maximo = s.iter().fold(0.0f32, |p, x| p.max(x.abs()));
        assert!(maximo <= crate::limitador::TECHO + 1e-6, "máximo {maximo}");
        assert_eq!(c.estadisticas.recortes.load(Relaxed), 0);
        assert!(c.estadisticas.cuadros_limitados.load(Relaxed) > 0);
    }

    #[test]
    fn fin_del_cue_con_fundido_de_salida() {
        let (mut m, mut c) = crear(FS, 1, sl(Ajustes::sin_rampas(FS)));
        let p = Arc::new(Pista::nueva(vec![0.5; 1000], 1, FS));
        c.enviar(Comando::IniciarCue { id: 1, pista: p, volumen: 1.0, inicio: 0, desde: 0, hasta: Some(800), bucle: false, fundido_entrada: 0, fundido_salida: 200 })
            .unwrap();
        let s = render(&mut m, 1000);
        assert_eq!(s[100], 0.5);
        assert!((s[700] - 0.25).abs() < 1e-3, "a mitad del fundido: {}", s[700]);
        assert!(s[799] < 0.01 && s[799] > 0.0);
        assert!(s[800..].iter().all(|&x| x == 0.0), "después del fin, silencio");
        assert_eq!(c.estadisticas.voces_activas.load(Relaxed), 0);
    }

    #[test]
    fn el_bucle_vuelve_al_inicio_del_cue() {
        let (mut m, mut c) = crear(FS, 1, sl(Ajustes::sin_rampas(FS)));
        let p = rampa_creciente(1000);
        c.enviar(Comando::IniciarCue { id: 1, pista: p.clone(), volumen: 1.0, inicio: 100, desde: 250, hasta: Some(300), bucle: true, fundido_entrada: 0, fundido_salida: 0 })
            .unwrap();
        let s = render(&mut m, 300);
        assert_eq!(s[0], p.muestras()[250]);
        assert_eq!(s[49], p.muestras()[299]);
        assert_eq!(s[50], p.muestras()[100], "vuelve a su inicio, no al principio del archivo");
        assert_eq!(s[250], p.muestras()[100]);
    }

    #[test]
    fn pausa_general_y_vuelta() {
        let (mut m, mut c) = crear(FS, 2, sl(Ajustes::para(FS)));
        iniciar_id(&mut c, 1, continua());
        iniciar_id(&mut c, 2, continua());
        render(&mut m, 100);
        c.enviar(Comando::PausarTodo).unwrap();
        render(&mut m, 480);
        assert!(render(&mut m, 480).iter().all(|&x| x == 0.0));
        c.enviar(Comando::ReanudarTodo).unwrap();
        let s = render(&mut m, 480);
        assert!((s[2 * 479] - 1.0).abs() < 1e-6, "las dos vuelven: {}", s[2 * 479]);
    }

    #[test]
    fn puntos_imposibles_se_rechazan() {
        let (mut m, mut c) = crear(FS, 2, Ajustes::para(FS));
        for (inicio, desde, hasta) in [(500, 100, None), (100, 900, Some(800)), (800, 800, Some(800)), (0, 2000, None)] {
            c.enviar(Comando::IniciarCue { id: 1, pista: rampa_creciente(1000), volumen: 1.0, inicio, desde, hasta, bucle: false, fundido_entrada: 0, fundido_salida: 0 })
                .unwrap();
        }
        render(&mut m, 10);
        assert_eq!(c.estadisticas.rechazados.load(Relaxed), 4);
        assert_eq!(c.estadisticas.voces_activas.load(Relaxed), 0);
        assert_eq!(c.recoger_basura(), 4, "la memoria vuelve");
    }

    #[test]
    fn termina_sola_al_final_del_archivo() {
        let (mut m, mut c) = crear(FS, 2, sl(Ajustes::para(FS)));
        let p = Arc::new(Pista::tono(440.0, 0.5, 1000, 1, FS));
        c.enviar(Comando::Iniciar { id: 1, pista: p, volumen: 1.0, desde: 0, bucle: false, fundido: 0 })
            .unwrap();
        render(&mut m, 2000);
        assert_eq!(c.estadisticas.voces_activas.load(Relaxed), 0);
    }
}
