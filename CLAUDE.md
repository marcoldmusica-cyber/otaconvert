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
- **Manual de usuario** (creado el 15/10/2026): `v2/manual/manual-usuario.html` + `v2/manual/manual-usuario.pdf`, con la estética del programa (el PDF y la impresión usan **siempre el tema oscuro normal**, nunca fondo blanco: decisión del usuario). **Regla:** cada cambio (correcciones de esta versión y versiones futuras) se anota en CLAUDE.md **y** en el manual de usuario, y después se regenera el PDF con `v2/manual/generar-pdf.ps1` (usa Edge, que viene con Windows). Lo que aún no funciona se marca "Próximamente" y lo que sí, "Ya funciona".
- **Sin botones de ajustes sueltos (15/10/2026):** los ajustes del programa (mostrar consejos, idioma, vistas, ajustes del show) van en la barra de menús de arriba (sección 6), no como botones en la ventana.
- **Regla de interfaz (15/10/2026):** cada opción configurable lleva un **mensaje emergente sutil** (al pasar el ratón o tocar "?") que recomienda en qué casos activarla o desactivarla y qué valor usar. Cada mensaje incluye **ejemplos de escena** ("si en una escena pasa X, actívala para Y"), uno para activar y otro para desactivar. **Idioma:** los mensajes salen siempre en el idioma configurado del programa; si el programa está en inglés todo está en inglés, y lo mismo en español (nunca mezclados). **Opcionales:** se pueden ocultar (para usuarios avanzados) con la casilla **Ayuda → Mostrar consejos** de la barra de menús; activados por defecto y **siempre se pueden volver a activar** (esa casilla nunca se oculta, ni siquiera con Vista → Personalizar).
 Los textos viven en el código, en español e inglés, junto a la opción, y se resumen en este archivo.
- **Avisar antes de cambios riesgosos (pedido el 16/10/2026):** si llegamos a un punto en que agregar funciones o cambiar algo sea riesgoso (por ejemplo, cuando ya haya una versión usada en funciones reales, cerca de un estreno, o al tocar el motor de audio o vídeo, el formato del archivo del show o la instalación), **avisar antes** al usuario, explicar el riesgo en simple y proponer cómo hacerlo seguro (rama aparte, versión estable separada, pruebas extra, piloto) antes de tocar nada.
- **Crédito del autor (pedido el 16/10/2026):** "Desarrollado por **@marcold.ok**" (Instagram del usuario), invitando a escribir para sugerir mejoras y reportar errores. Va en: el manual de usuario (sección "Libre y gratuito" y pie de página), la ventana del programa (línea discreta abajo a la derecha y, desde la etapa 3B, **Ayuda → Acerca de Otaconvert**), `--version`, la cabecera del informe de la Prueba de hardware y el README. Siempre en un rincón, sin estorbar.
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
   - **Que funcione en equipos modestos sin sacrificar funciones ni experiencia** (deseo del usuario, 15/10/2026): se logra por eficiencia, no recortando: decodificación por la tarjeta gráfica, sin remuestreo en vivo, audio en memoria, motor en Rust, precarga solo de lo necesario. La Prueba de hardware mide qué aguanta cada equipo (resolución, códec, cuántos vídeos a la vez) y la comprobación previa avisa si un show pide más de lo que el equipo puede; el preajuste "Vídeo para función" de Convertir ayuda en equipos justos. Nunca se quita una función por defecto pensando en equipos lentos.
5. **Show portable:** una carpeta con proyecto y medios.
6. **Pestañas arriba:** *Show* y *Convertir*.

Atajos aprobados por el usuario (15/10/2026): Espacio GO, Esc Parar todo, P Pausa, F Fundir todo, **Mayús+F fundir solo el último cue que se disparó** (pedido del usuario), ↑↓ mover cue en espera, H Ayuda. Se podrán cambiar en Vista.

## 5. Pestaña Convertir

La V1 como pestaña aparte, en proceso aparte con prioridad baja. Bloqueada en Función; si se entra en Función con una conversión en curso, termina el archivo actual y pausa el resto. Los cues **no** necesitan convertirse antes: el motor prepara cada audio en segundo plano (decodifica a memoria a la frecuencia de la tarjeta). Preajuste "Vídeo para función" para equipos que no aguanten ciertos vídeos.

## 5b. Botonera de sonidos (pedida por el usuario el 15/10/2026)

Panel con sonidos sueltos que se disparan a mano **superpuestos** a lo que esté sonando en cualquier cue (aplausos, timbre, risas, un trueno para improvisar…).

- **Dos formas de verla:** **botones cuadrados** (cuadrícula) o **lista de sonidos**. Se elige en **Vista → Botonera ▸ Cuadrícula / Lista** (regla de la barra de menús). El panel se muestra u oculta en Vista → Personalizar, como cualquier otro; viene **oculto** por defecto para no sumar botones a quien no lo usa.
- **No toca la lista de cues:** disparar un botón no mueve el cue en espera ni cuenta como GO.
- **Cada botón:** nombre, color, sonido (de la carpeta del show), volumen, fundido de entrada y de salida, bucle, salida lógica (*Principal* / *Auriculares*), tecla rápida opcional (avisa si choca con otra) y qué pasa si se toca otra vez mientras suena: **parar** (por defecto), **volver a empezar** o **superponer** otra copia. Cada opción con su consejo emergente y ejemplos de escena (regla de interfaz).
- **Varias páginas** por show (p. ej. "Acto 1", "Improvisación"); viajan con el show.
- **Edición y Función:** en las dos se disparan; en Función no se editan.
- **Parar todo** (Esc) y **Fundir todo** (F) también los paran o funden (pánico). **Mayús+F** (fundir el último cue) no los toca: un botón se funde tocándolo otra vez si tiene fundido de salida. A confirmar con el usuario.
- **Motor:** los sonidos de la botonera quedan **siempre en memoria** desde que se abre el show (no solo al quedar en espera), para que suenen al instante; el mezclador ya mezcla voces superpuestas (hasta 64) con las mismas rampas configurables. Cuentan para la comprobación previa (memoria, picos > 0 dB).
- **Fase 1** (el motor ya lo soporta; es un panel más). Pendiente: agregarla al boceto.
- Ayuda para principiantes: "La botonera es como un teclado de efectos: cada botón hace sonar algo cuando lo tocas, aunque ya esté sonando otra cosa".

## 6. Barra de menús y menú Vista

### Barra de menús (decidido el 15/10/2026)
El usuario pidió **no llenar la ventana de botones** para ajustes del tipo "mostrar/ocultar mensajes". Esos ajustes van en una **barra de menús desplegables arriba de todo** (como Archivo · Edición · Vista · Ayuda en cualquier programa). En la ventana solo quedan los botones de trabajo: GO, Pausa, Fundir todo, Parar todo, el indicador Edición/Función y las pestañas Show/Convertir. Solo los menús necesarios:

| Menú | Contenido |
|---|---|
| **Archivo** | Nuevo show · Abrir… · Abrir reciente ▸ · Guardar · Guardar como… · Reunir medios… · Salir |
| **Edición** | Deshacer · Rehacer · Copiar · Pegar · Eliminar cue · **Ajustes del show…** (rampas cortas, salidas y perfiles de teatro, protección contra doble GO) |
| **Vista** | Personalizar… · Vistas listas ▸ · Modo noche · Modo día · Botonera ▸ (Cuadrícula / Lista) · Idioma ▸ (Español / English) · Restaurar vista |
| **Ayuda** | Modo Ayuda (H) · Primeros pasos · **Mostrar consejos** ✓ · Atajos de teclado · Prueba de hardware… · Acerca de Otaconvert |

- Cada opción aparece **una sola vez** (nada repetido en dos menús ni como botón suelto).
- En Función los menús siguen ahí pero lo que edita queda gris: solo Vista → Modo noche/Modo día/brillo, Archivo → Guardar y todo Ayuda. Abrir un menú nunca roba Espacio (GO) ni Esc (Parar todo). **Decidido en la etapa 3B (16/10/2026): Esc para todo siempre** (también con un menú o una ventanita abierta, o escribiendo) y el menú se cierra a la vez; así el pánico nunca se retrasa.
- Boceto actualizado el 15/10/2026: la barra de menús ya está en `v2/diseno/boceto-pantalla-principal.html` (Vista y Ayuda dejaron de ser botones de la cabecera; el Modo Ayuda se activa desde Ayuda → Modo Ayuda o con H). Captura de cómo se ve al abrir, a 1366×768: `v2/diseno/pantalla-inicio.png` (hecha con Edge sin ventana: `msedge --headless=new --window-size=1366,768 --screenshot=…`). Comprobado en el navegador: los menús abren y cierran, en Función Edición queda en gris y Guardar sigue activo, Mostrar consejos se apaga y se vuelve a encender. La versión publicada del boceto (enlace de la sección 2) todavía es la anterior.

### Menú Vista

- **Vista → Personalizar:** arrastrar paneles y botones con vista previa en vivo y pulsar **Listo**. Nada de ventanas con cien casillas.
- Personalizable: pestañas y paneles visibles (lista, detalle, onda, medidores, reloj, notas) y su posición; botones de Función (orden, tamaño, color, etiqueta); columnas; teclas rápidas; tamaño de letra; densidad (compacta, normal, táctil); brillo; acento; atajos (avisa si se repiten).
- Vistas: Estándar, Minimalista, Pantalla táctil, Alto contraste, **Modo noche** (tenue y rojizo) y **Modo día** (fondo blanco y contraste máximo para funciones al aire libre, pedido el 15/10/2026). Noche y día se pueden cambiar también en Función.
- Perfiles por operador, exportables, viajan con el show.
- Estética protegida: paletas diseñadas + acento libre con contraste automático; cuadrícula imantada; **Restaurar vista** siempre a mano.
- En Función el diseño queda bloqueado (solo brillo, Modo noche y Modo día).
- GO y Parar todo se pueden mover, agrandar y recolorear, **nunca ocultar**.
- La casilla **Ayuda → Mostrar consejos** (mensajes emergentes con recomendaciones y ejemplos de escena), activada por defecto. Igual que GO y Parar todo, **nunca se puede ocultar**: siempre se pueden volver a activar los consejos. Los consejos siguen el idioma del programa.
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
- **Mensajes emergentes de cada rampa** (pedidos el 15/10/2026; ejemplos de escena añadidos el mismo día), en `v2/crates/audio/src/consejos.rs`: `TipoRampa::consejo(Idioma)` devuelve título, qué hace, cuándo activar + ejemplo de escena, cuándo dejar en 0 + ejemplo de escena. `consejo_visible(idioma, PreferenciasConsejos)` devuelve nada si el usuario los ocultó. Pruebas: los dos idiomas completos y distintos (nada sin traducir), ejemplos que empiezan por "Ej.: "/"E.g.: ", ≤ 200 caracteres por texto, ocultar y volver a mostrar. Resumen en español:

| Rampa | Activar (ms) · ejemplo | Dejar en 0 · ejemplo |
|---|---|---|
| Inicio | 5 ms si el cue empieza a mitad del archivo o hay clic. *Al abrir el telón la música entra desde el estribillo.* | Sonidos que deben pegar con toda su fuerza; seguro si el archivo empieza en silencio. *El disparo de la escena final, el portazo del segundo acto.* |
| Reanudar | 5 ms (recomendada). *Pausas la lluvia de fondo durante un parlamento y la retomas.* | Corte seco a propósito. *Una radio en escena que se "enciende" de golpe al tocarla.* |
| Pausa | 10 ms; 20–50 ms más suave. *Congelas la música de una coreografía cuando los bailarines se quedan quietos (30 ms).* | Corte seco a propósito. *Efecto de "tiempo detenido".* |
| Parada | 10 ms (recomendada). *Un actor se adelanta y cortas ambiente, música y efectos con Parar todo.* | Corte seco a propósito. *Apagón total: el sonido se corta con la luz.* |
| Salto | 5 ms (recomendada). *En el ensayo el director pide repetir desde el minuto 2.* | Casi nunca. *Solo pruebas técnicas.* |
| Volumen | 5 ms (recomendada). *Bajas el ambiente de la fiesta cuando empieza un diálogo.* | Solo pruebas técnicas. *Medir la tarjeta en la Prueba de hardware.* |
| Final del archivo | 10 ms si termina de golpe. *Un efecto de motor recortado a mano que acaba en pleno ruido.* | Si ya termina en silencio. *Una canción con su final desvanecido completo.* |


- Interfaz (Fase 1): en **Edición → Ajustes del show…**, sección "Rampas cortas", una fila por rampa con un valor en ms y un interruptor; además, en cada cue, fundido de entrada y de salida. Guardado en el archivo del show. Ayuda para principiantes: "Una rampa es una subida o bajada de volumen tan corta que no se nota, pero evita el clic. Si la pones en 0, el sonido empieza o se corta de golpe".

### Salidas lógicas y perfiles de teatro
**Elegir la salida es FUNDAMENTAL** (dicho por el usuario el 15/10/2026), tanto la de **audio** como la de **vídeo**: el usuario siempre puede elegir a qué tarjeta y canales va cada salida lógica y en qué pantalla sale el Proyector. Si la elegida no está, se avisa y **nunca** se cambia en silencio a otra. Estado: en el motor ya se elige la salida de audio (`--salida <número | nombre | id>`, id estable de cpal) y se listan las pantallas (`--listar-pantallas`: nombre del monitor/proyector por EDID con DisplayConfig, tarjeta gráfica que la maneja, resolución en píxeles reales con conciencia de DPI por monitor, frecuencia, posición, principal). Falta: abrir la ventana de vídeo en la pantalla elegida (prototipo de vídeo) y la pantalla de elección en la interfaz (Edición → Ajustes del show → Salidas, Fase 1).
**Probado en la PC del usuario el 15/10/2026** (exe de Actions run 38804864704, SHA-256 `5D8ACA0B…6B687303` verificado): `--listar-salidas` muestra "1. Altavoces (USB Audio CODEC)" (predeterminada) y "2. C24F390 (NVIDIA High Definition Audio)" con sus id; `--listar-pantallas` muestra "C24F390 (principal), 1920 × 1080 a 60 Hz, NVIDIA GeForce GTX 1050 Ti, `\\.\DISPLAY1`" y avisa que hay una sola pantalla. Prueba en vivo de 20 s por la salida 1 (elegida por número) y por la 2 (por nombre "NVIDIA"), y tono por la 2 (por id): todo **BIEN**, 0 cortes, 0 discontinuidades. Salida inexistente (`--salida 9`, `--salida "no existe"`): error con la lista de salidas y código 1, sin sonar por otra. En GitHub (equipo virtual) lista "HyperVMonitor, Microsoft Hyper-V Video". Pendiente: probar con un proyector conectado en modo Extender.

El show guarda nombres lógicos (*Principal*, *Auriculares*; luego *Sala*, *Efectos*…); cada teatro tiene un **perfil** que los empareja con tarjeta y canales. Evitar el "primer cue mudo" al cambiar de máquina. Por defecto nada que configurar: *Principal* → salida predeterminada de Windows. El emparejamiento aparece solo cuando hace falta; la app propone coincidencias por nombre, el usuario confirma, se recuerda por teatro. Sin luz verde hasta que cada salida tenga destino. Si faltan salidas: mezcla a estéreo y avisa. Buffer calibrado con la Prueba de hardware y guardado en el perfil. V2.0: solo *Principal* y *Auriculares*; matriz multicanal en Fase 3.

### Vídeo sin negros
- **Salida de vídeo de hasta 4K** (requisito confirmado por el usuario el 15/10/2026): proyector o pantalla de hasta 3840 × 2160, objetivo 60 Hz, con vídeos H.264 y HEVC de hasta 4K60 decodificados por la GPU. La Prueba de hardware mide la resolución real de la salida del teatro (incluida 4K) con un vídeo de 60 s contando cuadros perdidos.
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
- **Versión portable obligatoria** (confirmado por el usuario el 15/10/2026): un `.zip` que se descomprime y se ejecuta, también desde un pendrive. Igual que la V1: un archivo `portable.txt` junto al exe hace que toda la configuración (preferencias, perfiles de teatro, vistas) se guarde en esa carpeta y no en el usuario de Windows; no escribe en el registro ni pide administrador. Mismo contenido y mismas funciones que el instalador. Se publica en cada Release junto al instalador, con su huella SHA-256.
- **Funciona 100 % sin internet** (confirmado el 15/10/2026): internet solo para descargarlo. Audio, vídeo, Ayuda, consejos y Convertir funcionan sin conexión; la búsqueda de actualizaciones viene apagada por defecto.
- **Componentes faltantes al instalar o al abrir por primera vez** (pedido por el usuario el 15/10/2026): todo lo que haga falta se **avisa**, con su **botón para instalarlo**, sin ser engorroso:
  1. Primero se **incluye todo lo posible** dentro del programa, para que normalmente no falte nada (runtime de C, libmpv, decodificadores; **cargador de Vulkan** `vulkan-1.dll` 1.4.363.0 de LunarG junto a libmpv, decidido el 15/10/2026; WebView2).
  2. El instalador y el primer arranque hacen la comprobación. **Si no falta nada, no se muestra nada.**
  3. Si falta algo: **una sola ventana** con la lista en palabras simples (qué es y para qué sirve) y **un solo botón "Instalar lo que falta"**, más "Ahora no". Instala con instaladores **incluidos** (sin internet), por usuario y **sin administrador** cuando se pueda; si algo pidiera administrador, se dice antes. Nunca se instala nada sin que el usuario toque el botón.
  4. Lo que no se puede instalar desde Otaconvert (driver de la tarjeta gráfica, versión de Windows) se explica con un botón "Cómo arreglarlo".
  5. La misma comprobación está en Ayuda → Prueba de hardware y en la comprobación previa (luz verde).
  - Hecho en el motor: `--comprobar` revisa Windows 10/11 (compilación ≥ 14393), libmpv junto al exe (se carga de verdad), `vulkan-1.dll` (de la carpeta del programa o de System32) y **WebView2** (claves de registro documentadas por Microsoft: `pv` en `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` y en `HKCU\Software\…` igual; vacío o 0.0.0.0 = falta). Código de salida 1 si falta algo. Probado en la PC del usuario: todo bien (Windows 19045 22H2, WebView2 155.0.4283.45); con libmpv escondida a propósito la marca como faltante con su arreglo.
  - Verificado (Microsoft Learn, "Distribute your app and the WebView2 Runtime"): Windows 11 trae WebView2 y casi todos los Windows 10 también; el **instalador independiente** (Evergreen Standalone) funciona **sin internet** y, sin elevar, instala **por usuario** (`/silent /install`).
  - **Decidido por el usuario (15/10/2026, noche): el instalador independiente de WebView2 (más de 100 MB) va en el instalador Y en la portable.** Si falta, el botón "Instalar lo que falta" lo instala sin internet y por usuario. No se publica suelto en la copia de dependencias (la documentación de Microsoft lo indica para incluirlo dentro del instalador de la aplicación): la compilación lo baja del enlace oficial `https://go.microsoft.com/fwlink/?linkid=2124701` (redirige a `MicrosoftEdgeWebView2RuntimeInstallerX64.exe`) y **comprueba la firma digital de Microsoft** antes de incluirlo. Se implementa al armar el instalador y la portable del programa (Fase 1).

- **Puntos de cuidado de la portable** (aprobado el 15/10/2026):
  1. Pendrive: los audios se cargan en memoria antes del GO (no les afecta), pero los vídeos se leen del disco mientras suenan. La comprobación previa avisa si el programa o la carpeta del show están en una unidad extraíble y recomienda copiar el show al disco interno.
  2. Carpeta sin permiso de escritura (p. ej. dentro de Archivos de programa): al abrir se avisa que no se podrán guardar ajustes ni copias automáticas, y se propone moverlo.
  3. Aviso de Windows (SmartScreen) la primera vez por no estar firmado; igual que el instalador; se resuelve con la firma (sección Seguridad).
- **Flujo de trabajo con los medios** (confirmado por el usuario): el material vive en la **carpeta del show** (archivo del proyecto + medios); desde ahí se agregan los cues. "Reunir medios" copia a esa carpeta lo que se haya agregado desde otro lugar. Los audios se leen de ahí al quedar en espera y pasan a memoria; los vídeos se leen de ahí durante la reproducción.

### Licencia (decidida el 15/10/2026)
**V2 gratuita y de código abierto, GPLv3.** Compatible con mpv (GPLv2+) y con ASIO (doble licencia, GPLv3 desde octubre de 2025). Añadir el archivo de licencia GPLv3 en `v2/` al empezar el código.

---

### Seguridad (revisión del 15/10/2026, pedida por el usuario)
Objetivo: que el programa no abra puertas a gente malintencionada de fuera. Principio: **nada escucha en la red, nada se ejecuta desde un archivo de show y nada se carga desde carpetas que otro pueda escribir**, salvo lo que el usuario active a propósito.

**Hecho y comprobado**
- **Carga de DLL (motor V2):** `/DEPENDENTLOADFLAG:0x800` en `v2/.cargo/config.toml` (importaciones del exe solo desde System32; documentado por Microsoft, Windows 10 1607+) y `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)` al arrancar (cargas posteriores solo desde la carpeta del programa y System32, nunca la carpeta actual ni el PATH). El workflow falla si `dumpbin /loadconfig` no muestra el 0x800. Evita que una DLL falsa dejada en Descargas o en la carpeta del show se cuele.
- **Librerías Rust:** las 92 de `v2/Cargo.lock` consultadas en OSV (incluye RustSec): 0 vulnerabilidades conocidas. Se comprobó que la consulta detecta una real (`time` 0.1.43 → RUSTSEC-2020-0071). El workflow V2 corre `cargo audit --deny warnings` en cada cambio y **cada lunes** (job `vulnerabilidades`, en Linux). En la PC del usuario no se puede instalar cargo-audit (falta `as.exe`); para revisar a mano: consulta por lotes a `https://api.osv.dev/v1/querybatch`.
- **Workflow V2:** token de solo lectura (`permissions: contents: read`); acciones fijadas por SHA (checkout v7.0.1 `3d3c42e…`, upload-artifact v7.0.2 `cf430e0…`); `persist-credentials: false`; sin secretos; los PR de forks corren sin secretos ni escritura. Imprime la huella SHA-256 del exe.
- **Motor V2 hoy:** no abre puertos, no usa internet, no lee archivos de show todavía; el hilo de audio no pide memoria; `unsafe` solo en la llamada a `SetDefaultDllDirectories` y en el asignador de la prueba `sin_memoria`.
- **V1 (Python):** FFmpeg se llama con lista de argumentos (sin `shell=True`) y rutas absolutas (un nombre de archivo no puede pasar por opción); la configuración JSON se lee con `try/except` y no ejecuta nada; no hay red.
- **Repositorio (GitHub):** público; escaneo de secretos y bloqueo de subida de secretos **activados**; permisos por defecto de los workflows: solo lectura.

- **Verificado en GitHub Actions el 15/10/2026** (run 38803101598, commit 59ac7a1): 21 pruebas en verde; el exe solo depende de DLL del sistema; `dumpbin /loadconfig` muestra `0800 Dependent Load Flag`; el motor arranca; `cargo audit` cargó 1.329 avisos de RustSec, revisó 94 librerías y no encontró nada. Huella del exe de esa compilación: `E907AE88…52B45E5`.

**Pendiente: necesita el permiso del usuario (ajustes de GitHub)**
- Activar **Dependabot alerts** (avisos de vulnerabilidades; hoy desactivado).
- Proteger la rama `main` contra borrado y `force push`.
- `main.yml` (V1): fijar acciones por SHA y dar `contents: write` solo al paso de Release. No se tocó porque el plan decía no modificarlo.
- V1: `requirements.txt` usa `>=` (cada compilación baja la última versión). Fijar versiones exactas haría las compilaciones repetibles.

**Reglas para lo que viene (obligatorias al implementar)**
1. **Interfaz servida por el motor:** escuchar solo en `127.0.0.1` (nunca `0.0.0.0`), puerto al azar, **token secreto por sesión** que solo conoce la ventana; rechazar peticiones con `Host` u `Origin` distintos (contra páginas web maliciosas y *DNS rebinding*); sin CORS abierto.
2. **GO desde el celular / OSC / MIDI por red (Fase 5):** **apagados por defecto**; al activarlos, emparejar con un código o PIN que se muestra en pantalla, solo red local, solo órdenes de show (GO, Parar, Pausa), nunca acceso a archivos ni ajustes; indicador visible mientras estén activos; límite de órdenes por segundo; el aviso del Firewall de Windows aparece solo entonces.
3. **Archivo del show:** formato de datos (JSON), sin scripts ni nada ejecutable; límites de tamaño; rutas relativas **dentro** de la carpeta del show (rechazar `..`, rutas absolutas inesperadas y rutas de red `\servidor\…`, que pueden filtrar la contraseña de Windows); un show dañado o malicioso nunca debe colgar ni cerrar el programa, solo avisar.
4. **Vídeo con libmpv:** decodificar en proceso aparte (ya decidido); configurarlo solo para archivos locales: `config=no`, `load-scripts=no`, `ytdl=no`, sin servidor IPC, sin OSC/teclado de mpv, protocolos limitados a archivo; libmpv cargada con **ruta completa** desde la carpeta del programa. La DLL de terceros (shinchiro) se fija por SHA-256 y se guarda copia propia; actualizarla cuando FFmpeg/mpv publiquen arreglos de seguridad.
5. **Sin administrador** (ya decidido) y **sin telemetría**. Buscar actualizaciones solo si el usuario lo activa, por HTTPS, y nunca instalar sin preguntar.
6. **Distribución:** publicar la huella SHA-256 de cada descarga en la Release; firmar el instalador cuando sea posible (para software libre existe la firma gratuita de SignPath Foundation; verificar requisitos antes de pedirla); certificados de origen de GitHub (*artifact attestations*) al publicar Releases.
7. **Registro de la función:** sin datos personales ni rutas privadas fuera de la carpeta del show.
8. Cada función nueva que toque red, archivos de fuera o DLL se revisa contra esta lista antes de subirla.

### Manual de usuario (15/10/2026)
- **Índice** (pedido: "muy fresco, intuitivo y al grano"): 4 grupos por momento del trabajo, numerados: **1 Empezar** (Qué es · Instalarlo · Tu primer show en 5 pasos), **2 Preparar el show** (La carpeta del show · La pantalla · Edición y Función · Elegir salidas · Vídeo y proyector · Clics: las rampas · Cómo se ve), **3 En la función** (Los 4 botones · Teclas rápidas · Antes de empezar · Si algo falla), **+ Más** (Ayuda y consejos · Convertir archivos · Datos técnicos · Libre y gratuito). Nombres cortos. **Buscador** arriba (busca también dentro de cada sección, sin importar tildes; "Nada encontrado" si no hay). La sección que se está leyendo se resalta. El contenido sigue el mismo orden que el índice. En el PDF el índice es la primera página. Cada sección nueva entra en uno de esos 4 grupos.
- Para no expertos, en español; secciones: bienvenida, "Hecho por amor al arte" (libre y gratuito, GPLv3, frase **"Nadie se salva solo."** al principio y al final), qué es, instalar y portable, carpeta del show, la pantalla (con `img/pantalla-inicio.png`), primer show, Edición y Función, botones, teclas, vídeo y proyector, rampas cortas con ejemplos, consejos y Modo Ayuda, Vista, antes de la función, si algo sale mal, Convertir, y **Especificaciones técnicas** para usuarios avanzados (plataforma, audio, pruebas, vídeo, seguridad, comandos del motor, licencias).
- Sin nada de internet (funciona sin conexión). Botón Oscuro/Día solo para leer en pantalla; PDF siempre oscuro. 16 páginas; páginas revisadas como imagen con la API de PDF de Windows.
- Pendiente: versión en inglés del manual (la interfaz es es/en); incluirlo en el instalador y en la portable; abrirlo desde Ayuda.

## 8. Hoja de ruta V2

- **Fase 0 — Prueba técnica del motor y Prueba de hardware.** Prototipo Rust (WASAPI/ASIO + libmpv) en procesos separados. Prueba de hardware con informe estándar (tarjeta, canales, frecuencias, cortes, latencia, tono por salida, estrés, vídeo 1080p 60 s contando cuadros perdidos, GPU por salida, cambio 44,1/48 midiendo chasquidos y deriva). Pruebas automáticas de cientos de cambios de cue (cuadros negros, discontinuidades, picos): si una falla, no se avanza. Pruebas de horas. Verificar vigilante < 1 s, runtime de C, D3D11VA con vuelta a CPU, ASIO opcional. Lista de equipos probados. Ideal: notebook con dos GPU.
- **Fase 1 — Reproductor mínimo útil.** Lista de cues; GO con espacio; **botonera de sonidos** (cuadrícula o lista, disparo superpuesto; sección 5b); volumen; fade in/out; loop; inicio/fin sobre onda; parar/pausar/pánico; elección de tarjeta; modos con Tab e indicador; guardar/abrir show; preescucha; cues de audio, vídeo e imagen; *Proyector* con 4 puntos; miniatura; *Principal* + *Auriculares* y perfiles; pestañas, paletas, Vista básica (paneles, letra, Modo noche, Modo día); **Ayuda para principiantes**; interfaz en español e inglés. Además (añadido el 15/10/2026 tras comparar con QLab, SCS y MultiPlay): deshacer/rehacer en Edición; notas por cue (lo que ve el operador antes del GO); numeración libre (1, 1.5, 2A) para coincidir con el libreto; parar o fundir un solo cue desde el panel "Sonando"; ir a cue y empezar un cue desde un punto (ensayos); protección contra doble GO (tiempo mínimo entre GOs, ajustable); volumen maestro y medidor de nivel simple; "Reunir medios" (copiar todos los archivos a la carpeta del show); registro de la función (qué se disparó, cuándo y cualquier error); archivo del show con número de versión para abrir shows viejos en versiones nuevas. Justo después: pestaña Convertir.
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

### Paso 2: prototipo de audio (15/10/2026)
- `crates/audio` (`otaconvert-audio`, sin dependencias salvo `rtrb` 0.4.0, cola sin bloqueo): `Rampa`, `Pista` (audio entero en memoria, f32 intercalado), `Mezclador`/`Control` (`crear`), `Detector`, `prueba::Escenario`.
  - Comandos: Iniciar (volumen, desde, bucle, fundido), Parar, Pausar, Reanudar, Volumen, Saltar, PararTodo, VolumenMaestro. Además `CambiarAjustes`. Siete rampas separadas en `Ajustes` (ver tabla "Rampas cortas", sección 7); por defecto inicio de golpe. Salto = bajar, mover, subir.
  - El hilo de audio no pide ni libera memoria: los `Arc<Pista>` que dejan de sonar vuelven por una cola "basura" y se liberan en el hilo de control. Rechaza pistas a otra frecuencia (sin remuestreo en vivo).
  - Salida recortada a ±1 y contada como "recortes". **Pendiente: limitador maestro transparente.**
  - Detector: segunda diferencia `x[n]-2x[n-1]+x[n-2]` por canal; umbral para tonos `1,5·A·(2·sin(πf/fs))² + 0,002`.
- Pruebas (`cargo test --release`, 16): arranque de golpe por defecto, rampa de inicio opcional, cambio de ajustes en vivo, parada de golpe sin rampas, fundido del cue por encima de la rampa; estrés con `Ajustes::suaves` de 10 min × 3 semillas (~1.850 órdenes cada una, bloques de 1–1024 cuadros): 0 discontinuidades (máxima 0,0023, umbral 0,0127), 0 recortes, 0 rechazos. Control: sin rampas el detector encuentra 568 en 2 min. `sin_memoria`: asignador que cuenta en el hilo de `procesar` (comprobado que falla si se pide memoria).
- Motor: `--tono [s] [margen_ms]` y `--prueba-vivo [s] [margen_ms]` (salida predeterminada, modo compartido, `BufferSize::Fixed(margen)`, con `Ajustes::suaves` para que el detector no confunda un arranque de golpe con un corte; mide intervalos entre llamadas, cortes probables = intervalo > margen, discontinuidades, recortes, errores; termina con "RESULTADO: BIEN" o código de error). cpal con feature `realtime` (MMCSS).
- Verificado en el código de cpal 0.18.2 (`host/wasapi`): solo modo compartido; el periodo de llamada es siempre `GetDevicePeriod` (10 ms) y el `BufferSize::Fixed` solo agranda el búfer de WASAPI (más margen y más latencia); cada llamada llena todo el hueco libre. Solo F32 por ahora.
- Compilar en la PC del usuario: no tiene Visual Studio. Toolchain temporal GNU en `%TEMP%\otr` (sirve para `cargo test -p otaconvert-audio` y `cargo check`; el motor no enlaza con GNU porque falta `as.exe`). Ruta corta obligatoria: el scratchpad supera MAX_PATH. El ejecutable real sale de GitHub Actions (MSVC).
- **Prueba en vivo en la PC del usuario (15/10/2026)**, exe de Actions (SHA-256 `E907AE88…52B45E5`, verificado), salida "Altavoces (USB Audio CODEC)", 48 kHz, 2 canales, F32, WASAPI compartido:
  - 60 s, margen 50 ms: 189 órdenes, 6.054 llamadas de 10–50 ms, intervalo máximo 10,9 ms, 0 cortes probables, 0 discontinuidades (máx. 0,00159), 0 recortes, 0 errores. **BIEN.**
  - 30 s, margen 20 ms: 3.030 llamadas, WASAPI redondeó el búfer a 1.056 cuadros (22 ms), intervalo máximo 10,9 ms, todo en 0. **BIEN.**
  - Límite honesto: "cortes probables" mide huecos entre llamadas del motor; no ve un corte que ocurra dentro del mezclador de Windows o del driver. Para eso la Prueba de hardware grabará la salida (loopback) y la pasará por el detector.
- Pendiente: decidir el margen por defecto con la Prueba de hardware en 2–3 equipos (50 ms = más seguro, unos 50 ms de demora entre GO y sonido; 20 ms también fue bien en esta PC). Sin elegir aún. Elegir la salida: hecho (ver "Elegir la salida es FUNDAMENTAL", sección 7).

### Paso 3: prototipo de vídeo (15/10/2026, en curso)
- `crates/video` (`otaconvert-video`): 
  - `mpv`: libmpv cargada a mano (`LoadLibraryExW` con **ruta completa** junto al exe + `GetProcAddress`; dependencias de la DLL solo desde su carpeta y System32). No usa biblioteca de importación (compatible con `/DEPENDENTLOADFLAG:0x800`). Exige API 2.x (la fijada es 2.5). `Reproductor` con opciones seguras (`config=no`, `load-scripts=no`, `ytdl=no`, `osc=no`, sin teclado ni cursor de mpv, `terminal=no`, `aid=no`, `keep-open=always`, `vo=gpu-next`, `gpu-context=d3d11`, `hwdec=d3d11va`, fondo negro). `precargar` = `loadfile` en pausa y esperar `MPV_EVENT_PLAYBACK_RESTART` (primer cuadro listo).
  - `ventana`: ventana de proyección `WS_POPUP` negra, siempre encima, sin cursor, en el rectángulo de la pantalla elegida, con una ventana hija por capa (`--wid`), en su propio hilo con bucle de mensajes. `arriba(i)` cambia el orden con `SetWindowPos(HWND_TOP)`. `color_centro` lee el píxel central de la pantalla compuesta (GDI).
- Motor: `--prueba-video [cambios] [--pantalla <n | nombre | id>] [--carpeta <ruta>]`. Sin `--pantalla` usa la primera que no es principal (el proyector) o la principal si es la única. Sin `--carpeta`, colores lisos generados por mpv (`av://lavfi:color`); con carpeta, vídeos llamados `RRGGBB.*` (color del centro). Mide muestras negras, de otro color, cambios no vistos en 1 s, tiempo del GO a la imagen, cuadros perdidos y decodificación por capa.
- **Resultados en la PC del usuario** (GTX 1050 Ti, C24F390 1920×1080 a 60 Hz, libmpv 20261015):
  - 30 y 300 cambios con colores generados: 0 negros, 0 raros, 0 perdidos; GO → imagen 17,9 ms de media, 33,5 ms máx.
  - **300 cambios con vídeos H.264 1080p60 reales** (8 Mb/s, generados con FFmpeg de la misma fuente, solo herramienta de prueba): 5.104 muestras, **0 negros**, 0 raros, 0 cuadros perdidos, **decodificación d3d11va** en las dos capas, GO → imagen 18,4 ms de media, 25,7 ms máx.
  - Límite honesto: se mira la pantalla unas 57 veces por segundo (GetPixel espera a la composición de Windows), casi un muestreo por cuadro a 60 Hz; un negro de un solo cuadro probablemente se vería, pero no está garantizado. Falta: captura por cuadro (Windows.Graphics.Capture o DXGI) y probar con un proyector real en modo Extender.
- **Hallazgo:** `libmpv-2.dll` importa `vulkan-1.dll`. Los drivers de NVIDIA/AMD/Intel la instalan en System32 (está en la PC del usuario), pero un equipo sin ella no puede cargar libmpv. Opciones: incluir el cargador de Vulkan de Khronos (Apache-2.0) junto a libmpv, o que la Prueba de hardware y la comprobación previa lo detecten y avisen. Pendiente de decidir.
- **Hallazgo:** shinchiro solo conserva unas 30 compilaciones diarias: la versión fijada (`mpv-dev-x86_64-20261015-git-492bae0768.7z`, SHA-256 `5db58c17…183a358ba8`) desaparecerá en ~1 mes. Hace falta una copia propia (p. ej. una Release de dependencias en este repo): **necesita permiso del usuario** porque publica algo.
- Workflow V2: descarga libmpv fijada, comprueba la huella, la copia junto al exe, corre `--prueba-video 20` (informativa: el equipo de GitHub no tiene GPU) y sube exe + DLL.
- **Verificado el 15/10/2026** (run 38808284778, commit ed138fe): compila; huella de libmpv correcta; 24 pruebas en verde; `--comprobar` en GitHub: todo bien (Windows 26100, WebView2 154). Prueba de vídeo en el equipo virtual de GitHub (sin GPU, HyperVMonitor 1024×768, decodificación CPU): 0 negros, pero 20 muestras de "otro color" (una por cambio), 18 cuadros perdidos en una capa y GO → imagen 59 ms de media: es el dibujo por software; el informe ahora muestra ejemplos de esos colores para entenderlo. **El exe de GitHub (MSVC, SHA-256 `44DDC6B6…1B245A5278`, libmpv `154A1610…24541539`) probado en la PC del usuario:** `--comprobar` todo bien; 100 cambios con vídeos H.264 1080p60 reales: 0 negros, 0 raros, 0 perdidos, d3d11va, GO → imagen 18,0 ms media, 22,0 ms máx. **BIEN.**
- **4K probado en la PC del usuario** (15/10/2026, GTX 1050 Ti, pruebas cortas a pedido del usuario): 20 cambios con vídeos 4K60 H.264 (40 Mb/s) y 20 con 4K60 HEVC (30 Mb/s), escalados a su monitor 1080p: 0 negros, 0 raros, 0 perdidos, d3d11va en las dos capas, GO → imagen 18 ms. **Reproducción continua** (`--prueba-reproduccion [s] --archivo <vídeo>`, una capa en bucle; informa cuadros por segundo medidos, perdidos al mostrar y al decodificar, mostrados tarde): 60 s de 4K60 H.264 y 60 s de 4K60 HEVC: 60 cuadros/s, **0 perdidos, 0 tarde**, d3d11va. Temperatura de la GPU (nvidia-smi cada 5 s): 35 °C en reposo, **máximo 41 °C**, ventilador 48–49 % sin cambios, uso de GPU hasta 56 %, decodificador hasta 54 %. **No probado:** salida a una pantalla 4K real (el usuario no tiene).
- **PC de casa del usuario** (15/10/2026): GTX 1050 Ti y un monitor 1080p. Mencionó la gráfica solo para que las pruebas en su casa no le dañen la PC; **no es un requisito ni un límite del programa**. Pruebas en su equipo: razonables (minutos, no horas) y avisando antes.
- Pendientes del paso 3: proceso aparte para la proyección (hoy en el proceso del motor); `demuxer-lavf-o=protocol_whitelist` para limitar a archivos locales (probar que no rompe nada); el reloj de la tarjeta de sonido mandando sobre el vídeo; fundido cruzado; mantener último cuadro / fundir a negro al terminar.
- Compilar en la PC del usuario (actualizado): además del toolchain GNU, **llvm-mingw 20261006** (`%TEMP%\otr\lm`, SHA-256 verificado) + toolchain `stable-x86_64-pc-windows-gnullvm` permiten **compilar y ejecutar el motor localmente** (`CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER` = `x86_64-w64-mingw32-clang.exe`; copiar `libunwind.dll` de llvm-mingw y `libmpv-2.dll` junto al exe). Solo para probar; el exe que se entrega sale de GitHub Actions (MSVC).

### Paso 4: vigilante (15/10/2026)
- `crates/motor/src/vigilante.rs`. El vigilante arranca el motor como proceso hijo y lee su salida estándar: `LISTO` (la salida de audio ya suena) y `LATIDO <ms>` cada 100 ms con la posición del sonido. **Caída** = el proceso terminó (se ve con `try_wait`); **cuelgue** = 300 ms sin latido → se cierra el proceso. En ambos casos lo vuelve a arrancar con `--hijo-audio <ms>` para que el sonido siga donde iba (posición del último latido + tiempo transcurrido).
- `SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX)` en el hijo y en el vigilante: un fallo cierra el proceso al instante, sin la ventana de error de Windows que lo dejaría abierto (comprobado: caídas detectadas en 6–12 ms).
- Prueba `--prueba-vigilante [fallos] [--salida X]` (caídas con `abort` y cuelgues alternados, a los 1,5–3 s). **En la PC del usuario, 10 fallos:** caídas: detección 6–12 ms, sonando de nuevo en 58–73 ms, **peor caso 86 ms**; cuelgues: detección 320–342 ms, **peor caso 401 ms**; media 233 ms. **Objetivo < 1 s cumplido.** Sin procesos huérfanos al terminar.
- Límite honesto: el motor de prueba solo abre la salida y un tono. Un show real tiene que volver a cargar sus audios en memoria y precargar vídeos, lo que puede pasar de 1 s con shows grandes. Propuesta para cumplir siempre: **motor de respaldo en caliente** (un segundo proceso con el show ya cargado y la salida abierta en silencio, que toma el control al instante) y que el vigilante guarde el estado completo (cues sonando, posiciones, volúmenes), no solo una posición. Pendiente de implementar con la Fase 1.
- Pendiente: vigilar también el proceso de proyección (vídeo) igual que el de audio.

### Estado al pausar (15/10/2026, tarde) y próximo paso
- Fase 0 hecha y probada en la PC del usuario: audio (paso 2), elección de salida de audio y lista de pantallas, vídeo con precarga y capas incl. 4K (paso 3), vigilante (paso 4), comprobación de componentes. Todo subido a GitHub (último run verde: 38811070736, commit 6413400).
- Hallazgo del último run en GitHub (equipo virtual sin GPU): 19 muestras de "otro color" con ejemplos `(68,106,211)`, `(192,89,188)`, `(62,172,185)`: son colores de la prueba (4868D0, B848B8, 48B8B8) vistos cuando no tocaba, o sea que sin GPU **la capa de abajo a veces se ve por encima** mientras precarga. En la PC real (GPU) no pasa. Investigar al hacer el proceso de proyección aparte (posibles arreglos: ocultar la capa de abajo con `ShowWindow` hasta que esté lista, o `WS_EX_NOREDIRECTIONBITMAP`/orden Z explícito).
- **Prueba de hardware (paso 5 de la Fase 0): HECHA**, ver "Paso 5" abajo. Diseño original:
  1. Doble clic en `Prueba de hardware.exe` (el mismo motor renombrado; si el nombre empieza por "Prueba" y no hay argumentos, corre la prueba). Ventana de consola en español: qué va a pasar (colores en pantalla, tonos), que no hay que tocar nada, duración ~5 min; al final "Pulsa Enter para cerrar" (`--sin-pausa` para GitHub).
  2. Orquestador que ejecuta cada prueba como proceso hijo del propio exe (aísla caídas), muestra la salida y la copia a `Informe-hardware-<equipo>-<fecha>.txt` junto al exe (o en Documentos si no se puede escribir).
  3. Orden: datos del equipo (`--info-equipo`: Windows, CPU por registro, RAM con GlobalMemoryStatusEx, GPUs) → `--comprobar` → `--listar-salidas` → `--prueba-vivo` 10 s por cada salida (margen 50 ms) y 10 s a 20 ms en la predeterminada → `--listar-pantallas` → `--prueba-video` 100 cambios de color en la pantalla del proyector → 50 cambios con vídeos 1080p → `--prueba-reproduccion` 30 s 1080p60, 30 s 4K H.264, 30 s 4K HEVC → `--prueba-vigilante 6` → resumen BIEN/PROBLEMA por línea y veredicto.
  4. Paquete en GitHub Actions: carpeta `Otaconvert-Prueba-de-hardware/` con el exe renombrado, `libmpv-2.dll`, `LEEME.txt` (pasos simples) y `pruebas/` con vídeos generados en CI con FFmpeg fijado por SHA-256 (6 colores 1080p60 H.264, 1 clip 4K60 H.264, 1 clip 4K60 HEVC).
  5. Para que el usuario lo use en 2–3 teatros conviene publicarlo como Release descargable sin cuenta de GitHub: **necesita su permiso** (publica algo).
  - Fuera de alcance por ahora (anotar en el informe como "no medido"): latencia real de salida (requiere grabar la salida), cambio 44,1/48 kHz con medición de deriva (cambiar el formato del dispositivo no tiene API pública), chasquidos dentro del driver (requiere loopback).
- Decisiones pendientes del usuario: copia propia de libmpv en su GitHub; incluir `vulkan-1.dll`; instalador de WebView2 (peso); publicar la Prueba de hardware como Release; dependabot y protección de rama `main`; tocar `main.yml` de la V1.

### Paso 5: Prueba de hardware (15/10/2026, noche)
- `crates/motor/src/prueba_hardware.rs` (orquestador) + `equipo.rs` (`--info-equipo`: nombre del equipo, Windows por registro, procesador por registro y hilos, memoria con `GlobalMemoryStatusEx`, tarjetas gráficas con `EnumDisplayDevicesW`, enchufado/batería con `GetSystemPowerStatus`).
- **Doble clic**: si el exe se llama `Prueba de hardware.exe` (nombre que empieza por "prueba") y no tiene argumentos, corre `--prueba-hardware`. Explica en español qué va a pasar, cuenta 5 s y empieza. Al final: resumen, ruta del informe y "Pulsa Enter para cerrar" (`--sin-pausa` para GitHub).
- Cada prueba es un **proceso hijo** del propio exe con límite de tiempo (si se cuelga, se cierra y cuenta como PROBLEMA). La salida se muestra y se copia a `Informe-hardware-<EQUIPO>-<AAAA-MM-DD_HH-MM>.txt` (UTF-8 con BOM) junto al exe, o en Documentos si ahí no se puede escribir.
- Orden (13 pasos en la PC del usuario): datos del equipo · componentes · salidas de audio · sonido 10 s por **cada** salida (margen 50 ms) · sonido 10 s con margen 20 ms en la predeterminada · pantallas · 100 cambios de color · 50 cambios con vídeos 1080p reales · reproducción 30 s de 1080p60, 4K60 H.264 y 4K60 HEVC · vigilante con 6 fallos. Resumen con BIEN / PROBLEMA / DATO / NO MEDIDO y veredicto. "NO MEDIDO" fijo: latencia real, cambio 44,1/48 kHz con deriva, cortes dentro del driver; y lo que falte (sin salidas de audio, sin vídeos de prueba).
- Vídeos de prueba en `pruebas/` junto al exe: `1080p/RRGGBB.mp4` (6 colores, 1080p60 H.264, 6 s), `4k-h264.mp4`, `4k-hevc.mp4` (4K60, 6 s). En GitHub se generan con FFmpeg de shinchiro fijado por SHA-256 (`ffmpeg-x86_64-git-ee8cbd7db.7z`, `dc123b3e…7e475ab`; mismo problema de que las compilaciones diarias desaparecen).
- **Paquete**: el workflow V2 arma `Otaconvert-Prueba-de-hardware/` (exe renombrado, `libmpv-2.dll`, `LEEME.txt` de `v2/prueba-hardware/`, `pruebas/`), corre la prueba completa como informativa (sin GPU ni audio en GitHub) y lo sube como artifact `Otaconvert-Prueba-de-hardware`. Descargar artifacts exige cuenta de GitHub: para otros teatros conviene una Release (**necesita permiso**).
- **Resultado en la PC del usuario** (paquete armado con el exe local, como doble clic): 188 s; equipo DESKTOP-T79LDS5, Windows 10 Pro 22H2 (19045), i5-12400 (12 hilos), 15,8 GB, **dos tarjetas gráficas** (Intel UHD 730 integrada + GTX 1050 Ti), enchufado; todo **BIEN** (salidas USB y HDMI, 100 colores, 50 vídeos 1080p, 1080p60/4K H.264/4K HEVC 30 s, vigilante: caída 78 ms y cuelgue 410 ms como peor caso). Veredicto: pasó todo lo medible.
- **Paquete de GitHub probado** (run 38823033023, commit e799093; artifact `Otaconvert-Prueba-de-hardware` de 78 MB; `Prueba de hardware.exe` SHA-256 `46119FF4…F07084FA`): en GitHub (sin GPU ni audio) el orquestador y el informe funcionan; componentes BIEN, las 5 pruebas de vídeo PROBLEMA y sin salidas de audio, como se esperaba. **En la PC del usuario:** 188 s, las 13 pruebas BIEN, veredicto "pasó todas las pruebas que se pudieron medir".
- Pendiente: correrlo en 2–3 equipos o teatros más; con un proyector en modo Extender; detectar qué GPU maneja cada salida en equipos con dos GPU (hoy se lista la GPU de cada pantalla).

### Decisiones del 15/10/2026 (noche) y lo hecho
- **Copia propia de dependencias** (aprobada): Release pública `dependencias-2026-10-15` (prerelease, "no es una versión del programa") en https://github.com/marcoldmusica-cyber/otaconvert/releases/tag/dependencias-2026-10-15 con `mpv-dev-x86_64-20261015-git-492bae0768.7z` (`5db58c17…183a358ba8`), `ffmpeg-x86_64-git-ee8cbd7db.7z` (`dc123b3e…7e475ab`), `vulkan-runtime-components-1.4.363.0.zip` (`a25a927a…2695769a`) y `SHA256SUMS.txt`; notas con origen, commits de mpv/FFmpeg, licencias y enlaces al código fuente (obligación GPL). Verificado: descargados de nuevo desde la Release, las tres huellas coinciden. El workflow V2 ya descarga de ahí (no de shinchiro ni LunarG).
- **Vulkan incluido** (aprobado): `vulkan-1.dll` x64 1.4.363.0 del Vulkan Runtime de LunarG (MIT + Apache 2.0, DLL firmada por LunarG, Inc.; enlace fijo `https://sdk.lunarg.com/sdk/download/1.4.363.0/windows/vulkan-runtime-components.zip`). Va junto a libmpv con su licencia (`VulkanRT-Licencia.txt`), en el artifact del motor y en el paquete de la Prueba de hardware. Probado en la PC del usuario: `--comprobar` → "incluida con el programa"; `--prueba-video 10` BIEN (libmpv carga la copia de su carpeta por `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR`). Khronos no publica la DLL compilada (Vulkan-Loader no tiene binarios en sus releases).
- **WebView2 en instalador y portable** (decisión del usuario): ver "Componentes faltantes", sección 7.
- **Prueba de hardware publicada** (aprobado): Release pública `prueba-hardware-0.0.1` (prerelease, "versión de prueba"): https://github.com/marcoldmusica-cyber/otaconvert/releases/tag/prueba-hardware-0.0.1 con `Otaconvert-Prueba-de-hardware-0.0.1.zip` (74,5 MB, SHA-256 `8B002E38…AD17DED3`, verificado descargándolo de nuevo) y su `.sha256`; compilado por el run 38837436754 (commit 6e72b79): exe `06C469D4…72C4F09F`, libmpv `154A1610…24541539`, `vulkan-1.dll` `E1FCFC94…10F314A3`. Antes de publicar, ese mismo paquete pasó las 13 pruebas en la PC del usuario. Las notas explican los pasos y el aviso de SmartScreen.
- Quedan pendientes de su permiso: Dependabot, protección de `main`, `main.yml` de la V1.

### Que Windows no lo trate como algo peligroso (pedido del usuario, 15/10/2026 noche)
- El aviso azul de **SmartScreen** ("Windows protegió su PC") sale porque el exe **no tiene firma digital** y todavía no tiene "reputación"; no significa que Windows lo haya detectado como malware.
- **Opciones de firma (investigado el 15/10/2026):**
  1. **SignPath Foundation** (https://signpath.org/terms): **gratis para código abierto**. Requisitos: licencia OSI sin doble licencia comercial (GPLv3: sí), repositorio público, descarga gratuita, proyecto mantenido y ya publicado, compilación automática desde el repositorio (GitHub Actions: sí) y una **política de firma** publicada en la página del proyecto. El certificado sale a nombre de **SignPath Foundation**, no del usuario. Hay que solicitarlo (lo hace el usuario con sus datos).
  2. **Microsoft Artifact Signing** (antes Trusted Signing): USD 9,99/mes (Basic). Según la guía de Microsoft, personas individuales solo en **EE. UU. y Canadá** (organizaciones también UE y Reino Unido): probablemente no aplica al usuario.
  3. Certificado comercial OV/EV de una autoridad (pago anual, validación de identidad). No investigado en detalle.
- **Recomendación:** pedir **SignPath Foundation** cuando haya una versión del programa publicada (requisito "ya publicado"; la Prueba de hardware podría servir). Mientras tanto, medidas gratis sin firma: (a) **datos de versión** en el exe (nombre, editor, versión, descripción, icono) y manifiesto; (b) nada que se parezca a malware: sin compresores tipo UPX, sin descargar ni ejecutar código; (c) **enviar cada versión a Microsoft** para análisis de falso positivo si Defender la marcara (portal de Microsoft Security Intelligence; lo envía el usuario o con su permiso); (d) publicar siempre en el mismo lugar con huella SHA-256.
- Ni siquiera con firma desaparece SmartScreen al instante: la reputación se gana con descargas. Lo que sí cambia es que el aviso muestra un editor verificado. (Afirmación general, sin comprobar con documentación de Microsoft.)
- Elegida por el usuario (15/10/2026): **SignPath Foundation**.

### Solicitud de firma a SignPath Foundation (preparada el 15/10/2026, noche)
- **Requisitos oficiales** (https://signpath.org/terms, leídos el 15/10/2026): licencia OSI sin doble licencia comercial en **todos** los componentes; sin código propietario; proyecto mantenido y **ya publicado** en la forma a firmar; funcionalidad descrita en la página de descarga; el equipo firma solo sus propios binarios (DLL de terceros sin firmar se pueden incluir); **MFA obligatorio** en GitHub y SignPath para todo el equipo; roles autores/revisores/aprobadores; cada firma aprobada a mano; compilación verificable desde el repositorio (CI cuenta como código); metadatos del archivo: **nombre del producto = nombre del proyecto** y versión igual en cada compilación; en la página principal y en la de descarga, una sección o enlace **"Code signing policy"** con la frase "Free code signing provided by SignPath.io, certificate by SignPath Foundation", la lista de roles y un enlace de privacidad o la frase de que no transfiere datos; desinstalación documentada.
- **Hecho para cumplirlos:** licencia **GPLv3 para todo el repositorio** (`LICENSE` en la raíz; decisión del usuario: también la V1); `README.md` raíz reescrito con V1 + V2, descargas, licencia, **Code signing policy** (firma solicitada, qué se firma, roles: committers/reviewers y approvers = marcoldmusica-cyber, MFA), **Privacy policy** (frase en inglés exigida + explicación) y **Desinstalar**; notas de la Release `prueba-hardware-0.0.1` con sección "Code signing policy"; descripción del repositorio en GitHub; **datos de versión, icono y manifiesto en el exe** con `winresource` 0.1.34 (`v2/crates/motor/build.rs`, `otaconvert.manifest`: `asInvoker`, Windows 10/11, DPI por monitor V2, rutas largas): ProductName "Otaconvert", versión del `Cargo.toml`; el workflow falla si no coinciden. Comprobado en la PC del usuario (Windows muestra los datos; `--comprobar` y `--prueba-video 10` BIEN). Para compilar local con llvm-mingw: variables `WINDRES` (`x86_64-w64-mingw32-windres.exe`) y `AR_PARA_RECURSOS` (`llvm-ar.exe`).
- **Formulario** (https://signpath.org/apply; campos leídos sin rellenar). Respuestas propuestas:
  | Campo | Respuesta |
  |---|---|
  | Project Name | Otaconvert |
  | Repository URL | https://github.com/marcoldmusica-cyber/otaconvert |
  | Homepage URL | https://github.com/marcoldmusica-cyber/otaconvert |
  | Download URL | https://github.com/marcoldmusica-cyber/otaconvert/releases |
  | Privacy Policy URL | https://github.com/marcoldmusica-cyber/otaconvert#privacy-policy |
  | Wikipedia URL | (vacío) |
  | Tagline | Free and open-source cue player and audio converter for theater technicians on Windows. |
  | Category | la más cercana a audio/vídeo/multimedia (lista desplegable; la elige el usuario) |
  | Description | Otaconvert is a free, GPLv3 Windows toolkit for theater sound and video technicians. Version 1 is a batch audio converter. Version 2, in development, is a cue player for live shows (audio, video up to 4K and projection mapping) built in Rust, with glitch-free audio, no black frames between videos, and a hardware test tool that checks whether a computer is ready for a performance. It works fully offline and collects no data. |
  | Reputation | New project: first public release in October 2026 (hardware test tool 0.0.1). Developed for theater technicians; builds are automated with GitHub Actions and every download is published with its SHA-256 hash. |
  | Maintainer Type | Individual (persona) |
  | Build System | GitHub Actions |
  | First/Last name, Email | los del usuario |
  | Company Name | (vacío) |
  | Primary Discovery Channel | lo elige el usuario |
  | Casillas | Code of Conduct (obligatoria, leerla antes) y almacenamiento de datos personales (obligatoria); la de comunicaciones es opcional |
- **Lo que hace el usuario:** (1) activar la verificación en dos pasos en GitHub (Settings → Password and authentication → Enable two-factor authentication); (2) leer el Code of Conduct de SignPath Foundation; (3) completar y enviar el formulario con estas respuestas (no lo envía Claude: datos personales y aceptación de términos); (4) esperar la respuesta por correo; al recibir la cuenta, activar también la verificación en dos pasos en SignPath.
- **Enviada el 16/10/2026:** el usuario activó la verificación en dos pasos en GitHub y envió el formulario de SignPath con las respuestas de la tabla. Esperando el correo de respuesta. Pendiente del usuario: guardar sus recovery codes de GitHub en lugar seguro y regenerarlos (los pegó en el chat; no se guardaron en la memoria de Claude por seguridad).
- **Arreglo (run 38838958923 falló):** con MSVC, meter el manifiesto como recurso con `rc.exe` (winresource) hizo que Windows rechazara los ejecutables ("side-by-side configuration is incorrect", error 14001). Ahora, con MSVC, `build.rs` pasa al enlazador `/MANIFEST:EMBED /MANIFESTINPUT:otaconvert.manifest /MANIFESTUAC:NO` (vía documentada por Microsoft); con llvm-mingw sigue como recurso. Run 38839312766 (commit ab43ea3): verde, datos de versión comprobados. GitHub detecta la licencia **GPL-3.0** y la portada muestra "Code signing policy" y "Privacy policy".
- **Después de la aprobación (lo hace Claude con el usuario):** conectar el workflow con la acción oficial de SignPath para enviar el exe a firmar, configurar la aprobación manual de cada firma y publicar las versiones firmadas. Firmar `Prueba de hardware.exe` y, en la Fase 1, el instalador y la portable.








## 11. Fase 1: plan aprobado (16/10/2026)

El usuario aprobó hacer la Fase 1 en **6 etapas**, cada una termina en algo que se puede probar:
1. **Base del show:** archivo del show (JSON con número de versión, rutas relativas seguras dentro de la carpeta), guardado automático con copias, registro de la función; motor que abre un show; canal seguro ventana ↔ motor (127.0.0.1 + token, regla de seguridad 1).
2. **Cues de audio reales:** preparar cualquier audio en memoria a la frecuencia de la tarjeta (en segundo plano, con picos), GO / Pausa / Parar todo / Fundir todo / Mayús+F, volumen, fundidos, bucle, inicio y fin, limitador maestro, protección contra doble GO, salidas *Principal* y *Auriculares* (preescucha).
3. **Interfaz:** la ventana del boceto (lista, GO, Edición/Función con Tab, barra de menús, Modo Ayuda y consejos), español e inglés desde el principio.
4. **Vídeo e imagen:** proyección en proceso aparte con vigilante, precarga y capas, imágenes, superficie de 4 puntos, miniatura.
5. **Edición completa:** deshacer/rehacer, notas, numeración libre, ir a cue / empezar desde un punto, onda, Reunir medios, Ajustes del show (rampas con consejos), perfiles de teatro, comprobación previa (luz verde), **botonera**, panel Sonando, medidor, Modo noche/día.
6. **Entrega:** instalador y portable (Vulkan, WebView2, firma de SignPath si la aprueban), manual al día, piloto en un teatro.
Después: pestaña Convertir.
Antes de programar se verifican tres decisiones técnicas: cómo se dibuja la ventana (WebView2 desde Rust), cómo se leen todos los formatos de audio y el formato del archivo del show.

### Decisiones técnicas verificadas (16/10/2026)
| Decisión | Elección | Verificado |
|---|---|---|
| Ventana | **wry 0.57.0** (Apache-2.0 OR MIT, MSRV 1.85) + **tao 0.37.1** (Apache-2.0): WebView2 en Windows (README de wry). La ventana carga la interfaz que sirve el motor en `http://127.0.0.1:<puerto>`; sin WebView2, la misma interfaz se abre en el navegador. | README de wry; crates.io |
| Leer audios | **Symphonia 0.6.1** (MPL-2.0, compatible con GPLv3: sin aviso de "Incompatible With Secondary Licenses") para decodificar y **rubato 5.0.1** (MIT OR Apache-2.0, MSRV **1.87** → subir el MSRV del workspace) para adaptar la frecuencia al prepararlos. Formatos: MP3, MP1/2, AAC-LC, ALAC, FLAC, Vorbis, Opus, ADPCM, PCM/WAV, WavPack; contenedores WAV, OGG, MKV/WebM, AIFF, CAF, MP4/M4A (los no predeterminados se activan con features). **No: WMA, AC3, DTS, AMR**: para esos, FFmpeg de la pestaña Convertir (a decidir en la etapa 2). Según su README, rinde ±15 % de FFmpeg. | README de Symphonia; LICENSE |
| Archivo del show | **JSON** con `serde` + `serde_json` 1.0.154 (MIT OR Apache-2.0), con `version_formato`. | crates.io |
| Canal ventana ↔ motor | HTTP + WebSocket mínimos con **tungstenite 0.30.0** (MIT OR Apache-2.0) sobre `std::net::TcpListener` en 127.0.0.1, token por sesión, comprobación de `Host`/`Origin`. `tiny_http` descartado (sin cambios desde 2022). | crates.io |

### Etapa 1, parte A: archivo del show (16/10/2026)
- `crates/show` (`otaconvert-show`, deps: serde, serde_json): `modelo` (Show, Cue, TipoCue audio/vídeo/imagen, AjustesShow con rampas en ms —inicio 0 por defecto—, tiempo mínimo entre GO 300 ms, Fundir todo 3000 ms, volumen maestro; botonera con PaginaBotonera/BotonSonido y `al_repetir` parar/reiniciar/superponer), `archivo` (abrir/guardar/validar), `rutas` (validar/resolver), `registro` (registro de la función).
- Archivo `<nombre>.otashow` (JSON con `version_formato` = 1) en la carpeta del show. Campos que faltan → valores por defecto. Formato más nuevo → aviso "Actualiza Otaconvert" (se lee primero solo la versión). Dañado → error con línea y columna, nunca se cierra el programa. Límites: 20 MB, 10.000 cues, volúmenes ≤ +24 dB y finitos, fin > inicio, rampas 0–10.000 ms. Acepta la marca BOM del Bloc de notas.
- Guardar: valida, copia el anterior a `.otaconvert/copias/<nombre>-AAAAMMDD-HHMMSS.otashow` (se conservan **20**), escribe a `.otashow.tmp`, `sync_all` y renombra (reemplazo atómico en Windows).
- Rutas de medios: relativas con `/`; se rechazan `..`, absolutas, unidades (`C:`), red (`\servidor`, `//servidor`), flujos alternativos (`a.wav:x`), caracteres `<>"|?*` y de control, nombres reservados (CON, NUL, COM1…), nombres que terminan en punto o espacio. `resolver` además comprueba con la ruta canónica que un enlace o **unión de carpetas** no saque el archivo de la carpeta.
- Registro: `registros/funcion-AAAA-MM-DD.txt`, una línea por evento con hora y milisegundos; quita saltos de línea; si no se puede escribir, sigue sin registro (nunca frena la función).
- **14 pruebas, todas en verde** en la PC del usuario (ida y vuelta, campos que faltan, versión más nueva, dañado con línea, rutas peligrosas en cues y botonera, valores imposibles, archivo de 20 MB+1, BOM, 26 guardados → 20 copias y la última es la anterior, resolver, **unión que apunta afuera rechazada** (creada con `mklink /J`, sin administrador), registro).
- Siguiente (parte B): canal seguro ventana ↔ motor, motor que abre un show y guardado automático.

### Etapa 1, parte B: canal seguro, motor que abre un show y guardado automático (16/10/2026)
- `crates/servidor` (`otaconvert-servidor`; deps: getrandom 0.4.3 (MIT OR Apache-2.0, en Windows usa `ProcessPrng`), tungstenite 0.30.0, serde_json, otaconvert-show):
  - `Servidor::iniciar(Arc<Mutex<Sesion>>)`: escucha **solo en 127.0.0.1**, puerto al azar; **token** de 32 bytes al azar (hex), comparación sin atajos; `url()` = `http://127.0.0.1:<puerto>/?t=<token>`.
  - HTTP: solo `GET /?t=<token>` con **Host** `127.0.0.1:<puerto>` (si no: 403; otro método: 405; otra ruta: 404; cabeceras > 8 KB: 431). Respuesta con CSP (`default-src 'none'`, `connect-src ws://127.0.0.1:<puerto>`, `frame-ancestors 'none'`…), `Referrer-Policy: no-referrer` (el token no se filtra), `X-Frame-Options: DENY`, `nosniff`, `no-store`, sin CORS.
  - WebSocket `/ws?t=<token>`: exige Host y **Origin** `http://127.0.0.1:<puerto>` (rechaza otras páginas web, `localhost` y pedidos sin Origin), mensajes ≤ 64 KB (uno mayor cierra solo esa conexión). Órdenes JSON: `go`, `parar_todo`, `espera_arriba`, `espera_abajo`, `guardar`, `estado`; cualquier otra → aviso "Orden desconocida". Cada cambio se envía a **todas** las ventanas conectadas. Máximo 16 conexiones; 5 s para mandar el pedido.
  - `Sesion`: abre el show (archivo o la carpeta con un solo `.otashow`), cue en espera, último disparado, **protección contra doble GO** (`tiempo_minimo_entre_go_ms`, 300 ms por defecto; el ignorado se anota), registro de la función (ABRIR, GO, IGNORADO, PARAR, GUARDAR, ERROR), guardado y **guardado automático cada 30 s solo si hay cambios**.
  - Interfaz mínima `crates/servidor/ui/index.html` (incluida en el exe): lista con número, nombre, nota, tipo; cue en espera resaltado; último disparado en verde; GO (Espacio), Parar todo (Esc), ↑↓, Ctrl+S; aviso; indicador de conexión que se reconecta solo. Textos del show siempre con `textContent` (nunca como HTML).
- Motor: `--show-ejemplo <carpeta>` (crea `Ejemplo.otashow` con 6 cues y una página de botonera; no sobrescribe) y `--abrir-show <carpeta | .otashow> [--sin-navegador]` (abre el navegador predeterminado con `ShellExecuteW`; la ventana propia con WebView2 es la etapa 3).
- **Pruebas** (`crates/servidor/tests/canal.rs`, 7, todas en verde): página solo con token y Host correctos (403 sin token, token corto o equivocado, Host `malicioso.com` o `localhost`; 404; 405; cabeceras de seguridad; sin CORS); cabeceras de 10 KB → 431; WebSocket rechaza otro origen, `localhost`, sin origen y token malo; GO avanza, doble GO ignorado con aviso, ↑, orden basura, registro con 2 GO y 1 IGNORADO; dos ventanas ven el mismo cambio; mensaje de 200 KB cierra esa conexión y el motor sigue; guardado automático solo con cambios.
- **Probado en la PC del usuario** (panel del navegador): el show de ejemplo se ve con la estética del programa, "Conectado al motor"; GO, GO y un tercer GO enseguida → los dos primeros avanzan (el disparado en verde) y el tercero se ignora con su aviso; registro: ABRIR, GO, GO, IGNORADO con milisegundos.
- Pendiente para más adelante: que el vigilante mantenga puerto y token al reiniciar el motor (hoy un motor nuevo tendría otra dirección); proceso de la interfaz separado del de audio.
- **Etapa 1 completa.** Siguiente: etapa 2 (cues de audio reales).

### Etapa 2, parte A: preparar audios (16/10/2026)
- `crates/audio/src/preparar.rs`: `preparar(ruta, frecuencia_salida) -> Preparado { pista, pico, frecuencia_original, canales_originales, paquetes_danados }`. Lee con **Symphonia 0.6.1** (feature `all`), pasa a float 32 y, si hace falta, a la frecuencia de la tarjeta con **rubato 5.0.1** (`Fft`, `FixedSync::Both`, `process_all`, que recorta el retardo inicial). Más de 2 canales → estéreo (izq. = 1º + 0,707·3º, der. = 2º + 0,707·3º). Límites: 60 min por audio, 50 paquetes dañados como máximo (se saltan y se cuentan). Errores en español, nunca cierra el programa.
- **Corrección:** Symphonia **0.6.1 no trae Opus ni WavPack** (el README del repositorio describe la versión en desarrollo). No se leen: **Opus, WMA, AC3, DTS, AMR, WavPack** → el error pide convertirlo en la pestaña Convertir (FFmpeg); decidir si el motor usa FFmpeg como respaldo.
- MSRV del workspace subido a **1.87** (lo pide rubato 5). `Pista::muestras()` público.
- Pruebas `crates/audio/tests/preparar.rs` (5, en verde) con archivos de `tests/datos/` creados con FFmpeg (tono 1 kHz, pico 0,5, 1 s, 44,1 kHz estéreo; ~700 KB en total): WAV, FLAC, AIFF, MP3, OGG/Vorbis y M4A/AAC → 48 kHz con duración (±1 ms sin pérdida, ±60 ms con pérdida), pico 0,5 ± 0,05, frecuencia 1000 ± 10 Hz y sin discontinuidades en la parte central; mono a 48 kHz queda igual (24.000 cuadros); 5.1 → estéreo; Opus → "no soportado… Convertir"; inexistente, basura y WAV cortado no cierran nada. Aprendido: el generador `sine` de FFmpeg usa amplitud 1/8 por defecto; los datos se crearon con `aevalsrc` a 0,5 exacto.

### Etapa 2, parte B: mezclador completo (16/10/2026)
- **Limitador maestro** (`crates/audio/src/limitador.rs`), activo por defecto (`Ajustes::limitador`): mira 1,5 ms adelante (retrasa la salida 72 cuadros a 48 kHz); para cada cuadro calcula la ganancia que necesita para no pasar el **techo de −0,1 dBFS** (0,988553), toma el mínimo de la ventana [t−L, t] (incluye el cuadro que sale) y promedia esos mínimos en L cuadros → **nunca pasa del techo** y la bajada es una rampa de L cuadros (≤ 1/L por cuadro); la subida vuelve con constante de 100 ms. Sin picos, ganancia exactamente 1: salida **idéntica** a la entrada (con el retraso). Todo reservado al crearlo. Corrección hecha antes de probar: la ventana necesitaba L+1 cuadros para incluir el que sale. Estadística `cuadros_limitados`. Después del limitador sigue el recorte de seguridad a ±1 (cuenta `recortes`, que debería quedar en 0).
- **`Comando::IniciarCue`** { inicio, desde, hasta, bucle, fundido_entrada, fundido_salida }: el cue termina en `hasta` (o al final del archivo) bajando en el mayor de la rampa de fin y el **fundido de salida** del cue; el **bucle vuelve a `inicio`** (no al principio del archivo); empezar desde un punto (`desde` ≥ inicio). Se rechazan puntos imposibles (inicio ≥ fin, desde fuera de [inicio, fin)). `Iniciar` sigue igual (inicio 0, sin fin).
- **Pausa general**: `PausarTodo` / `ReanudarTodo` (usan las rampas de pausa y de reanudar).
- Pruebas: limitador 3 (transparente idéntico, nunca pasa del techo con golpes de hasta +12 dB y vuelve a su nivel, bajada ≤ 1/L por cuadro); mezclador 5 nuevas (limitador en el mezclador con 4 voces que suman 2,0: máximo ≤ techo, 0 recortes; fin con fundido de salida —0,25 a mitad, silencio después—; bucle vuelve a su inicio; pausa general; 4 puntos imposibles rechazados y su memoria devuelta). El estrés de 30 min y `sin_memoria` corren **con el limitador encendido**: 0 discontinuidades y 0 pedidos de memoria. Las pruebas viejas que miran la muestra exacta usan `limitador: false`. **61 pruebas en verde.**

### Etapa 2, parte C: el GO suena de verdad (16/10/2026)
- `crates/servidor/src/sonido.rs`: trait `Sonido` (frecuencia, tiene_auriculares, enviar(Destino, Comando), recoger_basura) y `Destino` Principal/Auriculares. Así la sesión se prueba con un sonido falso y el motor usa cpal.
- `Sesion` (servidor) ahora: `conectar_sonido` (pasa al mezclador las rampas del show en cuadros, el limitador activo y el volumen maestro); **preparación anticipada** del cue en espera y los 2 siguientes de audio (`PREPARAR_ADELANTE` = 3) en un hilo "preparador" que no bloquea la sesión mientras lee el disco; suelta de memoria lo que ya no hace falta (salvo lo que suena); archivo que falta o no se puede leer → estado "error" con el motivo. **GO**: audio listo → `IniciarCue` (volumen dB→lineal, inicio/fin/fundidos en cuadros, bucle) a la salida del cue; **sin preparar → no dispara y avisa** ("todavía se está preparando"); **con error → avanza y avisa** (un cue roto no traba la función); vídeo e imagen → avanza con aviso (etapa 4). Cue para *Auriculares* sin auriculares configurados → suena por *Principal* y avisa. **Pausa (P)** alterna PausarTodo/ReanudarTodo en las dos salidas; **Fundir todo (F)** en `fundir_todo_ms` (3 s); **Mayús+F** funde solo el último disparado; **Parar todo (Esc)**. Lista "sonando" con tiempo restante (descontando pausas) que se limpia sola. Registro: SONIDO, GO, IGNORADO, PAUSA, SIGUE, FUNDIR, PARAR, AVISO (pico > 0 dB), ERROR.
- Servidor: órdenes `pausa`, `fundir_todo`, `fundir_ultimo`; hilos "preparador" y "refresco" (cada 250 ms manda el estado a las ventanas si cambió). Interfaz mínima: estado por cue (Listo / Preparando… / No se puede reproducir, con el motivo al pasar el ratón), línea "sonando" con cuenta atrás o "en bucle", "En pausa · P para seguir", teclas P, F, Mayús+F.
- Motor: `--abrir-show <show> [--salida X] [--auriculares Y] [--margen ms]` (margen 50 ms por defecto). `sonido_cpal.rs`: un mezclador por salida con `Ajustes::para` (limitador activo); los streams viven en el hilo principal. Auriculares a otra frecuencia que Principal → queda sin usar con aviso (por ahora deben coincidir). **Si una tarjeta da errores** (p. ej. se desconecta) → aviso fuerte en consola y en la ventana y se anota; nunca se cambia en silencio de salida.
- Pruebas: `crates/servidor/tests/sonido.rs` (7): GO sin preparar no dispara; GO manda el cue completo (−6 dB ≈ 0,501, inicio 100 ms = 4.800 cuadros, fin 900 ms, fundidos 1,5 s y 250 ms, bucle, a 48 kHz) y prepara los 3 siguientes; pausa/seguir/Mayús+F/F/Esc mandan exactamente las órdenes esperadas a cada salida; archivo que falta → error y la función sigue; auriculares sin configurar → Principal con aviso y, configurados, Auriculares; suelta de memoria los cues que ya pasaron; lo que suena se ve y termina solo. Las pruebas del canal ahora esperan el estado que corresponde (llegan refrescos periódicos). **63 pruebas en verde** en la PC del usuario.
- **Prueba en vivo en la PC del usuario** (show de ejemplo con audios generados: sala.mp3 en bucle, viento.wav, campanas.flac, final.wav, a volumen suave; salida "Altavoces (USB Audio CODEC)", 48 kHz, margen 50 ms): los audios quedaron "Listo"; GO sala, GO vídeo (saltado), GO viento, P, P, Mayús+F (solo el viento), GO campanas, F; el registro anota todo en orden. **Confirmado de oído por el usuario (16/10/2026, segunda prueba con él delante): sin clics ni cortes.** (La primera vez no estaba en el ordenador; se repitió.)
- **Etapa 2 completa.** Siguiente: etapa 3 (interfaz).

### Etapa 3, parte A: ventana propia (16/10/2026)
- Plan de la etapa 3: **3A** ventana propia con WebView2 (vuelta al navegador); **3B** la interfaz del boceto conectada al motor (barra de menús, Edición/Función con Tab, Pausa y Fundir todo como botones, panel Sonando); **3C** Modo Ayuda, consejos emergentes y español/inglés.
- **`otaconvert-ventana.exe`** (`crates/ventana`, **wry 0.57.0 + tao 0.37.1**, sin consola en la versión final, mismos datos de versión, icono y manifiesto que el motor) + `crates/motor/src/ventana_ui.rs` (el motor la busca junto a su exe, la abre y la vigila). Es un **exe aparte** a propósito: si se cae, el sonido sigue, y el cargador de WebView2 no se mezcla con libmpv (ver licencia abajo). La ventana es un **proceso aparte**: el motor le pasa la dirección con el token por la **entrada estándar** (no por la línea de comandos, que otros programas pueden leer) y la vigila cada 500 ms:
  - termina con 0 (el usuario la cerró) → el motor guarda si hay cambios, anota CERRAR y termina;
  - termina con 3 (`SIN_WEBVIEW2`: no se pudo crear WebView2) → aviso en consola, ventana y registro, y se abre el navegador;
  - otra cosa (caída) → se vuelve a abrir; más de 3 caídas en 60 s → navegador con aviso. El sonido nunca se toca.
- Seguridad de la ventana: solo acepta `http://127.0.0.1:<puerto>/?t=<64 hex>` (prueba `solo_la_direccion_del_motor`); solo navega dentro de ese origen; niega ventanas nuevas, descargas y permisos (cámara, micrófono…); sin herramientas de desarrollo (solo en depuración), sin atajos del navegador (F5, Ctrl+P, Ctrl+F) y sin menú contextual. Fondo #0E1117 desde el primer cuadro (sin destello blanco). Icono del exe (recurso 1). Abre maximizada; mínimo 720×480.
- Datos de WebView2 (caché): `%LOCALAPPDATA%\Otaconvert\WebView2`, o `<carpeta del exe>\datos\WebView2` si hay `portable.txt` (`carpeta_datos()`, primera pieza del modo portable de la V2). Nunca junto al exe.
- **Cargador de WebView2** (verificado en el código de webview2-com-sys 0.39.1): con **MSVC** se enlaza `WebView2LoaderStatic.lib` → queda **dentro del exe**, sin DLL extra (compatible con `/DEPENDENTLOADFLAG:0x800`); el workflow falla si el exe depende de `WebView2Loader.dll`. Con llvm-mingw (solo pruebas locales) usa `WebView2Loader.dll`: copiarla de `webview2-com-sys-0.39.1/x64/` junto al exe. Licencia del cargador: **BSD-3-Clause** (Microsoft; según fuentes secundarias; confirmar con el LICENSE del paquete NuGet). Es un binario sin código fuente dentro de un programa GPLv3. **Decidido por el usuario (16/10/2026): permiso adicional de la GPLv3 (sección 7)** en `LICENSE-EXCEPTION-WEBVIEW2.txt` (raíz; texto legal en inglés + resumen en español; el texto de la GPL no se toca), citado en los README. Solo cubre el código propio de Otaconvert; por eso el cargador va únicamente en `otaconvert-ventana.exe`, que no carga libmpv (GPLv2+ de terceros, que no da ese permiso). El workflow lo comprueba con `cargo tree`: falla si el motor depende de webview2 o si la ventana no lo tiene (para que la comprobación no quede ciega). También prueba que la ventana arranca y rechaza una dirección falsa (código 2), y el artifact del motor lleva la ventana y el archivo del permiso.
- Si falta `otaconvert-ventana.exe` junto al motor: aviso y navegador.
- Motor: `--abrir-show` abre la ventana por defecto; `--navegador` abre el navegador; `--sin-navegador` no abre nada. `Sesion::anotar(tipo, texto)` nuevo.
- **Probado en la PC del usuario:** la ventana abre maximizada con la interfaz, "Conectado al motor" y los audios "Listo"; cerrada a la fuerza → **vuelve en 130 ms**; cerrada normal → el programa termina sin procesos sueltos y el registro anota CERRAR; WebView2 roto a propósito (`WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` falso) → código 3; dirección falsa → código 2. **64 pruebas en verde.**
- Pendiente: confirmar el cierre si algo suena; que el motor final no abra consola (etapa 6).
- **Arreglos tras el run 38853948306 (falló):** (1) la prueba `cabeceras_enormes_se_cortan` falló en GitHub: el motor respondía 431 y cerraba con datos sin leer, y Windows corta así la conexión (RST) y el otro lado puede perder la respuesta. Ahora, tras el 431, el motor avisa el fin (`shutdown(Write)`) y descarta lo que llegue (máx. 1 MB, 500 ms). (2) `cargo audit` marcó **glib 0.18.5** (RUSTSEC-2024-0429) y **proc-macro-error 1.0.4** (RUSTSEC-2024-0370, sin mantenimiento): entran por la parte **Linux** (GTK) de tao/wry y no se compilan nunca para Windows (comprobado con `cargo tree --target x86_64-pc-windows-msvc`). Se ignoran en `cargo audit` con esa justificación en el workflow, y un paso nuevo falla si algún día entraran en la versión de Windows.

### Etapa 3, parte B: la interfaz del boceto conectada al motor (16/10/2026)
- `crates/servidor/ui/index.html` reescrita a partir del boceto aprobado: barra de menús (Archivo · Edición · Vista · Ayuda), cabecera (marca animada mientras suena, nombre del show con "guardado" / "cambios sin guardar", pestañas Show/Convertir, Edición/Función + Tab, conexión, reloj), aviso que se puede cerrar, lista de cues (número, icono por tipo, nombre, bucle, Listo/Preparando…/error, nota o archivo, salida, duración, en verde y con barrita lo que suena), panel lateral y botones GO (con el nombre del siguiente), Pausa (marcado en pausa), Fundir todo, Parar todo. Crédito abajo a la derecha. Todo con `textContent` (nunca HTML del show).
- **Lo que todavía no funciona aparece en gris con "Próximamente"** en los menús (Nuevo, Abrir, Guardar como, Reunir medios, Deshacer…, Ajustes del show, Personalizar, Idioma, Modo Ayuda, Primeros pasos, Mostrar consejos, pestaña Convertir). Funcionan: Guardar (y Ctrl+S), Vista → Modo noche / Modo día / Alto contraste / Restaurar vista (preferencia de la ventana en `localStorage`, con try/catch; pasará a los perfiles de operador), Ayuda → Atajos de teclado y **Acerca de Otaconvert** (con el crédito @marcold.ok y "Nadie se salva solo.").
- **Edición:** clic (o Enter) en un cue → queda en espera y se muestra a la derecha: nombre, nota, archivo, salida (Principal/Auriculares), volumen (−60 a +12 dB en la barra), fundido inicial y final, bucle, duración. Cada cambio se manda al motor al soltar o al salir del campo. Cue que suena: bloqueado con "Editar igualmente"; los cambios valen desde el próximo GO. El panel no se redibuja mientras se escribe.
- **Función:** a la derecha "En vivo": lo que suena (nombre, salida, tiempo restante o "en bucle", "Fundiendo", barra), "En pausa", y el siguiente con tipo, duración, estado y su nota. Clic en la lista no hace nada (↑↓ sí).
- **El motor manda en los modos:** `Sesion::modo` (Edición por defecto); órdenes `modo_edicion`, `modo_funcion` (se anota MODO), `ir_a {cue}` (rechazado en Función), `editar_cue {cue, cambios, forzar}` (rechazado en Función; valida tipo y rango de cada campo y el show entero con `validar` antes de aplicar; si algo no vale no cambia nada; solo nombre, nota, volumen −60..+24 dB, fundidos 0..600 s, bucle, salida Principal/Auriculares; nombre sin saltos de línea, ≤ 200 letras; nota ≤ 2000; un cue que suena exige `forzar`; anota EDITAR y marca cambios sin guardar), `cerrar_aviso`. Estado nuevo: `modo`, `auriculares`, por cue `archivo`, `salida`, `volumen_db`, fundidos y `duracion_s` (cuando está preparado), y por lo que suena `progreso`, `fundiendo`, `salida`.
- **Mayús+F** ahora funde el último cue que **todavía no se está fundiendo** (un segundo Mayús+F funde el anterior).
- **Esc decidido (sección 6):** Esc **para todo siempre**, también con un menú o una ventanita abierta o escribiendo en un campo, y de paso los cierra. Así el pánico nunca se retrasa.
- Teclas: Espacio, Esc, P, F, Mayús+F, ↑↓, Tab (con foco en un campo de texto, Tab pasa de campo), Ctrl+S.
- Título de la ventana = título de la página ("<show> · Otaconvert"): `with_document_title_changed_handler` → evento propio de tao → `set_title` (sin caracteres de control, ≤ 200).
- Pruebas nuevas (`tests/sonido.rs`): en Función no se cambia el show (editar e ir_a rechazados, ↑↓ sí, MODO en el registro); editar valida todo antes de cambiar (9 pedidos malos, incluido cambiar `archivo` a `../../Windows/system.ini`, no cambian nada; cue sonando solo con `forzar`; editar no manda nada al sonido); el estado trae duración, salida, archivo, progreso y fundiendo. **67 pruebas en verde.**
- **Probado en el panel del navegador** (1366×768): se ve como el boceto; clic en un cue lo deja en espera; renombrar se ve en la lista y en el GO y pone "cambios sin guardar"; Tab pasa a Función con "En vivo" y el clic en la lista no cambia nada; menús abren y cierran; Acerca de muestra el crédito; Esc para todo y cierra; Ctrl+S guarda. Registro: EDITAR, MODO, GUARDAR, PARAR. Captura nueva del programa real en el manual (`v2/manual/img/pantalla-inicio.png`, Edge sin ventana con `--virtual-time-budget`).
- Pendiente para 3C: Modo Ayuda, consejos emergentes y español/inglés. Más adelante: confirmar el cierre si algo suena, onda, deshacer, Ajustes del show.
- Pendiente: volumen y nivel por salida en pantalla (medidor, etapa 5); auriculares a otra frecuencia (preparar por salida); FFmpeg como respaldo para Opus/WMA/AC3/DTS.
