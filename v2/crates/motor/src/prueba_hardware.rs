//! Prueba de hardware (Fase 0): un doble clic, unos minutos, un informe.
//!
//! Cada prueba se ejecuta como proceso hijo del propio programa: si una se
//! cae o se cuelga, las demás siguen y el informe lo dice. Lo que muestra la
//! pantalla se copia tal cual al informe de texto.

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Veredicto {
    Bien,
    Problema,
    Dato,
}

struct Paso {
    titulo: String,
    args: Vec<String>,
    /// Si es solo información (no aprueba ni suspende).
    dato: bool,
    limite: Duration,
}

impl Paso {
    fn nuevo(titulo: impl Into<String>, args: &[&str], segundos: u64) -> Self {
        Self {
            titulo: titulo.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            dato: false,
            limite: Duration::from_secs(segundos),
        }
    }

    fn dato(mut self) -> Self {
        self.dato = true;
        self
    }
}

struct Informe {
    archivo: Option<File>,
    pub ruta: Option<PathBuf>,
}

impl Informe {
    fn linea(&mut self, s: &str) {
        println!("{s}");
        if let Some(f) = &mut self.archivo {
            let _ = writeln!(f, "{s}");
        }
    }
}

/// Fecha y hora local `AAAA-MM-DD_HH-MM` para el nombre del informe.
fn fecha() -> String {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        // SAFETY: rellena una estructura de salida.
        let t = unsafe {
            let mut t = std::mem::zeroed();
            GetLocalTime(&mut t);
            t
        };
        format!("{:04}-{:02}-{:02}_{:02}-{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute)
    }
    #[cfg(not(windows))]
    {
        "fecha".into()
    }
}

/// Crea el archivo del informe junto al programa o, si ahí no se puede
/// escribir, en Documentos.
fn crear_informe(equipo: &str) -> Informe {
    let nombre: String = format!("Informe-hardware-{equipo}-{}.txt", fecha())
        .chars()
        .map(|c| if c.is_alphanumeric() || "-_.".contains(c) { c } else { '_' })
        .collect();
    let mut lugares: Vec<PathBuf> = Vec::new();
    if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        lugares.push(d);
    }
    if let Ok(perfil) = std::env::var("USERPROFILE") {
        lugares.push(Path::new(&perfil).join("Documents"));
    }
    for d in lugares {
        let ruta = d.join(&nombre);
        if let Ok(mut f) = File::create(&ruta) {
            // Marca UTF-8 para que el Bloc de notas muestre bien las tildes.
            let _ = f.write_all("\u{FEFF}".as_bytes());
            return Informe { archivo: Some(f), ruta: Some(ruta) };
        }
    }
    Informe { archivo: None, ruta: None }
}

/// Ejecuta un paso como proceso hijo, copiando su salida al informe.
fn correr(paso: &Paso, inf: &mut Informe) -> Veredicto {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(_) => return Veredicto::Problema,
    };
    let mut hijo = match Command::new(exe)
        .args(&paso.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(h) => h,
        Err(e) => {
            inf.linea(&format!("   No se pudo ejecutar: {e}"));
            return Veredicto::Problema;
        }
    };
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let mut lectores = Vec::new();
    for flujo in [hijo.stdout.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>), hijo.stderr.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>)].into_iter().flatten() {
        let tx = tx.clone();
        lectores.push(std::thread::spawn(move || {
            for l in BufReader::new(flujo).lines().map_while(Result::ok) {
                let _ = tx.send(l);
            }
        }));
    }
    drop(tx);
    let inicio = Instant::now();
    let codigo = loop {
        while let Ok(l) = rx.try_recv() {
            inf.linea(&format!("   {l}"));
        }
        match hijo.try_wait() {
            Ok(Some(estado)) => break estado.code(),
            Ok(None) => {}
            Err(_) => break None,
        }
        if inicio.elapsed() > paso.limite {
            let _ = hijo.kill();
            let _ = hijo.wait();
            inf.linea("   La prueba no terminó a tiempo y se cerró.");
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    for l in lectores {
        let _ = l.join();
    }
    while let Ok(l) = rx.try_recv() {
        inf.linea(&format!("   {l}"));
    }
    match (paso.dato, codigo) {
        (true, _) => Veredicto::Dato,
        (false, Some(0)) => Veredicto::Bien,
        _ => Veredicto::Problema,
    }
}

fn pausa_final(pausa: bool) {
    if pausa {
        println!();
        println!("Pulsa Enter para cerrar.");
        let mut s = String::new();
        let _ = std::io::stdin().read_line(&mut s);
    }
}

pub fn ejecutar(pausa: bool) -> Result<bool, String> {
    crate::vigilante::sin_ventanas_de_error();
    let equipo = crate::equipo::leer();
    let mut inf = crear_informe(&equipo.nombre);

    inf.linea("Otaconvert · Prueba de hardware");
    inf.linea(&format!("Versión {} · {}", crate::VERSION, fecha().replace('_', " ")));
    inf.linea("Desarrollado por @marcold.ok (Instagram): envíame este informe, sugerencias o errores.");
    inf.linea("");
    println!("Esta prueba revisa la tarjeta de sonido, las pantallas y el vídeo de este equipo.");
    println!("  - Tarda unos 7 minutos.");
    println!("  - Sonarán tonos suaves y la pantalla del proyector (o esta, si no hay otra) se llenará de colores.");
    println!("  - El ratón se moverá solo un momento sobre la proyección (para comprobar que no se ve).");
    println!("  - No toques nada hasta que termine. Al final se guarda un informe para enviar.");
    println!();
    for s in (1..=5).rev() {
        print!("\rEmpieza en {s}… ");
        let _ = std::io::stdout().flush();
        std::thread::sleep(Duration::from_secs(1));
    }
    println!("\r                 ");

    // Carpeta con los vídeos de prueba, junto al programa.
    let pruebas = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("pruebas")));
    let existe = |rel: &str| pruebas.as_ref().map(|p| p.join(rel)).filter(|p| p.exists());

    let mut pasos: Vec<Paso> = vec![
        Paso::nuevo("Datos del equipo", &["--info-equipo"], 30).dato(),
        Paso::nuevo("Componentes necesarios", &["--comprobar"], 60),
        Paso::nuevo("Salidas de audio", &["--listar-salidas"], 30).dato(),
    ];
    let n_salidas = crate::salidas().map(|l| l.len()).unwrap_or(0);
    for n in 1..=n_salidas {
        let s = n.to_string();
        pasos.push(Paso::nuevo(
            format!("Sonido por la salida {n} (10 s, margen 50 ms)"),
            &["--prueba-vivo", "10", "50", "--salida", &s],
            60,
        ));
    }
    if n_salidas > 0 {
        pasos.push(Paso::nuevo(
            "Sonido por la salida predeterminada con margen corto (10 s, 20 ms)",
            &["--prueba-vivo", "10", "20"],
            60,
        ));
    }
    pasos.push(Paso::nuevo("Pantallas", &["--listar-pantallas"], 30).dato());
    pasos.push(Paso::nuevo("Cambios de imagen sin negros (100 cambios de color)", &["--prueba-video", "100"], 180));
    let mut no_medido: Vec<String> = Vec::new();
    match existe("1080p") {
        Some(d) => {
            let d = d.to_string_lossy().into_owned();
            pasos.push(Paso::nuevo(
                "Cambios entre vídeos 1080p reales (50 cambios)",
                &["--prueba-video", "50", "--carpeta", &d],
                180,
            ));
        }
        None => no_medido.push("Cambios entre vídeos reales (faltan los vídeos de prueba)".into()),
    }
    for (rel, titulo) in [
        ("1080p/4868D0.mp4", "Vídeo 1080p a 60 cuadros, 30 s seguidos"),
        ("4k-h264.mp4", "Vídeo 4K H.264 a 60 cuadros, 30 s seguidos"),
        ("4k-hevc.mp4", "Vídeo 4K HEVC a 60 cuadros, 30 s seguidos"),
    ] {
        match existe(rel) {
            Some(p) => {
                let p = p.to_string_lossy().into_owned();
                pasos.push(Paso::nuevo(titulo, &["--prueba-reproduccion", "30", "--archivo", &p], 120));
            }
            None => no_medido.push(format!("{titulo} (falta el vídeo de prueba)")),
        }
    }
    // Mapeo (etapa 4C): que esta tarjeta gráfica pueda con las 4 esquinas, el
    // patrón de prueba y el encuadre (si no puede, la imagen sale sin deformar).
    pasos.push(Paso::nuevo("Mapeo: 4 esquinas, patrón de prueba y encuadre", &["--prueba-mapeo"], 120));
    if n_salidas > 0 {
        pasos.push(Paso::nuevo("Vigilante: el motor vuelve tras caídas y cuelgues (6 fallos)", &["--prueba-vigilante", "6"], 120));
    }
    // La función simulada: el show completo como en una función (proyector a
    // pantalla completa, vídeos con sonido, imagen, ambiente en bucle, GO,
    // pausa, fundidos), grabando lo que sale por la tarjeta para buscar clics.
    match (existe("funcion"), n_salidas > 0) {
        (Some(d), true) => {
            let d = d.to_string_lossy().into_owned();
            pasos.push(Paso::nuevo(
                "Función simulada de 2 minutos (imagen sin negros y sonido sin clics, grabando lo que sale)",
                &["--prueba-funcion", "120", "--carpeta", &d],
                300,
            ));
        }
        (None, _) => no_medido.push("Función simulada (faltan los vídeos de prueba con sonido)".into()),
        (_, false) => {}
    }

    let mut resumen: Vec<(Veredicto, String)> = Vec::new();
    for (i, paso) in pasos.iter().enumerate() {
        inf.linea("");
        inf.linea(&format!("[{}/{}] {}", i + 1, pasos.len(), paso.titulo));
        let v = correr(paso, &mut inf);
        resumen.push((v, paso.titulo.clone()));
    }
    if n_salidas == 0 {
        no_medido.push("Sonido: no hay ninguna salida de audio".into());
    }
    for t in [
        "Latencia real de la salida (hace falta grabar lo que sale por la tarjeta)",
        "Cambio entre 44,1 y 48 kHz y deriva del reloj",
        "Cortes dentro del propio driver de la tarjeta o en el cable (la grabación de la función simulada se toma antes del driver)",
    ] {
        no_medido.push(t.into());
    }

    inf.linea("");
    inf.linea("— Resumen —");
    for (v, t) in &resumen {
        let marca = match v {
            Veredicto::Bien => "BIEN     ",
            Veredicto::Problema => "PROBLEMA ",
            Veredicto::Dato => "DATO     ",
        };
        inf.linea(&format!("{marca} {t}"));
    }
    for t in &no_medido {
        inf.linea(&format!("NO MEDIDO {t}"));
    }
    let problemas = resumen.iter().filter(|(v, _)| *v == Veredicto::Problema).count();
    inf.linea("");
    if problemas == 0 {
        inf.linea("VEREDICTO: este equipo pasó todas las pruebas que se pudieron medir.");
    } else {
        inf.linea(&format!("VEREDICTO: {problemas} prueba(s) con problemas. Mira arriba qué falló."));
    }
    match &inf.ruta {
        Some(r) => {
            println!();
            println!("Informe guardado en:");
            println!("  {}", r.display());
            println!("Envíalo tal cual; no hace falta abrirlo.");
        }
        None => println!("No se pudo guardar el informe (no hay carpeta con permiso de escritura)."),
    }
    pausa_final(pausa);
    Ok(problemas == 0)
}
