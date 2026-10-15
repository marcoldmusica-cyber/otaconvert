# Otaconvert: contexto del proyecto

Léelo entero antes de proponer o tocar nada. Si algo de aquí choca con el código, el código manda en lo que ya existe y este archivo manda en lo decidido para el futuro. En caso de duda, pregunta.

## 0. Cómo trabajar con el usuario

- **Siempre en español**, claro y sin jerga. El usuario es técnico de teatro, no programador ni experto en Claude.
- Pasos **cortos y numerados**, con el nombre exacto de cada botón (GitHub está en inglés: nombrar botones en inglés).
- Usa **Windows 10**. Los equipos de los teatros varían mucho.
- Los riesgos técnicos **no deben existir**: diseñar para eliminarlos y verificarlo con pruebas. Lo no eliminable (driver defectuoso, cable flojo) se **detecta y se avisa antes de la función**.
- Antes de afirmar cómo funciona una librería o programa de terceros, verificarlo (documentación o código).
- Ser honesto sobre lo **no** probado: distinguir lo comprobado de lo pendiente.
- Si pide respuestas rápidas, respetarlo.
- Cuando el usuario dice **"el manual"** o "el registro del proyecto" se refiere a **este archivo (CLAUDE.md)**. Toda decisión, función y opción nueva se anota aquí.
- **Regla de interfaz (15/10/2026):** cada opción configurable lleva un **mensaje emergente sutil** (al pasar el ratón o tocar "?") que recomienda en qué casos activarla o desactivarla y qué valor usar. Los textos viven en el código, en español e inglés, junto a la opción, y se resumen en este archivo.
- El usuario quiere que Claude maneje GitHub de forma automática (commits, subidas, compilaciones). Avisar antes de cada `git push` salvo que diga lo contrario.

### Entorno de la PC del usuario
- Git y GitHub CLI instalados con winget (15/10/2026). Rutas: `C:\Program Files\Git\cmd\git.exe`, `C:\Program Files\GitHub CLI\gh.exe`. Sesión iniciada en GitHub como `marcoldmusica-cyber` (alcances repo y workflow).
- Repositorio local: `C:\Users\Marco1\Desktop\CLOUDE - Herramientas Ota\otaconvert`. En la carpeta superior el usuario tiene builds descargadas de la V1 y un zip; no tocarlas.

---

## 1. Otaconvert V1 (terminada y funcionando)

Conversor de audio por lotes de escritorio: se cargan muchos archivos (arrastrar y soltar, carpetas recursivas o botón) y se convierten todos a un formato.

- Repositorio **github.com/marcoldmusica-cyber/otaconvert**, rama `main`. Compila con GitHub Actions (`.github/workflows/main.yml`, creado a mano por el usuario desde la web): matriz `windows-latest` + `macos-latest`, Python 3.12, `choco install innosetup`, `python build.py`, artifacts `Otaconvert-Windows` y `Otaconvert-macOS`; con etiqueta `v*` publica una Release. El usuario instaló `Otaconvert-Setup.exe` y confirmó que funciona.
- Avisos pendientes (no errores): deprecación de Node.js 20 en `actions/checkout@v4`, `actions/setup-python@v5`, `actions/upload-artifact@v4`. Actualizar cuando haya versiones con Node 24.

### Archivos
| Archivo | Función |
|---|---|
| `otaconvert.py` | Programa completo (Python + CustomTkinter + FFmpeg). |
| `requirements.txt` | `customtkinter>=5.2`, `tkinterdnd2>=0.4`, `imageio-ffmpeg>=0.5` |
| `otaconvert.spec` | PyInstaller modo carpeta (`COLLECT`), `console=False`, `collect_all` de customtkinter, tkinterdnd2 e imageio_ffmpeg (trae FFmpeg). Excluye numpy, matplotlib, scipy, pandas, IPython, pytest. En macOS `.app` con `BUNDLE`. Versión desde `OTA_VERSION`. |
| `build.py` | Limpia, ejecuta PyInstaller, corre la **autoprueba del ejecutable empaquetado** (si falla, aborta), crea la carpeta portable (+ `portable.txt`), la comprime y en Windows llama a Inno Setup. Salida `dist/release/`. |
| `installer.iss` | Inno Setup 6, por usuario sin administrador (`PrivilegesRequired=lowest`), español + inglés, acceso directo opcional, desinstalador. AppId `{7C1D3B5E-4A62-4E0B-9C57-0F7A2B6D8E41}`. Salida `Otaconvert-Setup.exe`. |
| `build_windows.bat` | Compilación local con venv `.buildenv`. |
| `assets/otaconvert.ico/.icns/.png` | Icono: cuadrado redondeado oscuro con 6 barras de onda en color acento. |
| `THIRD_PARTY.txt` | Licencias: FFmpeg (LGPL/GPL), CustomTkinter (MIT), tkinterdnd2 (MIT), Python (PSF), imageio-ffmpeg (BSD-2). |
| `.gitignore` | `dist/`, `build/`, `.buildenv/`, `__pycache__/`, `*.pyc` (no se había subido; añadido el 15/10/2026). |

### Funcionalidad
- Entrada: casi cualquier audio y el audio de vídeos (`AUDIO_EXT`: mp3, wav, flac, ogg, oga, opus, m4a, aac, wma, aiff, aif, aifc, ape, wv, mka, weba, webm, mp4, m4b, m4r, mov, mkv, avi, caf, amr, ac3, dts, 3gp, mpc, tta, au, snd, mp2, spx, gsm, ra).
- Salida (`FORMATS`): WAV 16 (def.)/24/32 float · MP3 128/192/256/320 (def. 320) · FLAC Automática (def.)/16/24 · OGG Media/Alta (def.)/Máxima · M4A 128/192/256 (def. 256) · OPUS 64/96/128/192 (def. 128, fuerza 48 kHz) · AIFF 16 (def.)/24.
- Frecuencia: Original, 44,1, 48, 96 kHz (MP3 máx. 48). Destino: junto al original (def.) o carpeta elegida.
- Paralelo con `ThreadPoolExecutor`, `min(8, max(2, núcleos−1))` hilos; los hilos envían eventos a una cola que la ventana lee cada 80 ms.
- Progreso por archivo (`-progress pipe:1`) y total; Cancelar termina procesos.
- Nunca sobrescribe (`unique_path` añade ` (1)`…); escribe a `.part` y renombra; al cancelar borra el temporal.
- Errores legibles sin frenar el lote. Recuerda formato, calidad, frecuencia y carpeta. Modo portable con `portable.txt` → `otaconvert-config.json` junto al exe; si no, `~/.otaconvert.json`.
- `Otaconvert.exe --selftest [informe.txt]`: tono convertido a los 7 formatos + comprobación de interfaz y arrastrar y soltar.
- Subprocesos con `stdin=DEVNULL` y `CREATE_NO_WINDOW`.
- Sin límite de archivos en código; con miles la lista (un widget `Row` por archivo) se vuelve lenta. Mejora posible: lista virtualizada (el usuario no respondió).

### Estética (base visual obligatoria de la V2)
```
BG #0E1117   CARD #151921   CARD2 #1B2029   HOVER #252B37   BORDER #262C38
TEXT #E9ECF2   MUTED #9098A8   DIM #5D6677
ACCENT #6F80FF   ACCENT_H #5C6DEE   OK #58D69A   ERR #F27A74
Fuente: Segoe UI (Windows), SF Pro Display (macOS), Helvetica (otros)
```
Tema oscuro minimalista, un solo acento. Tarjetas 18 px, botones 10–12 px. Marca de **6 barras de onda** (alturas 0.38, 0.72, 1.0, 0.58, 0.84, 0.46) animada mientras trabaja. Sin etiquetas en mayúsculas ni flechas decorativas. Estado vacío centrado con la marca grande, título y botón "Elegir archivos".

---

## 2. Otaconvert V2: reproductor de cues para teatro

Audio, vídeo e imágenes, al estilo de QLab o MultiPlay, para técnicos que disparan pistas y visuales en funciones. Requisitos:
- **Muy intuitivo**, con los botones justos. Estética de la V1.
- **Ayuda para principiantes:** apartados que explican cada función y cómo usar el programa **como para que lo entienda un niño de 5 años**. Modo Ayuda (tocar cualquier parte y ver qué hace) + guía de primeros pasos. Desde la Fase 1, no como añadido final.
- Instalación tan fácil como Audacity, VLC u OBS: instalador y portable, sin instalar Python, librerías, FFmpeg ni códecs.
- **Ningún clipeo ni chasquido** de audio y **ningún parpadeo negro** entre vídeos (el usuario lo sufrió en OBS).
- Los visuales están en la mayoría de los shows: el vídeo es núcleo.
- **Mapeo de proyección** sobre figuras del escenario.

**Estado (15/10/2026):** Fase 0 empezada; hay esqueleto del motor en `v2/` (ver sección 10). Boceto interactivo (Edición/Función/Ayuda): `v2/diseno/boceto-pantalla-principal.html`, publicado en https://claude.ai/artifact/REKcem8j1vWFoAJPcNqAA1. Doc de mapeo: https://claude.ai/code/artifact/b6be72d1-fcde-48b6-b0c2-4d14c79abc8c (su contenido está replicado aquí).

**Organización:** la V2 vive en `v2/` para que la V1 siga compilando igual.

---

## 3. Investigación de referencia

- **QLab:** estándar, solo Mac, 25 tipos de cue. Superficies de 4 esquinas, máscaras en grises; un solo cue de vídeo por superficie; trapezoidal difícil de controlar; para mapeos complejos se usa MadMapper. *Audio patches*. Disco amarillo para cues cargados. MSC.
- **Show Cue System:** Windows, de pago. Nivel, pan, *hotkeys*, playlists, *Device Maps*, modo primario/respaldo. Recomienda ensayo técnico una semana antes.
- **MultiPlay:** gratis, Windows. Parar todo / fundir todo, *Audio Patches*, preescucha. Reporte 2024 de chasquidos al arrancar pistas.
- **LivePlay** (Electron) y **Linux Show Player:** alternativas abiertas.
- **MadMapper:** superficies como capas, grupos, malla, bordes suaves, máscaras.
- **Watchout:** primero geometría, encima máscaras y fusión de bordes.
- Hueco: en Windows hay opciones potentes y torpes o simples y limitadas; QLab no existe para Windows.
- Consejo de técnicos: equipo de función limpio, sin internet ni otros programas.

| Programa | Copiamos | Evitamos y por qué |
|---|---|---|
| OBS | Idea de monitor/vista previa. | Carga fuentes al activarse (flash negro), problemas con dos GPU, pide administrador, chasquidos y deriva 44,1/48 kHz. Nace de capturar en vivo; la V2 solo reproduce archivos preparados. |
| VLC | Funciona sin instalar códecs. | No soporta vídeo sin huecos; desmonta la salida entre archivos; negro al cambiar resolución. |
| Resolume | Capas con fundido cruzado, cambios exactos al cuadro, decodificación por GPU (DXV). | No puede precargar todo; en teatro la lista es ordenada y sabemos qué viene: nuestra ventaja. |

Los foros exageran; los fallos citados son estructurales para este uso, no de calidad general.

---

## 4. Principios de diseño

1. **Dos modos, Edición y Función.** **Tab** alterna en cualquier momento sin interrumpir el sonido. Indicador siempre visible arriba (modo activo resaltado + tecla Tab), también clicable; se distingue por etiqueta y posición, no solo color. La lista de cues y el GO no se mueven al cambiar; Edición muestra propiedades, onda y ajustes; Función los esconde y agranda el cue actual y el siguiente. Cada modo con su distribución (personalizable en Fase 2). GO y Parar todo funcionan en ambos. En Edición un cue sonando aparece bloqueado (parar o confirmar). Con foco en un campo de texto, Tab pasa de campo. Opción "Proteger Función" en Vista (desactivada por defecto): salir de Función exige mantener Tab medio segundo.
2. **Botones en Función:** GO grande, Pausa, Fundir todo, Parar todo. Nada más.
3. **Pocos tipos de cue** (unos 4).
4. **Estabilidad antes que funciones.**
5. **Show portable:** una carpeta con proyecto y medios.
6. **Pestañas arriba:** *Show* y *Convertir*.

Atajos aprobados por el usuario (15/10/2026): Espacio GO, Esc Parar todo, P Pausa, F Fundir todo, **Mayús+F fundir solo el último cue que se disparó** (pedido del usuario), ↑↓ mover cue en espera, H Ayuda. Se podrán cambiar en Vista.

## 5. Pestaña Convertir

La V1 como pestaña aparte, en proceso aparte con prioridad baja. Bloqueada en Función; si se entra en Función con una conversión en curso, termina el archivo actual y pausa el resto. Los cues **no** necesitan convertirse antes: el motor prepara cada audio en segundo plano (decodifica a memoria a la frecuencia de la tarjeta). Preajuste "Vídeo para función" para equipos que no aguanten ciertos vídeos.

## 6. Menú Vista

- **Vista → Personalizar:** arrastrar paneles y botones con vista previa en vivo y pulsar **Listo**. Nada de ventanas con cien casillas.
- Personalizable: pestañas y paneles visibles (lista, detalle, onda, medidores, reloj, notas) y su posición; botones de Función (orden, tamaño, color, etiqueta); columnas; teclas rápidas; tamaño de letra; densidad (compacta, normal, táctil); brillo; acento; atajos (avisa si se repiten).
- Vistas: Estándar, Minimalista, Pantalla táctil, Alto contraste, **Modo noche** (tenue y rojizo) y **Modo día** (fondo blanco y contraste máximo para funciones al aire libre, pedido el 15/10/2026). Noche y día se pueden cambiar también en Función.
- Perfiles por operador, exportables, viajan con el show.
- Estética protegida: paletas diseñadas + acento libre con contraste automático; cuadrícula imantada; **Restaurar vista** siempre a mano.
- En Función el diseño queda bloqueado (solo brillo, Modo noche y Modo día).
- GO y Parar todo se pueden mover, agrandar y recolorear, **nunca ocultar**.
- **Desde la Fase 1** la interfaz se construye con la distribución guardada como datos (qué bloque va dónde y con qué tamaño), aunque el editor de arrastrar y soltar llegue en la Fase 2. Así personalizar no obliga a rehacer la interfaz.
- El boceto (`v2/diseno/`) incluye una demo de Vista → Personalizar: arrastrar paneles, barra y botones; letra, densidad, GO grande; vistas listas; Restaurar vista; bloqueo en Función.

## 7. Arquitectura técnica decidida

| Riesgo | Solución |
|---|---|
| Cortes por Python | **Motor en Rust**, en **proceso aparte** de la ventana. La V2 no usa Python en el motor. |
| Disco/decodificación en el GO | Cada audio **entero en memoria** al quedar "en espera". |
| ASIO difícil | **WASAPI por defecto**, **ASIO opcional**. Lista de tarjetas, sin DLLs. **Buffers grandes**. Candidato: crate Rust **cpal** (WASAPI + feature ASIO). Verificar en Fase 0. |
| Tarjetas distintas | **Prueba de hardware** + **comprobación previa**. Si la tarjeta se desconecta: aviso fuerte, nunca cambiar en silencio a los altavoces del PC. |
| Interfaz | La sirve el propio motor. **Mínimo Windows 10 64 bits** (WebView2 dejó de actualizarse en 7/8; última compatible 109). Sin WebView2, se abre en el navegador. |
| Caídas | **Vigilante** que reinicia el motor y recarga el show (objetivo < 1 s, a verificar). En Función se bloquean suspensión y salvapantallas. Guardado automático con copias. Más adelante modo respaldo con segundo PC. |
| Validación | Pruebas automáticas de muchas horas y piloto en 1–2 teatros. |
| No eliminable | Actualizaciones de Windows: la comprobación previa avisa y recomienda pausarlas. |

### Audio
Mezcla en float 32; limitador maestro transparente activo por defecto; **rampas cortas configurables** (ver "Rampas cortas" abajo; decidido por el usuario el 15/10/2026: los audios arrancan de golpe por defecto); medición de picos al preparar (la comprobación previa marca > 0 dB, opción de normalizar); **sin remuestreo en vivo** (todo preparado a la frecuencia real de la tarjeta, leída sola); loops sin hueco. Si el teatro trabaja a otra frecuencia, re-preparación en segundo plano con barra; sin luz verde hasta terminar.

#### Rampas cortas (decidido el 15/10/2026)
El usuario pidió que **por defecto los audios arranquen de golpe** (sin rampa al iniciar) y que **todas** las rampas cortas sean opcionales y configurables. Cada una va de 0 (de golpe) a lo que elija el usuario:

| Rampa | Por defecto | Cuándo actúa |
|---|---|---|
| Inicio | **0 ms (de golpe)** | Al disparar un cue. |
| Reanudar | 5 ms | Al quitar la pausa. |
| Pausa | 10 ms | Al pausar. |
| Parada | 10 ms | Al parar un cue y en Parar todo. |
| Salto | 5 ms | Bajada antes de saltar a otro punto y subida después. |
| Volumen | 5 ms | Duración mínima de un cambio de volumen (cue o maestro). |
| Final del archivo | 10 ms | Últimos ms de un audio que no está en bucle. |

- Son **mínimos**: si el cue tiene un fundido (de entrada o de salida) más largo, manda el fundido. Fundido de entrada 0 + rampa de inicio 0 = arranque de golpe.
- Se cambian **en vivo** sin cortar lo que suena (`Comando::CambiarAjustes`).
- Preajustes en código: `Ajustes::para` (por defecto), `Ajustes::suaves` (todas activas, también inicio 5 ms), `Ajustes::sin_rampas` (todo de golpe).
- Riesgo que hay que explicar en la Ayuda (no se elimina, se avisa): arrancar de golpe solo suena limpio si el archivo empieza en silencio o en un ataque pensado así; si el cue empieza desde un punto intermedio (`desde` > 0) o el archivo empieza a mitad de onda, puede oírse un clic. La comprobación previa debería avisar de los cues con inicio de golpe cuyo primer cuadro no es ~0 (pendiente).
- **Mensajes emergentes de cada rampa** (pedido el 15/10/2026), en `v2/crates/audio/src/consejos.rs` (`TipoRampa::consejo(Idioma)` → título, qué hace, cuándo activar, cuándo dejar en 0; es/en; prueba que exige los dos idiomas y ≤ 200 caracteres por texto). Resumen en español:

| Rampa | Activar (ms recomendados) | Dejar en 0 |
|---|---|---|
| Inicio | 5 ms si el cue empieza a mitad del archivo o se oye un clic; 2–5 ms no se nota. | Golpes, disparos, portazos, música que empieza fuerte; seguro si el archivo empieza en silencio. |
| Reanudar | 5 ms (recomendada): se reanuda a mitad de onda. | Solo corte seco a propósito; puede sonar clic. |
| Pausa | 10 ms (recomendada); 20–50 ms para una pausa más suave. | Corte seco a propósito; puede sonar clic. |
| Parada | 10 ms (recomendada): Parar todo limpio; para salidas lentas, el fundido del cue. | Cortes secos con un apagón de luces; puede sonar clic. |
| Salto | 5 ms (recomendada): el salto cae a mitad de onda. | Casi nunca conviene. |
| Volumen | 5 ms (recomendada): cambio instantáneo = chasquido; fundidos largos en el cue. | Solo pruebas técnicas. |
| Final del archivo | 10 ms si el archivo termina de golpe o a mitad de sonido. | Si ya termina en silencio y se quiere tal cual se editó. |

- Interfaz (Fase 1): en Preferencias del show, sección "Rampas cortas", una fila por rampa con un valor en ms y un interruptor; además, en cada cue, fundido de entrada y de salida. Guardado en el archivo del show. Ayuda para principiantes: "Una rampa es una subida o bajada de volumen tan corta que no se nota, pero evita el clic. Si la pones en 0, el sonido empieza o se corta de golpe".

### Salidas lógicas y perfiles de teatro
El show guarda nombres lógicos (*Principal*, *Auriculares*; luego *Sala*, *Efectos*…); cada teatro tiene un **perfil** que los empareja con tarjeta y canales. Evitar el "primer cue mudo" al cambiar de máquina. Por defecto nada que configurar: *Principal* → salida predeterminada de Windows. El emparejamiento aparece solo cuando hace falta; la app propone coincidencias por nombre, el usuario confirma, se recuerda por teatro. Sin luz verde hasta que cada salida tenga destino. Si faltan salidas: mezcla a estéreo y avisa. Buffer calibrado con la Prueba de hardware y guardado en el perfil. V2.0: solo *Principal* y *Auriculares*; matriz multicanal en Fase 3.

### Vídeo sin negros
- Candidato **mpv/libmpv**, **D3D11VA explícito** con vuelta a CPU si falla (medir en Prueba de hardware).
- **Precarga:** al quedar en espera un cue (y los 2–3 siguientes) se abre y decodifica, primer cuadro listo en GPU.
- **Capas, no reemplazo:** el nuevo se dibuja encima; el anterior se retira cuando el nuevo ya muestra imagen. Nunca un cuadro vacío.
- Un decodificador por cue; ventana de salida fija con escalado por GPU.
- Al terminar: mantener último cuadro (def.), fundir a negro o cortar. Transiciones: corte o fundido cruzado.
- **El reloj de la tarjeta de sonido manda.**
- Ventana de proyección en proceso aparte.
- Pantalla lógica *Proyector* por teatro; en reposo **negro** (nunca escritorio, notificaciones ni cursor); se recoloca si Windows reordena pantallas; comprobación exige modo **Extender** y resolución correcta.
- Decodificadores propios incluidos (MP4, MOV, MKV). **Hap** opcional.
- Portátiles con dos GPU: detectar qué GPU maneja cada salida y dibujar ahí.
- Sin permisos de administrador.

### Mapeo de proyección
- Niveles revelados solo cuando se piden: 1) pantalla completa, 2) superficie de 4 puntos (trapezoidal), 3) máscara con borde suave, 4) malla, 5) fusión de bordes.
- Un vídeo alimenta varias superficies con un solo decodificador.
- Orden: contenido → recorte → deformación → máscara → bordes.
- Vista Mapeo en Edición: puntos arrastrables, flechas 1 px (Mayús 10 px), patrones de prueba, foto del escenario de fondo; posición exacta guardada por teatro, forma y nombre viajan con el show.
- En Función bloqueado; solo miniaturas. Cues apuntan a superficies por nombre (*Árbol*, *Telón*).
- Deformación en GPU en el mismo paso de dibujo. Comprobación previa compara vídeos simultáneos pedidos con lo medido. Plantillas por tipo de superficie.
- Fuera de V2: mapeo 3D y seguimiento de objetos.

### Instalación y distribución
Instalador + portable con todo dentro (motor, vídeo, decodificadores, **runtime de C incluido** para no pedir el VC++ Redistributable: verificar en Fase 0). Requisitos: Windows 10 64 bits y drivers normales. Compilación por GitHub Actions.

### Licencia (decidida el 15/10/2026)
**V2 gratuita y de código abierto, GPLv3.** Compatible con mpv (GPLv2+) y con ASIO (doble licencia, GPLv3 desde octubre de 2025). Añadir el archivo de licencia GPLv3 en `v2/` al empezar el código.

---

## 8. Hoja de ruta V2

- **Fase 0 — Prueba técnica del motor y Prueba de hardware.** Prototipo Rust (WASAPI/ASIO + libmpv) en procesos separados. Prueba de hardware con informe estándar (tarjeta, canales, frecuencias, cortes, latencia, tono por salida, estrés, vídeo 1080p 60 s contando cuadros perdidos, GPU por salida, cambio 44,1/48 midiendo chasquidos y deriva). Pruebas automáticas de cientos de cambios de cue (cuadros negros, discontinuidades, picos): si una falla, no se avanza. Pruebas de horas. Verificar vigilante < 1 s, runtime de C, D3D11VA con vuelta a CPU, ASIO opcional. Lista de equipos probados. Ideal: notebook con dos GPU.
- **Fase 1 — Reproductor mínimo útil.** Lista de cues; GO con espacio; volumen; fade in/out; loop; inicio/fin sobre onda; parar/pausar/pánico; elección de tarjeta; modos con Tab e indicador; guardar/abrir show; preescucha; cues de audio, vídeo e imagen; *Proyector* con 4 puntos; miniatura; *Principal* + *Auriculares* y perfiles; pestañas, paletas, Vista básica (paneles, letra, Modo noche, Modo día); **Ayuda para principiantes**; interfaz en español e inglés. Además (añadido el 15/10/2026 tras comparar con QLab, SCS y MultiPlay): deshacer/rehacer en Edición; notas por cue (lo que ve el operador antes del GO); numeración libre (1, 1.5, 2A) para coincidir con el libreto; parar o fundir un solo cue desde el panel "Sonando"; ir a cue y empezar un cue desde un punto (ensayos); protección contra doble GO (tiempo mínimo entre GOs, ajustable); volumen maestro y medidor de nivel simple; "Reunir medios" (copiar todos los archivos a la carpeta del show); registro de la función (qué se disparó, cuándo y cualquier error); archivo del show con número de versión para abrir shows viejos en versiones nuevas. Justo después: pestaña Convertir.
- **Fase 2 —** Cue de Fade; esperas y auto-continuar; Grupos; Playlist; teclas rápidas; exportar/imprimir la hoja de cues para el regidor; Vista → Personalizar, perfiles de operador, botones a medida.
- **Fase 3 — Multisalida y mapeo.** Matriz de ruteo, pan, medidores, maestro; varias pantallas; varias superficies, máscaras, patrones, foto de fondo; **malla** (adelantada desde la Fase 4 porque el usuario proyecta seguido sobre superficies curvas).
- **Fase 4 — Vídeo avanzado.** Capas, transiciones, texto; fusión de bordes (el usuario no usa dos proyectores solapados: prioridad baja), 4 puntos animado.
- **Fase 5 — Control externo.** GO desde celular por Wi-Fi local; MIDI/OSC; MSC (GO, STOP, LOAD).
- **Siempre:** guardado automático, comprobación previa, modo respaldo.
- **Fuera de V2:** 25 tipos de cue, DMX, scripting, plugins, edición de audio avanzada, colaboración en línea, mapeo 3D.

## 9. Preguntas abiertas (no inventar respuestas)

1. V1: ¿lista virtualizada para miles de archivos?
2. Menor: actualizar acciones del workflow (Node 20).

### Decidido el 15/10/2026
- **Boceto aprobado** por el usuario.
- Probará la Prueba de hardware en **2 o 3 equipos o teatros**.
- Mapeo: proyecta sobre **superficies planas, siluetas y superficies curvas**; no usa dos proyectores solapados.
- Licencia GPLv3 gratuita (ver sección 7).
- **Idiomas de la interfaz: español e inglés** (también la Ayuda).
- **Solo Windows, versiones 10 y 11 en adelante, 64 bits.** No macOS. **Windows 7 descartado** por el usuario tras ver el impacto: Rust ≥ 1.78 solo admite Windows 7 con un target de nivel 3, WebView2 se congeló en la 109 y mpv dejó de funcionar en Windows 7 desde la 0.37 (issue #16607). No volver a proponerlo.
- El usuario delega en Claude el reparto de funciones entre fases, siempre que sea lo mejor para el proyecto. Las 9 funciones esenciales pasaron a la Fase 1; la hoja de cues a la Fase 2.

## 10. Fase 0 en curso

### Plan aprobado por el usuario (15/10/2026)
1. Esqueleto en `v2/` (hecho: ver abajo).
2. Prototipo de audio: tono, rampas, prueba automática de cientos de disparos/paradas buscando chasquidos, discontinuidades y picos.
3. Prototipo de vídeo: precarga y capas; prueba automática que cuenta cuadros negros o perdidos en cientos de cambios. **Es el mayor riesgo y no está probado.**
4. Vigilante: reinicio del motor, medir < 1 s.
5. Prueba de hardware descargable con informe; el usuario la ejecuta en su PC y en 2–3 equipos más.
Regla: si una prueba automática falla, no se avanza.

### Esqueleto (`v2/`)
- `Cargo.toml` (workspace, versión 0.0.1, edición 2021, MSRV 1.85, GPL-3.0-or-later), `LICENSE` (GPLv3), `README.md`, `.gitignore` (`/target/`).
- `.cargo/config.toml`: `+crt-static` para `x86_64-pc-windows-msvc` (runtime de C dentro del exe).
- `crates/motor` → `otaconvert-motor.exe` con `--version` y `--listar-salidas` (cpal 0.18.2, WASAPI). Prueba `runtime_de_c_estatico`.
- Workflow `.github/workflows/v2.yml` (separado de `main.yml`): se dispara con cambios en `v2/**`; compila, prueba, comprueba con `dumpbin /dependents` que no hay VCRUNTIME/MSVCP/api-ms-win-crt, ejecuta el motor y sube el artifact `Otaconvert-V2-motor`. `Cargo.lock` en el repo y compilación con `--locked`.
- **Comprobado (15/10/2026):** compila en Actions (Rust 1.99), la prueba pasa, el exe solo depende de DLLs del sistema (sin VCRUNTIME). En la PC del usuario `--listar-salidas` ve WASAPI, "Altavoces (USB Audio CODEC)" y la salida NVIDIA HDMI, ambas 2 canales, 48 kHz, F32, buffer 480 (min = max = 10 ms; probablemente el periodo fijo del modo compartido de WASAPI, sin verificar; resolver cómo lograr buffers grandes en el paso 2).

### Verificado en documentación (15/10/2026)
- **cpal 0.18.2** (16/08/2026, MSRV 1.85): WASAPI por defecto. Nombres de dispositivo con `Display` (`{d}`); ya no hay `name()`, sí `description()` e `id()`. ASIO: feature `asio`; compilar exige SDK de Steinberg (`CPAL_ASIO_DIR` o lo descarga solo), LLVM/clang (`LIBCLANG_PATH`) y Visual Studio. En ejecución usa los drivers de la tarjeta, sin DLL.
- **libmpv2 6.0.0** (crate Rust, requiere mpv ≥ 0.35; en Windows enlaza con archivos en `MPV_SOURCE/64`; su render API solo trae OpenGL). DLL: compilaciones diarias de shinchiro/mpv-winbuild-cmake (`mpv-dev-x86_64-…7z`, ~30 MB); fijar una versión con SHA-256 y guardar copia propia (las diarias rotan). Contenido del .7z sin verificar.
- **mpv**: `hwdec=d3d11va` requiere `vo=gpu`/`gpu-next` con `gpu-context=d3d11`; si los decodificadores de la lista fallan, pasa a software. `--wid=<HWND>` dibuja dentro de una ventana nuestra (crea ventana hija). `--keep-open` mantiene el último cuadro. Idea a probar para capas: dos instancias en ventanas hijas, cambiar orden Z cuando la nueva ya pinta.
- **GitHub Actions windows-latest** (Server 2025): Rust 1.99, LLVM 20.1.8, VS 2022 17.14 con clang, 7zip.

### Paso 2: prototipo de audio (15/10/2026, en curso)
- `crates/audio` (`otaconvert-audio`, sin dependencias salvo `rtrb` 0.4.0, cola sin bloqueo): `Rampa`, `Pista` (audio entero en memoria, f32 intercalado), `Mezclador`/`Control` (`crear`), `Detector`, `prueba::Escenario`.
  - Comandos: Iniciar (volumen, desde, bucle, fundido), Parar, Pausar, Reanudar, Volumen, Saltar, PararTodo, VolumenMaestro. Además `CambiarAjustes`. Siete rampas separadas en `Ajustes` (ver tabla "Rampas cortas", sección 7); por defecto inicio de golpe. Salto = bajar, mover, subir.
  - El hilo de audio no pide ni libera memoria: los `Arc<Pista>` que dejan de sonar vuelven por una cola "basura" y se liberan en el hilo de control. Rechaza pistas a otra frecuencia (sin remuestreo en vivo).
  - Salida recortada a ±1 y contada como "recortes". **Pendiente: limitador maestro transparente.**
  - Detector: segunda diferencia `x[n]-2x[n-1]+x[n-2]` por canal; umbral para tonos `1,5·A·(2·sin(πf/fs))² + 0,002`.
- Pruebas (`cargo test --release`, 16): arranque de golpe por defecto, rampa de inicio opcional, cambio de ajustes en vivo, parada de golpe sin rampas, fundido del cue por encima de la rampa; estrés con `Ajustes::suaves` de 10 min × 3 semillas (~1.850 órdenes cada una, bloques de 1–1024 cuadros): 0 discontinuidades (máxima 0,0023, umbral 0,0127), 0 recortes, 0 rechazos. Control: sin rampas el detector encuentra 568 en 2 min. `sin_memoria`: asignador que cuenta en el hilo de `procesar` (comprobado que falla si se pide memoria).
- Motor: `--tono [s] [margen_ms]` y `--prueba-vivo [s] [margen_ms]` (salida predeterminada, modo compartido, `BufferSize::Fixed(margen)`, con `Ajustes::suaves` para que el detector no confunda un arranque de golpe con un corte; mide intervalos entre llamadas, cortes probables = intervalo > margen, discontinuidades, recortes, errores; termina con "RESULTADO: BIEN" o código de error). cpal con feature `realtime` (MMCSS).
- Verificado en el código de cpal 0.18.2 (`host/wasapi`): solo modo compartido; el periodo de llamada es siempre `GetDevicePeriod` (10 ms) y el `BufferSize::Fixed` solo agranda el búfer de WASAPI (más margen y más latencia); cada llamada llena todo el hueco libre. Solo F32 por ahora.
- Compilar en la PC del usuario: no tiene Visual Studio. Toolchain temporal GNU en `%TEMP%\otr` (sirve para `cargo test -p otaconvert-audio` y `cargo check`; el motor no enlaza con GNU porque falta `as.exe`). Ruta corta obligatoria: el scratchpad supera MAX_PATH. El ejecutable real sale de GitHub Actions (MSVC).
- Pendiente del paso 2: ejecutar `--prueba-vivo` en la PC del usuario con el exe de Actions y decidir el margen por defecto.
