# Otaconvert: contexto del proyecto

Léelo entero antes de proponer o tocar nada. Si algo de aquí choca con el código, el código manda en lo que ya existe y este archivo manda en lo decidido para el futuro. En caso de duda, pregunta.

## 0. Cómo trabajar con el usuario

- **Siempre en español**, claro y sin jerga. El usuario es técnico de teatro, no programador ni experto en Claude. **Nunca en otro idioma salvo que él lo pida** (16/10/2026): también los resúmenes finales, los botones de respuesta, la lista de tareas y las descripciones de cada paso.
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
- **Fase 1. Hecha el 21/10/2026** (ver sección 12, «Botonera de sonidos»). Pendiente: agregarla al boceto.
- Ayuda para principiantes: "La botonera es como un teclado de efectos: cada botón hace sonar algo cuando lo tocas, aunque ya esté sonando otra cosa".

## 5c. Gráficos en vivo: temporizadores y complementos (pedido por el usuario el 16/10/2026)

Pedido: "agregar temporizadores para proyectar cuando hay intervalos en medio de alguna película o algo que lo requiera. Temporizadores con distintos diseños seleccionables. También varios complementos de ese estilo que puedan servir."

- **Nuevo tipo de cue: Gráfico** (lo dibuja el programa, sin archivo). Sale por el proyector como un vídeo más (capas sin negros, Parar todo, Fundir todo, Mayús+F), y en la caja «al aire» de la Vista Estudio.
- **Temporizador** (lo principal): cuenta regresiva **durante** X minutos o **hasta** una hora del reloj ("El show continúa a las 21:30"); texto arriba y abajo ("Intervalo", "Volvemos en"); al llegar a cero: quedarse en 0:00, mostrar un texto, o seguir con el próximo cue (cuando exista auto-continuar, Fase 2); aviso sonoro opcional en el último minuto (timbre de sala). Se puede pausar, sumar o restar un minuto en vivo (el intervalo se alarga).
- **Diseños seleccionables** (plantillas con vista previa): Grande y limpio · Anillo que se vacía · Barra de progreso · Reloj digital (tipo LED) · Elegante sobre una imagen o vídeo de fondo del show. Cada uno con colores, letra y fondo a elección, con la estética protegida (contraste automático, como en Vista).
- **Complementos propuestos** (a confirmar con el usuario cuáles quiere primero): **Reloj** (hora actual), **Cartel de texto** (avisos: "Apaguen sus celulares", título de la obra, créditos, "Gracias por venir"), **Cuenta hacia arriba** (cronómetro), **Patrón de prueba** para enfocar y encuadrar el proyector (cuadrícula, colores, círculos), **Pantalla de color** (negro, blanco, un color de luz), **Logo** con fondo, **Cartel + temporizador** juntos (sala: "La función empieza en 5:00").
- **Cómo se dibuja (a verificar antes de programar):** idea: con la capa de texto de mpv (OSD/ASS: letras, colores, bordes y dibujos vectoriales para el anillo y la barra) sobre un fondo de color, imagen o vídeo, en el mismo proceso de proyección y por la tarjeta gráfica; se actualiza una vez por segundo (liviano en equipos modestos). Verificar en la documentación de mpv antes de decidir.
- **Confirmado por el usuario (17/10/2026: "sí a todo"):** quiere **todos** los complementos propuestos (reloj, carteles de texto, cronómetro, patrón de prueba, pantalla de color, logo, cartel + temporizador) además del temporizador con sus diseños.
- **Fase:** propuesta (el usuario delega el reparto): **Fase 2**, junto con esperas y auto-continuar (el temporizador que al llegar a 0 dispara el siguiente cue). Si hace falta antes para un show concreto, se adelanta.
- Cada opción con su consejo emergente y ejemplos de escena (regla de interfaz), en español e inglés.

## 6. Barra de menús y menú Vista

### Barra de menús (decidido el 15/10/2026)
El usuario pidió **no llenar la ventana de botones** para ajustes del tipo "mostrar/ocultar mensajes". Esos ajustes van en una **barra de menús desplegables arriba de todo** (como Archivo · Edición · Vista · Ayuda en cualquier programa). En la ventana solo quedan los botones de trabajo: GO, Pausa, Fundir todo, Parar todo, el indicador Edición/Función y las pestañas Show/Convertir. Solo los menús necesarios:

| Menú | Contenido |
|---|---|
| **Archivo** | Nuevo show · Abrir… · Abrir reciente ▸ · Guardar · Guardar como… · Salir (sin "Reunir medios": al agregar, lo de fuera ya se copia a `Medios/`, etapa 5A) |
| **Edición** | Deshacer · Rehacer · **Agregar cues…** · Eliminar cue · Subir cue · Bajar cue · Copiar · Pegar · **Botonera…** · **Ajustes del show…** (rampas cortas, salidas y perfiles de teatro, protección contra doble GO) |
| **Vista** | Personalizar… · Vistas listas ▸ · Modo noche · Modo día · Madera de fresno · Botonera ▸ (Oculta / Cuadrícula / Lista) · Idioma ▸ (Español / English) · Restaurar vista |
| **Ayuda** | Modo Ayuda (H) · Primeros pasos · **Mostrar consejos** ✓ · Atajos de teclado · Prueba de hardware… · Acerca de Otaconvert |

- Cada opción aparece **una sola vez** (nada repetido en dos menús ni como botón suelto).
- En Función los menús siguen ahí pero lo que edita queda gris: solo Vista → Modo noche/Modo día/brillo, Archivo → Guardar y todo Ayuda. Abrir un menú nunca roba Espacio (GO) ni Esc (Parar todo). **Decidido en la etapa 3B (16/10/2026): Esc para todo siempre** (también con un menú o una ventanita abierta, o escribiendo) y el menú se cierra a la vez; así el pánico nunca se retrasa.
- Boceto actualizado el 15/10/2026: la barra de menús ya está en `v2/diseno/boceto-pantalla-principal.html` (Vista y Ayuda dejaron de ser botones de la cabecera; el Modo Ayuda se activa desde Ayuda → Modo Ayuda o con H). Captura de cómo se ve al abrir, a 1366×768: `v2/diseno/pantalla-inicio.png` (hecha con Edge sin ventana: `msedge --headless=new --window-size=1366,768 --screenshot=…`). Comprobado en el navegador: los menús abren y cierran, en Función Edición queda en gris y Guardar sigue activo, Mostrar consejos se apaga y se vuelve a encender. La versión publicada del boceto (enlace de la sección 2) todavía es la anterior.

### Menú Vista

- **Vista → Personalizar:** arrastrar paneles y botones con vista previa en vivo y pulsar **Listo**. Nada de ventanas con cien casillas.
- Personalizable: pestañas y paneles visibles (lista, detalle, onda, medidores, reloj, notas) y su posición; botones de Función (orden, tamaño, color, etiqueta); columnas; teclas rápidas; tamaño de letra; densidad (compacta, normal, táctil); brillo; acento; atajos (avisa si se repiten).
- Vistas: Estándar, Minimalista, Pantalla táctil, Alto contraste, **Modo noche** (tenue y rojizo) y **Modo día** (fondo blanco y contraste máximo para funciones al aire libre, pedido el 15/10/2026) y **Madera de fresno** (tema opcional cálido de madera clara, pedido y aprobado el 21/10/2026; ver sección 12). Noche y día se pueden cambiar también en Función.
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
- **Fase 2 —** Cue de Fade; esperas y auto-continuar; **gráficos en vivo: temporizadores con diseños, reloj, carteles y otros complementos (sección 5c)**; Grupos; Playlist; teclas rápidas; exportar/imprimir la hoja de cues para el regidor; Vista → Personalizar, perfiles de operador, botones a medida.
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
- Más adelante: confirmar el cierre si algo suena, onda, deshacer, Ajustes del show.
- **CI tras subir 3B** (run 38855333926): falló el paso nuevo "Probar la ventana": la ventana final es un programa sin consola y PowerShell no lo espera si se llama directo (`$LASTEXITCODE` vacío). Arreglo: `Start-Process -RedirectStandardInput … -NoNewWindow -Wait -PassThru` y leer `ExitCode` (probado en la PC: código 2). Run 38855726634 (commit del arreglo): **todo verde** (compilación, pruebas, ventana, vulnerabilidades).

### Etapa 3, parte C: Modo Ayuda, consejos y español/inglés (16/10/2026)
- **Textos de la interfaz** en `crates/servidor/ui/textos.json`: `es` y `en`, cada uno con `ui` (118 textos de pantalla), `consejos` (10 opciones), `ayuda` (13 partes de la pantalla, para el Modo Ayuda) y `pasos` (Primeros pasos, 6 pantallas). El motor lo incluye en el exe y lo mete en la página al servirla (`"__TEXTOS__"`; `</` se escapa para que ningún texto pueda cerrar el `<script>`). En la página: función `T("clave")` y atributos `data-t`, `data-t-title`, `data-help`, `data-consejo`.
- **Pruebas** (`tests/textos.rs`, 5): mismas claves en los dos idiomas; nada vacío; cada consejo con sus 6 partes, ≤ 200 letras, distinto en cada idioma y con ejemplos que empiezan por "Ej.: " / "E.g.: "; ayuda ≤ 400 letras y traducida (el título puede coincidir: GO); **todo texto, ayuda o consejo que usa la página existe en el JSON**; el escape de `</` deja un JSON válido.
- **Consejos emergentes** (regla de interfaz): botón "?" junto a Salida, Volumen, Fundido inicial, Fundido final, Repetir en bucle y Nota, y en Vista (Modo noche, Modo día, Alto contraste) y Ayuda → Mostrar consejos. Aparecen al pasar el ratón (medio segundo) o al tocar el "?": título, qué hace, "Cuándo usarlo" + ejemplo de escena, "Cuándo no" + ejemplo. **Ayuda → Mostrar consejos** los oculta o muestra; esa casilla nunca se oculta. Resumen de los consejos nuevos (español):

| Opción | Cuándo usarlo · ejemplo | Cuándo no · ejemplo |
|---|---|---|
| Salida | Auriculares para escuchar sin que oiga la sala. *Revisar la música del final antes de abrir sala.* | Principal para lo que es parte de la función. *La lluvia de la escena 3.* |
| Volumen | Bajarlo si tapa a los actores. *Ambiente de fiesta a −12 dB bajo el diálogo.* | 0 dB si el archivo ya está nivelado. *La canción final ya preparada.* |
| Fundido inicial | Para que algo aparezca suave. *El viento entra con 3 s mientras baja la luz.* | Si tiene que pegar justo. *El timbre del teléfono cuando el actor lo mira.* |
| Fundido final | Si el archivo termina de golpe. *Música recortada a mitad: 4 s de fundido.* | Si ya termina solo. *Un portazo grabado.* |
| Bucle | Ambientes que duran toda la escena. *Música de sala, lluvia.* | Sonidos de una vez. *Disparo, aplauso, canción final.* |
| Nota | Cuándo dar el GO, como en el libreto. *«cuando la novia cruza la puerta».* | Si el momento es obvio. *La música de sala del principio.* |
| Modo noche | Cabina a oscuras. *La pantalla se ve desde la platea en los apagones.* | De día o en ensayos con luz. *Ensayo técnico de tarde.* |
| Modo día | Al aire libre. *Función en una plaza a las 5 de la tarde.* | En sala oscura. *La función de noche en el teatro.* |
| Alto contraste | Pantalla difícil de leer. *Monitor viejo de cabina.* | Si ves bien los colores. *Con un buen monitor cansa menos el normal.* |
| Mostrar consejos | Aprendiendo u otra persona opera. *Un compañero nuevo opera mañana.* | Si ya conoces el programa. *Meses con el mismo show.* |

- **Modo Ayuda** (tecla **H** o Ayuda → Modo Ayuda): barra arriba "Toca cualquier parte con borde punteado…", bordes punteados en 13 partes (menús, pestañas, modos, conexión, reloj, aviso, lista, panel en Edición y en Función, GO, Pausa, Fundir todo, Parar todo); un clic muestra la explicación para principiantes **en vez de** hacer la acción. **Las teclas siguen funcionando** (decisión de seguridad: Espacio da GO y Esc para todo también en el Modo Ayuda; Esc además sale de la ayuda). Se sale con H, Esc o "Salir de la ayuda".
- **Primeros pasos**: 6 pantallas (bienvenida + 5 pasos) con Anterior / Siguiente; aparece sola la primera vez que se abre el programa y desde Ayuda → Primeros pasos.
- **Idioma** (Vista → Idioma → Español / English): cambia todo junto. La ventana manda la orden `idioma {es|en}` al motor al conectarse y al cambiarlo, y el motor manda sus avisos y los errores de preparación en ese idioma: `Texto { es, en }` (`crates/servidor/src/texto.rs`) en `Sesion::aviso`, los errores de cada cue, `editar_cue`, el canal y los avisos del motor (tarjeta desconectada, ventana que no abre). `ErrorPreparar::texto(idioma)` en la librería de audio (variantes nuevas `CambiaAMitad`, `DemasiadasPartesIlegibles`, `NoSePuedeAdaptar` para no dejar detalles en español). El registro de la función sigue en español. **En inglés el modo Función se llama "Live"** (no "Show", para no confundirse con la pestaña Show).
- Límite honesto: el detalle técnico entre paréntesis de algunos errores viene de Windows o de la librería (en su idioma), y los detalles de validación del archivo del show están en español.
- Preferencias de esta ventana (idioma, consejos, tema, primeros pasos vistos) en `localStorage`, con try/catch; los datos de WebView2 están en la carpeta de datos del programa (o la portable), así que se conservan. Pasarán a los perfiles de operador.
- **Probado en el panel del navegador:** Primeros pasos aparece la primera vez y recorre las 6 pantallas; el "?" de Volumen muestra el consejo con sus ejemplos; en inglés cambian menús (File, Edit, View, Help), botones (Pause, Fade all, Stop all), panel, etiquetas, crédito ("Made by @marcold.ok…") y el aviso del motor ("Video and image cues arrive in stage 4…"); Modo Ayuda con H, un clic en GO muestra la explicación y **no** dispara; ocultar consejos quita los "?" y volver a activarlos los trae (6). **73 pruebas en verde.**
- **Etapa 3 completa.** Siguiente: etapa 4 (vídeo e imagen: proyección en proceso aparte con vigilante, precarga y capas, imágenes, superficie de 4 puntos, miniatura). Es la parte de más riesgo técnico: avisar al usuario antes de empezarla (regla de cambios riesgosos).
- CI de 3C: run 38856602326, **todo verde**.

### Etapa 4: riesgos explicados y decisiones del usuario (16/10/2026)
- Riesgos explicados al usuario antes de empezar: negros entre vídeos en el flujo real (la prueba de la Fase 0 mira ~57 veces por segundo, no cada cuadro); la capa de abajo asomando en equipos sin GPU; caída del proceso de proyección (debe quedar en negro, nunca el escritorio, y volver en < 1 s); una sola pantalla (la proyección no debe tapar el programa); la superficie de 4 puntos (mpv no deforma con 4 esquinas de forma simple: filtro por CPU o dibujo propio por GPU, a investigar y medir); equipos modestos con dos vídeos a la vez; lo ya hecho no corre peligro (el audio no se toca; el archivo del show suma campos compatibles).
- **Decisiones del usuario:**
  1. **Los vídeos a veces tienen sonido que debe oírse** → el audio del vídeo va por **nuestro mezclador** (limitador, salida elegida, rampas, Parar todo), no por mpv.
  2. **No puede conectar otra pantalla en su PC.** Las pruebas con proyector o segunda pantalla las hará **en el teatro, cuando haya una demo de la V2 instalable** en la PC del teatro. Hasta entonces se prueba en su PC con una **ventana de vista previa** (con una sola pantalla la proyección sale en una ventana, no a pantalla completa encima del programa).
  3. "Como mejor lo veas, que salga bien sin que se haga eterno" → división elegida: **4A** proyección en proceso aparte con vigilante, vídeos por capas sin negros e imágenes; **4B** sonido de los vídeos por el mezclador, miniatura del proyector y elección de pantalla; luego **demo portable** para probar en el teatro; **4C** superficie de 4 puntos (después de investigar el mejor camino).
  4. **Encuadre y recorte en la 4C** (preguntado por el usuario el 16/10/2026; confirmado el 17/10/2026: "sí a todo"): por cue de vídeo o imagen, **Encuadre** (Ajustar con bandas negras · Llenar recortando · Estirar), **Zoom**, **Posición** (mover la imagen) y **Recorte** de los bordes (arriba, abajo, izquierda, derecha). Con su consejo y ejemplos de escena. Se guarda en el cue (viaja con el show).

### Etapa 4, parte A: proyección en proceso aparte (16/10/2026)
- **Hallazgo antes de empezar:** en el prototipo siempre había un vídeo sonando arriba que tapaba la precarga; en el programa real la proyección empieza en negro y un vídeo precargado se vería **antes de su GO**. Solución: un **telón** (ventana hija negra sin vídeo) encima de todo. Precargar = capa debajo del telón; GO = esa capa sube encima de todo y arranca; Parar todo = el telón sube al instante.
- `crates/video/src/ventana.rs`: `Proyeccion::abrir_con(rect, capas, Modo::{Completa, VistaPrevia})` con telón (`telon_arriba`, `arriba`, `abajo`), las capas siguen el tamaño de la ventana (WM_SIZE), nunca toma el foco (WS_EX_NOACTIVATE: Espacio y Esc siguen yendo al programa), el botón de cerrar de la vista previa no cierra la proyección, `color_centro` mide el centro de la ventana.
- `crates/motor/src/proyector.rs`: proceso `otaconvert-motor --proyector x y ancho alto completa|vista`. 4 capas mpv + telón. Órdenes JSON por la entrada estándar: `precargar {id, ruta, bucle, imagen, inicio, fin}`, `mostrar`, `negro`, `fundir {ms}` (baja el brillo de mpv de 0 a −100 y al final el telón), `pausa`, `quitar`, `salir`. Avisos por la salida: `hola`, `listo {id, duracion}`, `error {id, motivo}` (códigos: abrir, ruta, capas, tiempo, no_listo; la sesión los traduce), `mostrando`, `fin` (vídeo sin bucle terminó: queda el último cuadro), `latido` cada 100 ms. La capa anterior se suelta 300 ms después de que la nueva la tapa. Solo acepta rutas absolutas de archivos que existen (nada de red ni `://`; `av://lavfi:` solo con `OTACONVERT_PRUEBA_VIDEO=1` para las pruebas). Si se cierra la entrada estándar (el motor ya no está), sale.
- `crates/motor/src/proyector_cliente.rs`: `ProyectorProceso` abre y **vigila** el proceso: si termina o pasa 1 s sin latido, lo cierra y lo vuelve a abrir (en negro) y avisa `Reiniciado {ms}`; más de 3 caídas en un minuto → `Caido`.
- `crates/servidor/src/proyector.rs`: trait `Proyector`, `OrdenProyector`, `AvisoProyector`, `motivo(codigo)` → `Texto`. `Sesion`: `conectar_proyector`; precarga los **2** próximos cues de vídeo o imagen (`PRECARGAR_VISUALES`) y suelta los que ya no hacen falta (`Quitar`); GO de vídeo/imagen: listo → `Mostrar` (lo anterior en la proyección deja de verse); preparando → no dispara y avisa; error o sin proyección → avanza y avisa (la función sigue). Parar todo → `Negro`; Fundir todo y Mayús+F → `Fundir`; Pausa → `Pausa`; aviso `Fin` → sale de "Sonando". **Tras una caída** se vuelve a precargar lo que se veía desde donde iba y se muestra solo al estar listo; se avisa "La proyección se cerró sola y se volvió a abrir. El sonido no se cortó." Estado: preparación y duración de vídeos, `visual`/`imagen` en lo que suena, `con_proyeccion`.
- Motor: `--abrir-show` abre la proyección en la primera pantalla que no es la principal, a pantalla completa; **con una sola pantalla, una ventana de vista previa** arriba a la derecha (un cuarto del ancho), que no tapa los botones; `--pantalla <n | nombre | vista>` para elegir. Si la proyección no arranca, el sonido funciona igual y se avisa.
- Ventana: cues de vídeo con "Repetir en bucle" y duración; "Imagen fija" en Función; "Sin proyección" en la conexión si falta.
- **Pruebas:** `tests/proyeccion.rs` (8, proyector falso): precarga de los 2 siguientes con ruta completa; GO sin precargar no dispara y con listo muestra; parar, fundir y pausa llegan; el fin del vídeo lo saca de "Sonando"; un error avanza y avisa, un archivo que falta no se pide; mover la espera suelta lo que sobra; tras una caída vuelve lo que se veía desde donde iba; sin proyección el GO avanza y avisa. Prueba `solo_archivos_locales` en el proceso. **82 pruebas en verde.**
- **`--prueba-proyector [cambios]`** (proceso de verdad, colores de mpv, mide el centro de la pantalla). **En la PC del usuario** (una sola pantalla: recuadro de 640×360 en el centro): antes del primer GO, mientras precarga: **0 muestras no negras**; 30 cambios: **0 negras, 0 de otro color**, GO → imagen 33 ms de media (37 máx.; la Fase 0 daba 18 ms sin proceso aparte); Parar todo → negro en 35 ms; Fundir 1 s → negro al final; **caída a propósito → de nuevo abierta en 92 ms y en negro**. RESULTADO: BIEN.
- **Probado con el programa** (show de ejemplo con un vídeo de prueba de 12 s y una imagen, creados con FFmpeg): los dos quedan "Listo"; GO del vídeo → se ve en la vista previa; GO de la imagen → la reemplaza sin negro; Parar todo → negro (píxel 0,0,0). Por un error de la prueba (la ventanita de Primeros pasos, cerrada a mano, seguía bloqueando ↓) sonó 8 s la música de sala; se paró con Esc.
- **Subida y comprobada:** run 38866828412 (commit 38c719c), **todo verde**.
- Límites y pendientes: el fundido por brillo baja rápido al principio (a la mitad ya está bastante oscuro): suavizar la curva; GO → imagen 33 ms (aceptable, 2 cuadros); el reloj de la tarjeta de sonido todavía no manda sobre el vídeo (4B, con el sonido del vídeo); sin GPU (GitHub) la capa de abajo puede asomarse (sin probar con el telón); falta probar con proyector real (en el teatro, con la demo).
- Pendiente: volumen y nivel por salida en pantalla (medidor, etapa 5); auriculares a otra frecuencia (preparar por salida); FFmpeg como respaldo para Opus/WMA/AC3/DTS.

### Etapa 4, parte B: el sonido de los vídeos (16/10/2026)
- **Hecho:** comprobado que `preparar` saca el sonido de un vídeo (elige la pista de audio con `default_track(TrackType::Audio)`): prueba `el_sonido_de_un_video_se_prepara` con `tests/datos/tono-video.mp4` (H.264 + AAC, 1 s, tono 1 kHz a 0,5) → duración, pico y frecuencia correctos; `video-mudo.mp4` (sin sonido) → `ErrorPreparar::SinAudio`.
- **Plan para seguir (diseño ya pensado):**
  1. La sesión prepara también el sonido de los cues de vídeo (como un audio). `SinAudio` = vídeo mudo (no es error); un sonido que no se puede leer avisa pero **no** impide mostrar el vídeo.
  2. GO de vídeo: listo cuando la imagen está precargada **y** el sonido preparado (o es mudo). Se dispara la imagen y el sonido a la vez por **nuestro mezclador**, con el volumen, la salida (Principal/Auriculares), los fundidos y el bucle del cue. `Sonando` guarda también la voz de audio del vídeo (Mayús+F funde imagen y sonido).
  3. En bucle, el punto de vuelta del sonido = la duración del vídeo que da mpv (si no, el sonido y la imagen se separan un poco en cada vuelta).
  4. En el panel, los cues de vídeo muestran Salida, Volumen y Fundidos (como el audio).
  5. Fundido a negro más suave: curva `t²` en el brillo (hoy a la mitad ya está muy oscuro).
  6. Pendiente honesto: que el reloj de la tarjeta de sonido mande sobre el vídeo (corregir la velocidad de mpv según la posición real del sonido). Medir primero la deriva en vídeos largos; en vídeos de pocos minutos la diferencia esperada es de milisegundos.
  7. **Miniatura del proyector dentro del programa:** propuesta de dejarla para la etapa 5 con un método liviano. Sacar capturas de mpv cada segundo cargaría a los equipos modestos durante la función. Hay que consultarlo con el usuario.
  8. Elegir la pantalla desde la interfaz va en Edición → Ajustes del show → Salidas (etapa 5); hoy se elige con `--pantalla`.
- Después de 4B: **demo portable** para que el usuario la pruebe en el teatro con el proyector.
- **Hecho (16/10/2026, al retomar):**
  - La sesión prepara el sonido de los cues de **vídeo** igual que un audio (`siguiente_a_preparar` incluye Audio y Vídeo). `Sesion::resultado_preparar(i, Result<Preparado, ErrorPreparar>)`: `SinAudio` en un vídeo → queda **mudo** (`mudos`, no es un error); lo demás va a `preparado`. El hilo preparador usa `resultado_preparar`.
  - Un vídeo está **listo** cuando la imagen está precargada **y** su sonido preparado (o es mudo, o su sonido falló, o no hay salida de sonido). Si el sonido no se puede leer, el vídeo **se muestra igual** y se avisa ("el vídeo se ve, pero su sonido no se puede reproducir").
  - GO de vídeo: `Mostrar` a la proyección y, a la vez, `iniciar_voz` manda el sonido al mezclador con el volumen, la salida (Principal/Auriculares), los fundidos y el bucle del cue. **En bucle**, el fin del sonido es la duración del vídeo que da mpv (con el fin del cue si lo tiene), así imagen y sonido vuelven a empezar juntos. `disparar` (audio) usa la misma `iniciar_voz`.
  - `Sonando.voz`: si tiene voz en el mezclador. **Mayús+F** funde la imagen **y** su sonido; un vídeo nuevo **para el sonido del anterior** (con la rampa de parada); Parar todo, Fundir todo y Pausa ya cubrían las dos cosas.
  - Estado: para vídeo, la duración la da mpv (manda sobre la del sonido); `"mudo"` por cue; la preparación de un vídeo dice "preparando" hasta que también esté su sonido.
  - Panel: un vídeo con sonido muestra Salida, Volumen, Fundido inicial y final y Bucle (con sus consejos), como un audio; uno mudo, solo Bucle y duración. Textos `video_pronto` (nuevo texto) y `video_mudo` en es/en.
  - **Fundido a negro más parejo:** brillo de mpv con curva `t²`. En la PC del usuario, a la mitad de 1 s: antes (16,128,128) desde (72,184,185) (~70 %); con la curva lineal estaba casi negro (0,54,0).
- **Pruebas** (`tests/proyeccion.rs`, 2 nuevas, con vídeos reales `tono-video.mp4` y `video-mudo.mp4` y un sonido falso): el vídeo con sonido no dispara hasta tener el sonido; al GO, imagen y sonido a la vez, en bucle con `hasta` = 0,9 s × 48 kHz = 43.200 cuadros; Mayús+F manda `Fundir` y `Parar {voz, 3 s}`. El mudo queda "listo", sin error, no manda sonido, y el vídeo nuevo para el sonido del anterior. **85 pruebas en verde.**
- **En la PC del usuario:** `--prueba-proyector 10`: 0 negros, nada antes del GO, Parar todo 32 ms, caída → 86 ms en negro, fundido parejo. Programa real con el vídeo de prueba con sonido (tono de 440 Hz suave): GO → "1 sonando" con imagen en la vista previa; Mayús+F a los 3 s → "0 sonando"; registro GO y FUNDIR "solo Cue 2". **Confirmado por el usuario (16/10/2026, tercera prueba con él delante: "perfecto"): el sonido del vídeo se oyó y bajó junto con la imagen.** (Las dos primeras veces no estaba en el ordenador.) Detalle a revisar: una muestra al final dio gris oscuro (21,21,21) en el lugar de la vista previa (posiblemente otra ventana encima: la vista previa no es "siempre encima"); la prueba automática del mismo fundido termina en (0,0,0).
- **Sin hacer y por qué:**
  - **Que el reloj de la tarjeta de sonido mande sobre el vídeo:** hoy imagen (mpv) y sonido (tarjeta) arrancan juntos y cada uno sigue su reloj. En vídeos de minutos la diferencia esperada es de milisegundos; hay que medirla en vídeos largos antes de corregirla (ajustar la velocidad de mpv).
  - **Miniatura del proyector y elegir pantalla en la interfaz:** **decidido por el usuario (16/10/2026): van a la etapa 5.** La miniatura con capturas de mpv cada segundo cargaría a los equipos modestos durante la función; elegir la pantalla va en Ajustes del show → Salidas. Hoy se elige con `--pantalla`.
- **Subida y comprobada:** run 38925664454 (commit 020540b), **todo verde**.
- Siguiente: **demo portable** para probar en el teatro con el proyector; luego 4C (4 puntos).

### Demo portable para probar en el teatro (16/10/2026)
- **Uso con el ratón** (motor, `main.rs`):
  - Un solo argumento que es una ruta que existe (lo que pasa al **arrastrar** una carpeta o un `.otashow` sobre el exe) → `--abrir-show <ruta>`.
  - **Doble clic** en un exe llamado `Otaconvert.exe` sin argumentos → abre la carpeta `Show de ejemplo` que está a su lado; si no existe, explica cómo arrastrar una carpeta.
  - Abierto así, si hay un error se espera Enter antes de cerrar la consola (para poder leerlo).
- **Carpeta sin `.otashow`** (`show_cmd::crear_desde_carpeta`): arma un show con **un cue por cada audio, vídeo e imagen**, también en subcarpetas (hasta 3 niveles), sin carpetas ocultas ni `registros`, **ordenados por nombre de archivo** (así "01 Luna.mp4" va primero aunque esté en una subcarpeta); nombre del cue = nombre del archivo sin extensión; tipo por extensión (audio: wav mp3 flac ogg oga m4a aac aif aiff aifc caf mka; vídeo: mp4 m4v mov mkv webm avi; imagen: png jpg jpeg bmp gif webp); solo rutas que acepta el archivo del show; máximo 10.000. Lo guarda como `<carpeta>.otashow` dentro de la carpeta y lo anuncia. Carpeta sin medios → error claro sin crear nada. Prueba `una_carpeta_con_medios_se_vuelve_un_show`. **86 pruebas en verde.**
- **Paquete** (paso "Armar la demo portable" del workflow V2, artifact **`Otaconvert-Demo-portable`**): carpeta `Otaconvert-Demo-<versión>` con `Otaconvert.exe` (el motor renombrado), `otaconvert-ventana.exe`, `libmpv-2.dll`, `vulkan-1.dll` + licencia, `LICENSE.txt` (GPLv3), `LICENSE-EXCEPTION-WEBVIEW2.txt`, `portable.txt`, `Manual de usuario.pdf`, `LEEME.txt` (`v2/demo-portable/LEEME.txt`), **`Prueba de hardware.exe`** con su carpeta `pruebas/`, y **`Show de ejemplo/`** (creado con `--show-ejemplo`; sus medios se generan en el workflow con FFmpeg: acordes suaves, viento, campanas, aplausos, trueno, un vídeo 1080p de 12 s **con sonido** (tono de 440 Hz suave) y una imagen de barras; nada de terceros).
- `LEEME.txt` (español, pasos simples): copiar al disco del equipo (no usar desde el pendrive); conectar el proyector y poner Windows en **Extender** (Windows + P) antes de abrir; elegir la salida de sonido en el icono del altavoz (por ahora sale por la predeterminada de Windows); doble clic en `Otaconvert.exe` (SmartScreen: "Más información → Ejecutar de todas formas"); la ventana negra con mensajes es el motor: no cerrarla; arrastrar una carpeta con los propios medios (numerar "01 …" para el orden); formatos; teclas; **lista de qué probar en el teatro** (pantalla completa sin escritorio ni ratón, varios GO sin negros, negro antes del GO, vídeo con sonido, Parar todo, Fundir todo, sonido por la mesa sin clics); la Prueba de hardware y su informe; contacto @marcold.ok.
- **Probado en la PC del usuario** (demo armada a mano con el exe local): arrastrar una carpeta con un vídeo en una subcarpeta, un audio y una imagen → "Se armó el show «mis-medios» con 3 cues" y se abrió la ventana con la proyección en vista previa; doble clic sin argumentos → abre el "Show de ejemplo"; al cerrar la ventana no quedan procesos.
- Para descargarla hace falta una cuenta de GitHub (artifact): el usuario la baja en su casa y la lleva al teatro en un pendrive. Publicarla como Release (sin cuenta) **necesita su permiso**.
- Límites de la demo: la consola del motor queda visible; la salida de sonido es la predeterminada de Windows (elegirla en la interfaz: etapa 5); no se pueden agregar ni quitar cues desde la ventana (etapa 5); sin firma digital (SignPath pendiente).
- **Paquete de GitHub comprobado** (run 38926833772, commit d33847c, **todo verde**): artifact `Otaconvert-Demo-portable` de **89 MB** comprimido (170 MB descomprimido); `Otaconvert.exe` SHA-256 `B11A8CC9…98ACE7D`. Descargado con `gh run download` en una carpeta aparte de la PC del usuario: trae todo (LEEME, manual PDF, licencias, `portable.txt`, Prueba de hardware con `pruebas/`, Show de ejemplo con sus 8 medios generados); `--comprobar`: **no falta nada**; doble clic (sin argumentos) → abre el "Show de ejemplo" en la ventana propia, "Conectado al motor", vídeo e imagen "Listo"; el usuario la estaba usando (Primeros pasos en el paso 4).
- **Hallazgo:** con una sola pantalla, la **vista previa del proyector queda detrás** de la ventana del programa maximizada (no es "siempre encima"). En el teatro (proyector en Extender) sale a pantalla completa y no pasa. Arreglo propuesto: que la vista previa sea "siempre encima" y chica, en un lugar que no tape botones (decidir con el usuario).

### Vista Estudio (estilo OBS) y Vista Lista (estilo QLab): decidido el 16/10/2026
- **Pedido del usuario** (tras probar la demo): ver la imagen del próximo vídeo o imagen **antes de dispararlo**, como en OBS. Motivo: **hay shows y eventos sin un orden fijo de cues** y a veces hay que **buscar un cue a mano en el momento**.
- Investigado: QLab tiene el modo **"Audition"** (reproduce un cue para el operador sin mandarlo a las salidas reales; mejorado en QLab 5; atajo Opción+Espacio); en el grupo de QLab hay usuarios que piden un monitor aparte para previsualizar vídeo; en Isadora se usa una foto fija de cada escena; de MultiPlay no se encontró documentación de vista previa del siguiente (no confirmado).
- **Decisión del usuario:** el programa tiene **dos estilos de trabajo alternables**:
  - **Vista Lista** (la actual, estilo QLab / MultiPlay): lista ordenada, GO al siguiente, en Función no se elige con el ratón.
  - **Vista Estudio** (estilo "Modo Estudio" de OBS, mejorado): **clic en un cue lo elige** (también en Función, solo cambia el cue en espera; nunca dispara) y lo muestra en **"Vista previa · lo que viene"** (en movimiento y sin sonido; opción en Vista para dejarla **fija** en equipos modestos, con su consejo); **GO** es la "transición": lo elegido pasa al proyector; el GO muestra siempre el nombre del cue elegido; recuadro **"En el proyector · al aire"** con lo que se ve y cuánto falta (al principio con la foto del cue al aire; copia en vivo del proyector más adelante); **fotos chicas** de cada vídeo e imagen en la lista; **buscador** arriba de la lista; **"Ver antes"** (▶): reproduce el cue con sonido **solo por Auriculares** (si no hay, en silencio), sin tocar el proyector ni la sala.
  - Se cambia en **Vista → Estilo ▸ Lista / Estudio** (barra de menús). Edición y Función siguen igual en los dos estilos. El buscador y las fotos sirven en los dos.
- **Plan (antes de 4C):**
  1. **Miniaturas:** una foto chica (≈320 px) del cuadro de inicio de cada vídeo e imagen, hecha **una sola vez por archivo**, en segundo plano y en un **proceso aparte** (`--miniatura`, libmpv con salida a imagen), guardada en `.otaconvert/miniaturas/` de la carpeta del show.
  2. **Servir medios a la ventana con seguridad:** rutas nuevas del canal (con token, Host y CSP como las demás) que solo entregan la miniatura o el archivo de **un cue del show** por su número (nunca una ruta pedida por la ventana), con lectura por partes (Range) para el vídeo de la vista previa.
  3. **Buscador** de cues (número o nombre, sin importar tildes; Enter lo deja en espera; nunca dispara).
  4. **Vista Estudio** en la ventana: vista previa (vídeo en movimiento sin sonido con `<video muted>` de WebView2, o foto fija), al aire, clic elige. El motor permite `ir_a` también en Función (solo cambia el cue en espera); la Vista Lista sigue sin mandarlo.
  5. **Ver antes por Auriculares** (después).
- Límites a tener en cuenta: la vista previa en movimiento usa la decodificación de WebView2 (H.264 bien; HEVC puede necesitar la extensión de Windows: si no se ve, foto fija); es un vídeo más decodificándose a la vez (por eso la opción fija).
- **Pedido extra del usuario (mientras se hacía):** en la vista previa, los vídeos muestran **al menos su primer cuadro**; se les puede dar **▶ solo en la vista previa** para verlos enteros; y se puede **elegir si el GO sale desde el principio o desde donde quedó** en la vista previa.

#### Hecho (16/10/2026, tarde)
- **Miniaturas** (`crates/motor/src/miniatura.rs`, `--miniatura <archivo> <salida.jpg> [inicio_s]`): libmpv sin ventana (`Reproductor::sin_ventana`: sin config, scripts, internet, teclado, terminal ni sonido) con `vo=image`, `vo-image-format=jpg`, calidad 80, `frames=1`, `vf=scale=w=320:h=-2`, `start`; las opciones se comprobaron probándolas (mpv las acepta; la página del manual de mpv es demasiado larga para leerla entera con la herramienta). 320×180, ~7 KB, ~0,2 s por foto en la PC del usuario. El motor las hace en segundo plano (`generar_miniaturas` en `show_cmd.rs`): **una a la vez, en un proceso aparte con prioridad baja, sin ventana y con 20 s de límite**; se guardan en `.otaconvert/miniaturas/<FNV-64 de archivo+tamaño+fecha+inicio>.jpg` de la carpeta del show (si cambia el archivo o su inicio, cambia el nombre); las que fallan no se reintentan en la sesión. La sesión recuerda cuáles están hechas (no revisa el disco en cada refresco).
- **Rutas nuevas del canal** (`crates/servidor/src/lib.rs`): `/miniatura?t=…&cue=N` y `/medio?t=…&cue=N`. Mismo token y Host que la página; **solo entregan lo de un cue de vídeo o imagen del show, por su número** (la ventana nunca pide una ruta: `cue=../x`, `archivo=…`, un audio o un número que no existe → 404); el archivo se resuelve con `rutas::resolver` (dentro de la carpeta). `/medio` entrega **por partes** (`Range`, 206/416, `Accept-Ranges`) copiando por trozos (nunca entero en memoria); la miniatura se puede guardar en la caché de la ventana (su dirección lleva la versión). CSP de la página: `img-src 'self' data:; media-src 'self'`. Prueba `miniatura_y_medio_solo_de_cues_del_show`.
- **Elegir en Función:** `ir_a` ahora funciona también en Función (solo cambia el cue en espera; **nunca dispara**) y se anota **ELEGIR** en el registro. La Vista Lista sigue sin mandarlo con el ratón en Función.
- **Salir desde aquí** (`Sesion::elegir_desde(cue, Option<s>)`, orden `elegir_desde {cue, s}`): solo para el **vídeo en espera**, dentro de su duración; vuelve a precargar en ese punto (`OrdenProyector::Precargar` tiene ahora `desde_s`: el proyector carga con `start` = inicio del cue —para que el **bucle vuelva al inicio del cue**— y, ya cargado, salta en pausa al punto con `seek absolute+exact` y recién avisa "listo"); el GO saca la imagen en ese punto y el **sonido desde el mismo segundo** (`IniciarCue.desde`), y el tiempo restante cuenta desde ahí; se usa **una sola vez**; al cambiar de cue se olvida y ese vídeo se vuelve a precargar desde su inicio. Tras una caída de la proyección también se usa `desde_s` (antes se cambiaba el inicio, lo que rompía el bucle). Estado `desde_s`. Se anota DESDE en el registro.
- **Ventana:**
  - **Vista → Estilo ▸ Lista (como QLab) / Estudio (como OBS)** y **Vista previa en movimiento** (apagada por defecto: el vídeo queda en su primer cuadro y se le da ▶; encendida, arranca solo al elegirlo), cada una con su consejo. Preferencias de la ventana (`localStorage`).
  - **Buscador** arriba de la lista (los dos estilos): número o nombre/nota/archivo, sin mayúsculas ni tildes; Enter deja en espera el primero que coincide y suelta el foco (Espacio vuelve a ser GO); **Ctrl+F** o **/** lo abren; "Ningún cue coincide…".
  - **Fotos** de vídeos e imágenes en la lista (los dos estilos).
  - **Vista Estudio**: clic (o Enter) en un cue lo elige también en Función. Caja **"Vista previa · lo que viene"**: vídeo con `<video muted>` de WebView2 (primer cuadro del cue, ▶/⏸, barra para recorrerlo, tiempo; si no se puede ver —p. ej. HEVC sin la extensión de Windows— foto fija con aviso), imagen, o "Solo sonido" para un audio; número, nombre, tipo, duración, estado y nota; botones **"Salir desde el principio" / "Salir desde aquí (m:ss)"** → "Con el GO sale desde m:ss". Caja **"En el proyector · al aire"**: foto del cue al aire con su nombre, lo que falta / "Imagen fija" / "en bucle", "Fundiendo", lo que suena (♪) y "En pausa"; "Negro" si no hay nada. En Edición, debajo, los ajustes del cue elegido. El vídeo de la vista previa no se rehace en cada refresco.
  - Textos nuevos es/en (`estilo*`, `buscar_ph`, `vista_previa`, `al_aire`, `salir_*`…), 4 consejos (`estilo_lista`, `estilo_estudio`, `previa_movimiento`, `salir_desde`) y 3 ayudas (`buscador`, `previa`, `aire`); atajo Ctrl+F en Atajos de teclado.
- **Pruebas:** 89 en verde (nuevas: rutas seguras, salir desde aquí con la imagen y el sonido en el mismo punto y una sola vez, cambiar de cue olvida el punto, elegir en Función sin disparar y anotado).
- **Probado en el panel del navegador** (show de ejemplo, Vista Estudio): las 2 fotos se hicieron solas al abrir; clic en el vídeo → vista previa en su primer cuadro con ▶, barra y botones; ▶ 2,5 s y pausa → "Salir desde aquí" → "Con el GO sale desde 0:02"; buscar "mapa" deja solo el 3A y Enter lo pone en espera; en Función, GO de la imagen → al aire "3A · Mapa del pueblo · Imagen fija"; clic en un cue en Función lo elige sin disparar. Corregido: en ventanas angostas la foto tapaba el nombre en la lista.
- Pendiente: **"Ver antes" con sonido por Auriculares**; copia en vivo del proyector en "al aire" (hoy, el mismo vídeo sincronizado: ver arreglo abajo); que el GO desde un punto quede probado en el programa real con sonido (las pruebas automáticas lo cubren).
- **Arreglo (16/10/2026, noche): «al aire» parecía congelado.** El usuario vio el vídeo "congelado" en la caja «En el proyector · al aire»: era la foto fija del cue, no el proyector (con `--prueba-movimiento --archivo <vídeo>`, prueba nueva que mira 16 puntos de la proyección cada 200 ms, el vídeo de verdad se movía: 15 de 15). Ahora la caja muestra **el vídeo en movimiento, sin sonido**, con `<video muted>` de WebView2 desde `/medio`, siguiendo el punto del proyector: el motor manda `pos_s` (inicio del cue + tiempo de la vuelta actual) en cada cosa que suena; si la copia se separa más de 0,5 s se corrige; en pausa queda quieta; al terminar, en su último cuadro; si no se puede ver (HEVC sin la extensión), foto fija. Al cambiar de estilo se sueltan los vídeos de la ventana. Probado en el panel del navegador: avanza 1,8 → 3,7 s en 2 s, en pausa quieta, sigue al quitarla, Parar todo → "Negro". Sigue siendo una **copia aproximada** (otro vídeo decodificándose en la ventana), no una captura de la proyección. 89 pruebas en verde.

### Comprobaciones rigurosas en lugar del teatro (16/10/2026, noche)
- **Pedido del usuario:** no pudo hacer las pruebas en el teatro; "procura hacerlos vos de forma ultra rigurosa". Se repitió la lista del LEEME de la demo en su PC, lo más parecido posible a una función real.
- **Prueba nueva `--prueba-funcion [segundos] --carpeta <vídeos> [--pantalla N | vista] [--salida X]`** (`crates/motor/src/prueba_funcion.rs`): arma un show real en una carpeta con **tildes, eñe y espacios** (`%TEMP%\Otaconvert prueba función ñandú\Medios del show\Vídeo D04848.mp4`…): ambiente en bucle (WAV a **44,1 kHz**, se adapta a la tarjeta), 4 vídeos 1080p60 de color liso **con sonido** (tono 440 Hz en AAC), una imagen BMP y tonos cortos, todos con fundidos de 30 ms. Lo abre con la **misma sesión, el mismo sonido WASAPI y la misma proyección en proceso aparte, a pantalla completa** (sin otra pantalla: en la única, tapándola). Durante los segundos pedidos da GO (esperando "Listo", como un operador), Pausa/Sigue, Fundir todo, Parar todo, Mayús+F, elegir cue y "salir desde aquí", al azar con semilla fija. Mide:
  - **Imagen**: el centro de la proyección ~40 veces por segundo (y las 4 esquinas 1 de cada 8) contra lo que tocaría según la sesión; tras cada cambio valen 300 ms lo de antes o lo nuevo; al fundir vale el color oscureciéndose.
  - **Sonido que sale de verdad**: graba la salida en modo *loopback* de WASAPI (verificado en el código de cpal 0.18.2: un dispositivo de salida usado como entrada activa `AUDCLNT_STREAMFLAGS_LOOPBACK`) y busca clics con la segunda diferencia (umbral de tonos de 440 Hz con amplitud total 0,6 + la aspereza propia de los archivos, medida antes). Guarda `grabacion.wav` y `acciones.txt` con la hora de cada acción para revisar a mano.
  - **Pantalla completa**: que tape centro, esquinas y **barra de tareas** (`WindowFromPoint` → clase `OtaconvertProyeccion`, también durante toda la función), que **no se vea el ratón** sobre la proyección (`GetCursorInfo`) y que **un clic sobre la proyección no le robe el teclado** al programa (solo hace clic si debajo está la proyección).
  - **Salida de sonido**: llamadas tardías de la tarjeta (ver abajo), **memoria** del motor y de la proyección cada 30 s, **registro** sin errores, GO ignorados y cues que no llegaron a "Listo", y que Windows tenga registrado el pedido de no suspender.
  - El aviso de la grabadora "buffer underrun or overrun" llega **a los 0,00 s** de grabar (Windows marca el primer paquete como discontinuo): no cuenta; solo cuentan los posteriores.
- **Problemas reales encontrados y arreglados:**
  1. **Un ambiente en bucle a 44,1 kHz hacía clic en cada vuelta** con la tarjeta a 48 kHz (los clics salían cada 4,0 s exactos, el largo del bucle). Causa: al adaptar la frecuencia, el filtro (rubato) supone silencio antes del principio y después del final; además rubato agrega un cuadro de más. Arreglo: `preparar_con(ruta, fs, bucle)` (`crates/audio/src/preparar.rs`): si el cue se repite **de punta a punta** (`bucle` sin inicio ni fin propios), antes de adaptar se pone el final del archivo delante y el principio detrás (relleno múltiplo exacto del paso de la conversión) y se recorta justo la vuelta original (largo exacto). La sesión prepara así esos cues (`TareaPreparar { cue, ruta, frecuencia, bucle }`: `Sesion::siguiente_a_preparar` la devuelve y `hacer()` prepara) y **vuelve a preparar** un cue si se le cambia el bucle. Pruebas: `un_bucle_que_empalma_sigue_empalmando_al_adaptar_la_frecuencia` (44,1 / 22,05 / 96 kHz → 48 kHz; sin el arreglo, a 44,1 kHz la prueba ve el salto) y `un_bucle_de_punta_a_punta_se_prepara_como_bucle_y_cambiarlo_lo_vuelve_a_preparar`.
  2. **El sonido de los vídeos MP4/MOV y de los M4A empezaba con ~21 ms del "arranque" del codificador AAC** (1024 cuadros) y terminaba con relleno: quedaba **21 ms atrasado respecto de la imagen** y un vídeo en bucle saltaba a ese silencio en cada vuelta (clic). Causa verificada en el código: **Symphonia 0.6.1 lee la lista de edición del MP4 (`elst`) pero no la aplica** (`ElstAtom` marcado sin usar; MP3, OGG y CAF sí recortan). Arreglo: `crates/audio/src/mp4.rs` lee con límites (`moov` ≤ 64 MB, se salta `mdat`) la lista de edición del primer sonido (`hdlr` = `soun`): ediciones vacías = silencio delante; la primera con sonido = cuánto saltear (`media_time`) y cuánto dura (`segment_duration`, de la escala de la película a la del sonido); si algo no cuadra, no recorta nada. Igual que FFmpeg y mpv. Pruebas: `mp4_y_m4a_saltan_el_arranque_del_codificador_como_ffmpeg` (el tono empieza antes del cuadro 48, dura lo que dice el archivo) y pruebas del lector con archivos basura.
- **Mejoras nuevas para la función:**
  - **Llamadas tardías de la tarjeta** (`sonido_cpal.rs`): el hilo de audio mide (sin pedir memoria) el tiempo entre pedidos de la tarjeta; si pasa del margen (50 ms), el búfer pudo vaciarse. El programa lo **anota en el registro** como AVISO "Posible corte de sonido en Principal… (mayor espera X ms)" sin molestar en pantalla. Sirvió para comprobar que el único "clic" restante era de la grabadora y no de la salida.
  - **Que el equipo no se duerma** (`despierto.rs`): con el show abierto, `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED)` desde el hilo principal (registro: ENERGIA). Antes no había nada: en una escena larga con el proyector en negro o una imagen fija, Windows podía apagar la pantalla o suspender. **Verificado en la documentación de Microsoft:** no frena el protector de pantalla ni cerrar la tapa o el botón de apagado. Por eso, si hay **protector de pantalla configurado** (`SCRNSAVE.EXE` no vacío en `HKCU\Control Panel\Desktop` o en la directiva; `ScreenSaveActive` vale 1 aunque no haya ninguno), se **avisa** al abrir el show con cómo quitarlo. Comprobado sin administrador con `CallNtPowerInformation(SystemExecutionState)` (prueba `el_pedido_se_ve_en_el_estado_del_sistema` y línea en la función simulada); `powercfg /requests` pide administrador. La PC del usuario no tiene protector de pantalla.
  - **La Prueba de hardware suma la función simulada de 2 minutos** (paso nuevo, con los vídeos `pruebas/funcion/RRGGBB.mp4` que genera GitHub: color liso + tono AAC; la demo copia la misma carpeta `pruebas`). Así, en el teatro, un doble clic prueba el proyector real y la tarjeta real. Texto inicial: "unos 7 minutos" y que el ratón se mueve solo un momento. "No medido" ahora dice que la grabación se toma antes del driver.
- **Resultados en la PC del usuario** (GTX 1050 Ti, monitor 1920×1080 a pantalla completa, USB Audio CODEC 48 kHz):
  - Antes de los arreglos (1 min): imagen perfecta; sonido con clics cada 4 s (bucle a 44,1 kHz) y uno en la vuelta del vídeo con "salir desde aquí".
  - **Después (5 min, 105 GO):** imagen 12.104 muestras, **0 negros, 0 colores equivocados**; esquinas y barra de tareas siempre tapadas; ratón oculto; clic sin robar el teclado; **sonido grabado 304,8 s, 0 clics** (máxima 0,0017, umbral 0,0050); 0 llamadas tardías (máximo 24 ms); 0 GO ignorados; registro sin errores.
  - **Memoria de la proyección: 1.041 → 1.192 MB en 5 min.** Se agregó la medición cada 30 s y una prueba de 15 min para ver si se estabiliza.
  - **15 min, 322 GO:** imagen 36.052 muestras, 0 negros, 0 colores equivocados; esquinas 4.507 muestras bien; barra de tareas siempre tapada; sonido 905 s grabados, **0 clics**; 0 llamadas tardías; Windows con "no suspender" y "no apagar la pantalla" registrados. **Memoria de la proyección: sube y baja entre 900 y 1.280 MB sin tendencia** (segunda mitad: +1,3 MB por minuto) → no es una pérdida de memoria, son las reservas de los decodificadores. El motor queda en ~10 MB más la grabación de la prueba.
  3. **Tercer error real (hallado en los 15 min): un GO al vídeo que está al aire se quedaba esperando para siempre.** Secuencia: GO del cue 6 (vídeo) → elegir otra vez el cue 6 → GO: "todavía se está preparando" hasta Parar todo. Causa: lo que se ve no se vuelve a precargar, pero el GO exigía la precarga. Fácil de provocar en la Vista Estudio (clic en el cue al aire y GO). Arreglo (`Sesion::go`): si el cue elegido es el vídeo o imagen al aire, **vuelve a empezar** (la proyección lo salta a su inicio y su sonido arranca de nuevo; "salir desde aquí" no se aplica a lo que ya se ve); se anota "GO … (otra vez, desde su inicio)"; el estado lo muestra "Listo". `Sonando.duracion_visual` guarda la duración del vídeo para eso. **Además**, revisando: la proyección, al repetir el mismo vídeo, saltaba al segundo 0 del archivo y no al inicio del cue (si el cue tenía inicio propio, la imagen y el sonido se desfasaban); ahora salta al `start` con que se precargó (`seek … absolute+exact`). Prueba: `otro_go_al_video_que_esta_al_aire_lo_vuelve_a_empezar`.
  - El programa en una carpeta con tildes, eñe, espacios y paréntesis (`…\Técnico ñandú\Mis Programas (copia)\Otaconvert Demo\Otaconvert.exe`): `--comprobar` "No falta nada" (libmpv y Vulkan cargados desde ahí).
- **Lo que sigue sin poder probarse en la PC del usuario** (honesto): proyector o segunda pantalla real en modo Extender (cambio de orden de pantallas, conectar y desconectar), pantallas con distinta escala (125 %, 150 %), portátil con dos tarjetas gráficas sacando la imagen por la otra, desconectar la tarjeta de sonido USB en plena función, una mesa de sonido real y el cable, una función de 2 horas. La Prueba de hardware con la función simulada cubre gran parte de esto en el equipo del teatro.
- **Subido y comprobado (17/10/2026):** commit bd096b5, run 38999724166 **verde** (pruebas, ventana, vulnerabilidades; el "exit code 1" es la Prueba de hardware informativa en el equipo de GitHub sin GPU ni sonido). Demo nueva (`Otaconvert.exe` SHA-256 `2cff7eca…`) con `pruebas/funcion/` (4 vídeos con sonido); `--comprobar`: no falta nada; **30 s de función simulada con el exe de GitHub (MSVC): todo BIEN**, 0 negros, 0 clics. Dato a vigilar: la mayor espera entre pedidos de la tarjeta fue **45,8 ms** (margen 50 ms; sin corte), más que los 24 ms del exe local. Revisar con la Prueba de hardware en otros equipos antes de decidir el margen por defecto. Copiada a `CLOUDE - Herramientas Ota\Otaconvert-Demo-0.0.1` (reemplaza la anterior).
- **Hallazgo del entorno:** ver la última línea de la sección siguiente (herramientas de compilación en `%TEMP%`).

### Etapa 4C: experimento antes de decidir (17/10/2026)
- **Decisiones del usuario (17/10/2026, "sí a todo"):** encuadre y recorte en la 4C; todos los complementos de gráficos en vivo (sección 5c).
- **Verificado en documentación:**
  - libplacebo (motor de dibujo de mpv con `vo=gpu-next`), `docs/custom-shaders.md`: shaders propios con `//!HOOK OUTPUT` (después de mezclar, antes del paso final), `HOOKED_tex(pos)` lee la imagen en cualquier punto, parámetros `//!PARAM` con `//!TYPE DYNAMIC float` que mpv pasa con `glsl-shader-opts`.
  - Manual de mpv: `--glsl-shaders`, `--glsl-shader-opts=param=valor,…` (sin decir si se cambia en vivo); encuadre: `panscan` 0–1 (llenar recortando), `keepaspect` (no = estirar), `video-zoom` (en log2: −1 = la mitad), `video-pan-x/y` (fracción del tamaño de la imagen), `video-crop=AnchoxAlto+x+y` ("funciona con hwdec, a diferencia de lavfi-crop"), además `video-scale-x/y`, `video-align-x/y`, `video-margin-ratio-*`, `video-rotate` (con hwdec sin copia, solo de a 90°).
- **Experimento `--prueba-esquinas [--carpeta <vídeos>] [--pantalla N]`** (`crates/motor/src/prueba_esquinas.rs`): shader `OUTPUT` con la homografía inversa (8 números `h0…h7`, calculados en el programa a partir de las 4 esquinas; fuera de la forma, negro); imágenes BMP generadas (blanca; 4:3 mitad roja y mitad azul). Mide con GetPixel en la pantalla real.
- **Resultado en la PC del usuario (GTX 1050 Ti, 1920×1080 a pantalla completa): todo BIEN.** Shader cargado sin deformar = pantalla entera blanca (9/9); forma torcida (0,12,0,08 · 0,80,0,18 · 0,92,0,88 · 0,20,0,95): 111 puntos, 0 mal (también confirma la orientación: `HOOKED_pos` va de arriba a la izquierda); **mover las esquinas en vivo** con `glsl-shader-opts`: 66 cambios en 3 s, el centro nunca dejó de verse; **encuadre en vivo** con una imagen 4:3: Ajustar (bandas negras), Llenar (`panscan=1`), Estirar (`keepaspect=no`), Zoom (`video-zoom=-1`), Mover (`video-pan-x=0.5`), Recortar (`video-crop=720x1080+720+0`): los 6 BIEN; **vídeo H.264 1080p60 real** con la forma torcida 12 s (4 moviéndola): **d3d11va, 0 cuadros perdidos, 0 tarde**.
- Límites honestos: probado en una sola tarjeta gráfica; el borde de la forma es duro (escalones de 1 píxel; se puede suavizar 1–2 px); falta ver qué pasa en una tarjeta que no pueda cargar el shader (hay que detectarlo y avisar, mostrando sin deformar); el shader deforma **toda** la salida (también las bandas negras), que es lo que hace falta para mapear.
- **Plan propuesto para la 4C** (a confirmar por el usuario, toca la proyección y suma campos al archivo del show): ver la respuesta del 17/10/2026. **Aprobado por el usuario el 17/10/2026** ("Sí, empieza la etapa 4C con ese plan").

### Etapa 4C hecha (17/10/2026): encuadre y 4 esquinas
- **Archivo del show** (`crates/show/src/superficie.rs`, compatible con los shows viejos: si no se tocan, no se escriben):
  - Por cue de vídeo o imagen, `encuadre`: `modo` (`ajustar` · `llenar` · `estirar`), `zoom` (1 = normal, de 0,1 a 10), `mover_x`/`mover_y` (fracción de la imagen, ±2), `recorte_arriba/abajo/izquierda/derecha` (0 a 0,45 cada uno). `Encuadre::problema()` valida.
  - En el show, `superficie.esquinas`: las 4 esquinas del Proyector en fracciones de la pantalla (arriba-izq., arriba-der., abajo-der., abajo-izq.; de −1 a 2). `Superficie::problema()` rechaza esquinas cruzadas, aplastadas o fuera de rango; `homografia_inversa()` da los 8 números del shader. Más adelante pasarán al perfil de cada teatro (etapa 5); hoy viajan con el show.
- **Proyección** (`crates/motor/src/proyector.rs`):
  - Al arrancar escribe el shader (`SHADER`, el probado en el experimento) y el **patrón de prueba** (BMP 1920×1080: cuadrícula, bordes, diagonales, círculo y cada esquina de un color: roja arriba-izq., verde arriba-der., azul abajo-der., amarilla abajo-izq.) en `%LOCALAPPDATA%\Otaconvert\proyeccion\` (o `datos\proyeccion\` en la portable). Las 4 capas cargan el shader (`glsl-shaders`); si no se puede escribir, se avisa y se proyecta sin esquinas.
  - Orden `precargar` con `encuadre`: antes de cargar pone **todas** las opciones de encuadre (`panscan`, `keepaspect`, `video-zoom` = log2(zoom), `video-pan-x/y`, `video-crop` vacío) para no heredar las del cue anterior; ya cargado, aplica el recorte en píxeles (`video-crop`, sabiendo `video-params/w,h`).
  - Orden `esquinas {h}`: `glsl-shader-opts` en todas las capas (y en el patrón).
  - Orden `patron {si}`: el reproductor del patrón **se crea solo al pedirlo y se suelta al quitarlo** (no ocupa memoria en la función; con una capa fija la memoria de la proyección subía ~200 MB). Va encima de todo; Parar todo (telón) lo tapa.
  - **Si la tarjeta gráfica no puede con el shader**: comprobado con un shader roto a propósito, mpv lo apaga ("Failed executing hook, disabling") y **la imagen sigue saliendo sin deformar** (no en negro). El reproductor ahora lee los mensajes de error de mpv (`pedir_mensajes`/`tomar_mensajes`, evento 2 de mpv) y la proyección avisa `aviso {codigo: "esquinas"}` una vez → la sesión muestra "La tarjeta gráfica de este equipo no pudo usar las 4 esquinas: la imagen sale sin deformar…" y lo anota (AVISO).
- **Sesión**: `cambiar_esquinas` (solo en Edición; valida; manda `Esquinas`; marca cambios sin guardar), `mostrar_patron` (solo en Edición; al pasar a Función se quita solo; registro MAPEO), `editar_cue` acepta `encuadre_modo`, `zoom`, `mover_x`, `mover_y`, `recorte_*` (solo vídeos e imágenes; si el cue estaba precargado con el encuadre viejo, se suelta y se vuelve a precargar). Al conectar la proyección y **tras una caída** se mandan las esquinas (y el patrón si estaba). Estado nuevo: `esquinas`, `patron` y el `encuadre` de cada cue. Órdenes del canal: `esquinas {esquinas: [[x,y]×4]}`, `patron {si}`.
- **Ventana**:
  - En el panel de un vídeo o imagen (Edición), sección **Encuadre**: cómo entra (Ajustar / Llenar / Estirar), Zoom 25–400 %, Mover a los lados y arriba/abajo (±100 %), Recortar bordes (4 casillas en %), "Volver al encuadre normal", y la nota "Se ve en el proyector con el próximo GO de este cue". Con consejos (`encuadre`, `zoom`, `mover`, `recortar`). Las casillas de número no disparan GO ni mueven la espera al escribir.
  - **Vista → Mapeo del proyector…** (solo Edición; en Función explica que se cambia en Edición y, si se pasa a Función con el mapeo abierto, se cierra): un recuadro que representa el proyector (con margen para sacar una esquina un poco afuera), la forma y 4 esquinas de los mismos colores que el patrón, que se **arrastran**; **flechas** = 1 píxel de un proyector 1920×1080, **Mayús + flechas** = 10; teclas **1 a 4** eligen la esquina; posición en %; casilla **Patrón de prueba en el proyector**; **Volver a la pantalla entera**; **Listo**. Al cerrar (Listo, Esc o clic afuera) se quita el patrón: el público nunca lo ve. Mientras se arrastra se manda como mucho cada 40 ms. Consejos `mapeo` y `patron`; ayuda `mapeo`.
  - Textos es/en nuevos (`encuadre_*`, `modo_*`, `zoom`, `mover_*`, `recortar`, `rec_*`, `mapeo*`, `esquina_0…3`). En español el consejo del zoom se titula "Acercar o alejar (zoom)" (la prueba exige títulos distintos en cada idioma).
- **Pruebas**: archivo (`encuadre_y_esquinas_van_y_vuelven_y_los_shows_viejos_siguen_abriendo`), superficie (homografía, figuras y encuadres imposibles), sesión (`esquinas_patron_y_encuadre_llegan_a_la_proyeccion`: esquinas al conectar y tras una caída, Función bloquea y quita el patrón, re-precarga con el encuadre nuevo, valores imposibles rechazados, estado y aviso). **102 pruebas en verde.**
- **`--prueba-mapeo [--pantalla N]`** (proceso de proyección de verdad, como el programa): **en la PC del usuario todo BIEN**: imagen sin deformar 9/9; esquinas por la orden del programa 111 puntos, 0 mal; patrón con cada esquina de su color en su lugar deformado (4/4); al quitarlo vuelve la imagen; encuadre Ajustar, Llenar, Zoom 50 % y Recortar BIEN; un cue normal después de otros con encuadre sale normal; 0 avisos. Sumada a la **Prueba de hardware** (paso "Mapeo").
- **Función simulada de 5 min con la 4C** (110 GO): imagen 12.107 muestras sin negros ni colores equivocados, sonido 314 s sin clics, 0 llamadas tardías, registro sin errores. Memoria de la proyección: entre 1.220 y 1.410 MB sin tendencia (antes de la 4C, 900–1.280): el shader suma algo de memoria por capa (dato a vigilar en equipos modestos).
- **Probado en el panel del navegador**: encuadre desde el panel (Zoom 150 % y Llenar quedan en el show), mapeo con flechas (Mayús+← ×5 y Mayús+↓ ×3 → la esquina 2 queda en 97,4 % · 2,8 %), patrón encendido y apagado al cerrar, guardado: el `.otashow` lleva `encuadre` y `superficie`.
- **Subido y comprobado:** commits 16aedf2 y b46f091, run 39045837816 **verde**. Demo nueva (`Otaconvert.exe` SHA-256 `471a4069…`): `--comprobar` no falta nada; **`--prueba-mapeo` con el exe de GitHub (MSVC): todo BIEN** (11/11). Copiada a `CLOUDE - Herramientas Ota\Otaconvert-Demo-0.0.1` sin la carpeta `datos`.
- Límites honestos: el borde de la forma es duro (escalones de 1 píxel); 1 píxel de las flechas supone un proyector de 1920×1080; las esquinas son del show (pasan al perfil del teatro en la etapa 5); la caja «al aire» de la Vista Estudio no muestra el encuadre ni la deformación (es una copia aproximada); falta probar con un proyector real.

- **Hallazgo del entorno:** la limpieza automática de Windows vació parte de `%TEMP%\otr` (las librerías descomprimidas de cargo): falló la compilación con "file not found for module ProcessStatus". Se arregló haciendo que cargo vuelva a descomprimir desde su copia (`registry/src` renombrada). Conviene mover las herramientas de compilación fuera de `%TEMP%`.

## 12. Etapa 5: editar el show, ajustes y salidas (17/10/2026)

Pedido del usuario: "Sí, sigue con la etapa 5." Repartida en cuatro partes que se pueden probar por separado:
- **5A Editar el show**: agregar cues (arrastrar o «Agregar cues…», copiando lo de fuera a `Medios/`), quitar y reordenar, número libre, inicio y fin, deshacer y rehacer.
- **5B Ajustes del show y salidas**: rampas con consejos, protección contra doble GO, Fundir todo, volumen maestro; elegir la salida de sonido y la pantalla del proyector desde la ventana; perfiles de teatro (también las 4 esquinas por teatro).
- **5C**: panel Sonando (parar o fundir un solo cue), medidor de nivel, luz verde (comprobación previa), botonera, «Ver antes» por Auriculares.
- **5D**: onda y miniatura del proyector.
- Riesgo avisado: reordenar mientras algo suena. Se evitó identificando cada cue por un **número interno** (`id`) en vez de su lugar en la lista.

### Etapa 5A hecha (17/10/2026)
- **Sesión** (`crates/servidor/src/sesion.rs`):
  - Cada cue tiene un `id` único. Al abrir un show, los repetidos o en 0 se renumeran.
  - La proyección recibe el `id` (así un cambio de orden no confunde lo que se precargó).
  - `reindexar` reacomoda por `id` todo lo que estaba por lugar: preparados, errores, sonando, precargas, miniaturas y el cue en espera.
  - `agregar_cues(archivos, después_de)`: valida cada ruta y tipo y el show entero antes de cambiar nada. Los nuevos entran detrás del cue en espera con los números siguientes al mayor. Registro AGREGAR.
  - `quitar_cue`: no se puede si está sonando; el archivo no se borra. Registro QUITAR.
  - `mover_cue`.
  - `editar_cue` acepta `numero` (1 a 20 letras, p. ej. 1.5 o 2A), `inicio_ms` y `fin_ms` (null = hasta el final). Si cambia inicio, fin o bucle, se vuelve a preparar.
  - **Deshacer / rehacer**: hasta 100 pasos, con una copia del show antes de cada cambio. Varios movimientos seguidos de las esquinas cuentan como uno.
  - Todo esto solo en Edición. En Función se rechaza con aviso.
  - Si cambió el orden mientras se preparaba un audio, ese resultado se descarta y se vuelve a pedir.
- **Traer archivos** (`crates/servidor/src/importar.rs`):
  - Rechaza rutas de red, relativas, que no existen o que no son audio, vídeo o imagen.
  - Lo que ya está dentro de la carpeta del show se usa tal cual.
  - Lo de fuera se copia a `Medios/`. Si ya hay uno con el mismo nombre y tamaño, se reutiliza; si es distinto, se guarda como «nombre (1).ext» y nunca se pisa.
  - Se copia a un `.copiando` y después se renombra. La copia va en otro hilo: GO y Parar todo siguen funcionando mientras se copia.
  - Órdenes del canal: `importar {rutas, despues_de}`, `buscar_archivos {despues_de}` (el motor abre el diálogo de Windows `GetOpenFileNameW`, varios a la vez, en `crates/motor/src/dialogo.rs`), `medios_carpeta` (responde `{tipo: "medios", lista}`), `agregar {archivos, despues_de}`, `quitar {cue}`, `mover {cue, hasta}`, `deshacer`, `rehacer`.
  - El estado trae `puede_deshacer` y `puede_rehacer`, y por cue `fin_s` y `largo_s`.
  - Quitar avisa «Se quitó «X»… Ctrl+Z lo vuelve a poner» (sin confirmación, porque se puede deshacer); deshacer y rehacer borran ese aviso.
- **Arrastrar desde el Explorador** (`crates/ventana/src/main.rs`):
  - `with_drag_drop_handler` de wry 0.57: wry apaga el arrastre propio de WebView2 y recibe las rutas.
  - La ventana avisa a la página con `otaArrastre(true/false)` y `otaSoltar([...])`. Las rutas van como texto JSON escapado: un nombre de archivo no puede cortar el código (prueba `un_nombre_de_archivo_no_corta_el_codigo`). Máximo 1.000 archivos.
  - Mientras se arrastra, una capa dice «Suelta para agregar al show». En Función dice que no se puede.
- **Ventana**:
  - **Menú Edición**: Deshacer (Ctrl+Z), Rehacer (Ctrl+Y o Ctrl+Mayús+Z), **Agregar cues…** (Ctrl+I), Eliminar cue (Supr), Subir y Bajar cue (Alt+↑/↓). En gris en Función o cuando no hay qué hacer. Copiar y Pegar siguen «Próximamente».
  - Ctrl+Z dentro de un campo de texto deshace lo escrito, no el show.
  - Ventanita **Agregar cues**: medios de la carpeta del show con casillas, marcando los que «ya está en el show»; «Buscar archivos…»; «Agregar N».
  - **Reordenar arrastrando una fila** en Edición, con una línea de color donde cae. Usa eventos del puntero, no el arrastre del navegador, que queda para los archivos.
  - Lista vacía: «Arrastra aquí…» y botón «Agregar cues…».
  - Panel: **Número**, **Inicio y fin (segundos)** («Desde» y «Hasta»; vacío = el final; coma o punto; fuera de rango avisa y no manda nada).
  - Consejos nuevos: `agregar`, `numero` e `inicio_fin`, en es/en, con ejemplos de escena. Atajos nuevos en Ayuda → Atajos de teclado.
- **Pruebas**: `servidor/tests/edicion.rs` (8: agregar, quitar, mover, número, inicio y fin, deshacer y rehacer, Función bloquea, lo que suena sigue bien al reordenar) y `canal.rs` `arrastrar_archivos_de_afuera_los_copia_y_agrega` (lo que no es medio se rechaza; se copia a `Medios/Timbre (1).wav` sin pisar el que había; el original queda; sin temporales; deshacer; Función bloquea). **113 pruebas en verde.**
- **Probado en el panel del navegador** (copia del show de ejemplo):
  - Ctrl+I muestra los medios de la carpeta; agregar 2 los pone detrás del cue en espera.
  - Alt+↓ dos veces; Supr con su aviso; Ctrl+Z lo devuelve.
  - Arrastrar filas hacia arriba y hacia abajo; Ctrl+Z.
  - Número «2B», Desde «1,5», Hasta «4»; un tiempo imposible avisa y no se manda.
  - Soltar 2 archivos de otra carpeta: se copian a `Medios/` y quedan como cues.
  - En Función todo el menú Edición queda en gris y Supr y Alt+↓ no cambian nada.
- Límites: el arrastre real desde el Explorador solo se puede probar en la ventana propia, no en el panel del navegador; el cue nuevo lleva el nombre del archivo copiado (p. ej. «Timbre (1)»).
- **Subido y comprobado:** commit 6281531, run 39048636745 **verde** (compilación, pruebas, ventana y vulnerabilidades). Demo nueva copiada a `CLOUDE - Herramientas Ota\Otaconvert-Demo-0.0.1` (`Otaconvert.exe` SHA-256 `e8e8d7a7…`; `--comprobar`: no falta nada). Solo se reemplazaron los archivos del programa: las carpetas `Show de ejemplo` y `datos` quedaron como el usuario las dejó.

### Etapa 5B, parte 1: Ajustes del show (21/10/2026)
- **Sesión:** `Sesion::cambiar_ajustes(cambios)` (orden del canal `ajustes {cambios}`):
  - Claves: `rampa_inicio`…`rampa_fin` (0–1000 ms), `doble_go_ms` (0–5000), `fundir_todo_ms` (0–60 000) y `volumen_maestro_db` (−60 a +12).
  - Valida todo antes de cambiar nada y rechaza claves desconocidas.
  - Solo en Edición.
  - Se aplica **en vivo** al mezclador sin cortar lo que suena (`CambiarAjustes`; el maestro cambia con una rampa de 50 ms).
  - Se puede **deshacer**: los cambios seguidos cuentan como uno, y deshacer vuelve a mandar los ajustes al mezclador.
  - Se anota AJUSTES en el registro.
  - El estado trae `ajustes`.
- **Archivo del show:** valida además doble GO ≤ 5 s y Fundir todo ≤ 60 s.
- **Ventana: Edición → Ajustes del show…**
  - Las 7 rampas en ms, cada una con su consejo.
  - Protección contra doble GO, Duración de Fundir todo (en s) y Volumen maestro (deslizador).
  - «Volver a los valores recomendados» y «Listo».
  - En Función explica que se cambia en Edición, y si se pasa a Función con la ventanita abierta, se cierra.
- **Consejos de las rampas: una sola copia.** Los textos están en `crates/audio/src/consejos.rs`; el servidor los agrega a los de la ventana al servirla (`textos()`, claves `rampa_inicio`…`rampa_fin`), así nunca se desfasan.
- **Consejos nuevos (es/en):** `ajustes_show`, `doble_go`, `fundir_todo_dur` y `volumen_maestro`.
- **Pruebas:**
  - `ajustes_del_show_en_vivo_validados_y_con_deshacer`: 5 ms = 240 cuadros a 48 kHz; el maestro a −6 dB ≈ 0,501 con rampa; Fundir todo usa la duración nueva.
  - 7 pedidos malos no cambian nada; en Función se rechaza.
  - Deshacer y rehacer funcionan, y se guardan con el show.
- **Probado en el panel del navegador:** la ventanita, los valores, un valor fuera de rango (avisa y no manda nada), el consejo de la rampa de inicio, «Volver a los valores recomendados» y el registro AJUSTES con el maestro a −6 dB.

### Etapa 5B, parte 2: elegir salida de sonido y pantalla desde la ventana (21/10/2026)
- **Plan aprobado por el usuario:**
  - Solo en Edición y con nada sonando ni a la vista.
  - Lo nuevo se abre **antes** de soltar lo viejo; si falla, queda lo que estaba y se avisa.
  - Se recuerda **en este equipo**, no en el show.
  - Si falta, se avisa fuerte; nunca se cambia en silencio.
- **Motor** (`crates/motor/src/equipo_salidas.rs`):
  - `lista_json()`: tarjetas (id estable de cpal, nombre, predeterminada, Hz, canales) y pantallas (dispositivo, nombre, tamaño, Hz, principal, GPU).
  - `Guardado` en `equipo.json` dentro de la carpeta de datos (`%LOCALAPPDATA%\Otaconvert` o `datos\` en la portable). Guarda Principal y Auriculares (id + nombre) y la pantalla (dispositivo + nombre, o «vista»). Se escribe a un temporal y se renombra; un archivo dañado = las predeterminadas.
  - `buscar_pantalla`: busca por dispositivo y nombre; si Windows reordenó las pantallas, por el nombre del proyector si es único.
  - `abrir_sonido`: abre Principal y Auriculares juntas. Si Auriculares no está o trabaja a otra frecuencia, queda sin usar y se avisa.
- **Al abrir el show:** se usa lo pedido por línea de comandos o, si no, lo recordado.
  - Si la tarjeta recordada no está: suena por la predeterminada con el aviso «¡Atención! La tarjeta elegida para Principal («…») no está…» (consola, ventana y registro).
  - Si la pantalla recordada no está: aviso y la proyección va donde la encuentre.
  - `sumar_aviso` junta los avisos del arranque para que ninguno tape a otro.
- **Cambio en vivo** (`show_cmd::cambiar_salida`, en el hilo principal, donde viven las tarjetas):
  - La ventana manda `elegir_salida {que: principal|auriculares|pantalla, id}`. El canal lo atiende en otro hilo sin tener la sesión tomada y espera la respuesta hasta 30 s.
  - Se comprueba `Sesion::se_pueden_cambiar_salidas` antes y otra vez después de abrir lo nuevo.
  - Sonido: se abren de nuevo Principal y Auriculares con lo pedido; `conectar_sonido` (si cambió la frecuencia, los audios se vuelven a preparar solos y se avisa); recién entonces se sueltan las tarjetas viejas.
  - Pantalla: se abre la proyección nueva y se reemplaza la vieja (`conectar_proyector`).
  - Registro: SALIDA.
  - Estado: `equipo` (lo elegido ahora) y `salidas_respondidas` (contador para que la ventana sepa que su pedido terminó). Orden `equipo_lista` → `{tipo: "equipo", salidas, pantallas}`.
- **Ventana:** en Ajustes del show, sección **«Salidas de este equipo»**:
  - Sonido Principal (la sala), Sonido Auriculares (escucha previa, con «Sin usar») y Pantalla del proyector (o «Vista previa en una ventana»).
  - Marca la predeterminada de Windows y la frecuencia, «(no está conectada)», «Cambiando…» y «Volver a buscar».
  - Consejos `salida_principal`, `salida_auriculares` y `salida_pantalla`.
- **Probado en la PC del usuario** (sin dar GO, nada sonó):
  - Principal USB → NVIDIA, Auriculares → USB, Auriculares → Sin usar, Principal → USB: cada cambio respondió en menos de 0,25 s, con el registro SALIDA.
  - `equipo.json` guardado.
  - Tarjeta y pantalla guardadas que no existen: los dos avisos, y suena por la predeterminada.
  - Con una imagen a la vista, el cambio se rechaza con «primero para todo (Esc)» y la salida no cambia.
- **Errores hallados y corregidos al probar:**
  1. La lista se redibujaba antes de leer lo elegido y mandaba la tarjeta de antes.
  2. «Cambiando…» quedaba trabado si no cambiaba nada visible (de ahí el contador).
  3. Abrir Ajustes con otra ventanita pendiente de cerrarse fallaba (ahora se cierra la anterior antes de preparar la nueva, también en Agregar cues).
- **Prueba nueva:** `las_salidas_solo_se_cambian_en_edicion_y_sin_nada_sonando`.
- **Sin probar:** cambiar la proyección a una pantalla completa real (el usuario tiene un solo monitor: taparía su pantalla). Queda para el teatro.
- **Pendiente de la 5B:** perfiles de teatro con las 4 esquinas por teatro. Toca el formato del show: **avisar antes** (regla de cambios riesgosos).

### Tema «Madera de fresno» (21/10/2026)
- **Pedido del usuario:** un modo de estética opcional que luzca como madera de fresno, acogedor y profesional, sin cambiar nada de lo existente. **Aprobado al ver las capturas** («Perfecto el tema»).
- **Dónde:** Vista → **Madera de fresno** (`data-tema="fresno"`), junto a Modo noche, Modo día y Alto contraste. Se guarda como preferencia de la ventana.
- **Colores:** fresno claro (`--bg #CDB491`) con veta suave hecha con degradados CSS (sin imágenes). Paneles como madera cepillada (`--card #F1E6D3` con veta leve, brillo arriba y sombra suave). Texto color nogal (`#2E2216`). Acento verde pino (`#2F6A55`) para GO y lo elegido. Parar todo sigue rojo.
- **Consejo `modo_fresno`:** para ensayos y preparación con luz; en cabina a oscuras, mejor Modo noche (el fondo claro encandila).

### Etapa 5C, parte 1: panel Sonando (21/10/2026, trabajando solo)
- **Sesión:** `fundir_uno(id)` funde solo lo que suena con ese `id`, en la duración de Fundir todo. `parar_uno(id)` lo para con la rampa de parada; si era la imagen o el vídeo que se ve, el proyector queda en negro. Los dos valen también en Función. Mayús+F ahora usa `fundir_uno`. Registro FUNDIR/PARAR «solo Cue N…».
- **Canal:** órdenes `fundir_uno {id}` y `parar_uno {id}`; cada cosa de `sonando` lleva su `id`.
- **Ventana:** botones **Fundir** y **Parar** (este en rojo) en cada cosa del panel «En vivo» y de la caja «al aire» de la Vista Estudio. Se atienden al **apretar** (`pointerdown`), no al soltar: el panel se redibuja varias veces por segundo y un clic se podría perder. Consejo `sonando_uno` es/en. En el Modo Ayuda no hacen nada.
- **Prueba:** `parar_o_fundir_un_solo_cue_desde_sonando`.
- **Probado en el programa**, con el volumen maestro en −60 dB para que no sonara nada en la PC del usuario mientras no estaba: música de sala en bucle + imagen. Fundir en la música → «Fundiendo · −0:03», y la imagen sigue. Parar en la imagen → sale, y en la Vista Estudio el proyector queda en «Negro». En el registro: FUNDIR y PARAR «solo…».

### Etapa 5C, parte 2: luz verde (21/10/2026, trabajando solo)
- **`crates/servidor/src/comprobar.rs`** solo **lee**: no cambia nada del show ni de lo que suena.
  - `Pedido`: copia del show tomada con la sesión. La revisión se hace después, sin tener la sesión tomada.
  - `revisar(pedido, progreso)`, cue por cue:
    - archivo que falta o ruta mala;
    - audio (o sonido de un vídeo) que no se puede reproducir;
    - pico sobre 0 dB contando el volumen del cue;
    - partes dañadas;
    - **arranque de golpe a mitad de onda** (rampa de inicio 0 y sin fundido inicial; ver la regla abajo);
    - cue para Auriculares sin auriculares elegidos.
  - Además: salida de sonido abierta, proyección abierta si hay vídeos o imágenes, cambios sin guardar y errores que la sesión ya conoce.
  - **Regla del clic:** el primer valor del cue está lejos de 0 **y** es más de 3 veces el paso al valor siguiente. Un tono que nace en 0 sube de a poco (0,029 → 0,10) y no se marca; un arranque en 0,5 sí.
  - `a_json`: la luz es el peor nivel; la lista va de lo más grave a lo menos, en el idioma de la ventana.
- **Motor** (`equipo_salidas::comprobar_equipo`): componentes que faltan (libmpv, Vulkan, WebView2), protector de pantalla, batería (enchufado o no), y show o programa en un pendrive (`GetDriveTypeW` = `DRIVE_REMOVABLE`; feature `Win32_Storage_FileSystem`).
- **Orden `comprobar`:**
  - Solo en Edición: decodifica todos los audios, no se hace en plena función.
  - Una a la vez, en otro hilo, avisando el avance como mucho 4 veces por segundo.
  - El resultado (`comprobacion`: luz, hora y puntos) se borra al cambiar el show o las salidas.
  - Registro COMPROBAR con la cuenta de problemas y avisos.
- **Ventana:**
  - Luz en la cabecera: Sin comprobar (gris), Todo listo (verde), Revisar avisos (amarilla), Hay problemas (roja), o «Comprobando… n de m».
  - Al tocarla, la ventanita **«Antes de la función»** con la lista y **Comprobar ahora** (en gris en Función).
  - Textos `luz_*`, consejo `luz_verde` y ayuda `luz` (Modo Ayuda) en es/en.
- **Pruebas** (`tests/comprobar.rs`, 4):
  - un show sano no tiene nada que revisar;
  - encuentra archivo que falta, Opus, imagen que falta, proyección cerrada, pico, Auriculares, clic y cambios sin guardar;
  - la rampa al iniciar quita el aviso de clic;
  - sin sonido es un problema;
  - la luz es el peor nivel.
  - **120 pruebas en verde.**
- **Probado en el programa:**
  - El show de prueba (10 cues) se revisó en menos de 0,25 s y salió **amarillo**: detectó un caso real, el cue «Viento nocturno» con Desde = 1,5 s empieza a mitad de onda.
  - Al ponerle un fundido inicial, la luz volvió a «Sin comprobar»; al comprobar de nuevo quedó solo «cambios sin guardar».
  - Tras Ctrl+S: **verde, «Todo listo»**.
- **Pendiente:** revisar que el proyector esté en modo Extender y con la resolución correcta; avisar de las actualizaciones de Windows (sección 7).

### Botonera de sonidos (21/10/2026, trabajando solo)
- **No cambia el formato del show:** usa `Show.botonera` (`PaginaBotonera` / `BotonSonido`), que el archivo ya tenía desde la etapa 1.
- **Sesión** (`crates/servidor/src/botonera.rs`, módulo hijo de `sesion.rs`):
  - **Siempre en memoria:** cada sonido de la botonera se prepara después de los cues en espera, en el mismo hilo preparador (`siguiente_boton_a_preparar` / `resultado_boton`). La clave es archivo + bucle (el empalme del bucle se prepara distinto). Al cambiar de tarjeta se vuelven a preparar, y se sueltan los que ningún botón usa.
  - **`tocar_boton(página, botón)`**, en Edición y en Función: suena encima de lo que haya, no mueve el cue en espera y no cuenta como GO (no activa la protección contra doble GO). Si se toca mientras suena, según `al_repetir`:
    - **parar** (por defecto): lo baja con su fundido final;
    - **reiniciar**: lo para con la rampa y lo arranca de nuevo;
    - **superponer**: suena otra copia.
  - Auriculares sin auriculares elegidos → suena por Principal.
  - Registro BOTON.
  - **`tecla_boton`**: una letra salvo P, F y H (las usa el programa), un número o F1–F12. No distingue mayúsculas.
  - **Parar todo** la para; **Fundir todo** la funde; **Mayús+F no la toca** (decisión de la sección 5b); **Pausa** la pausa y corre su fin.
  - **`editar_botonera(páginas)`**, solo en Edición. Valida todo antes de cambiar nada:
    - hasta 20 páginas de 64 botones;
    - nombre de 1 a 60 letras;
    - color `#RRGGBB`;
    - el sonido tiene que ser un audio dentro de la carpeta;
    - volumen de −60 a +12 dB y fundidos de hasta 10 min;
    - salida Principal o Auriculares;
    - teclas válidas y sin repetir.
    Se puede deshacer; registro BOTONERA. Lo que sonaba sigue sonando, pero ya no se puede tocar de nuevo para pararlo (los botones pudieron cambiar de lugar): Parar todo lo para.
  - Estado: `botonera` con preparación, error y si suena cada botón.
- **Canal:** órdenes `boton {pagina, boton}`, `boton_tecla {tecla}` y `botonera {paginas}`.
- **Ventana:**
  - Panel **Botonera** debajo de la lista de cues, oculto por defecto. Se muestra con **Vista → Botonera ▸ Oculta / Cuadrícula / Lista** (preferencia de la ventana).
  - Pestañas de página. Cada botón muestra su franja de color, nombre, tecla y estado (Preparando… / No se puede reproducir), y se ilumina mientras suena.
  - Se toca al apretar (`pointerdown`), porque el panel se redibuja varias veces por segundo. En el Modo Ayuda no suena.
  - Las teclas de los botones funcionan también en Función, nunca con una ventanita abierta ni escribiendo.
  - **Edición → Botonera…** (en Función explica que se arma en Edición):
    - Páginas: agregar, renombrar, quitar.
    - Botones: nombre, sonido (los audios de la carpeta del show), color, tecla, volumen, salida, fundido inicial y final, «Si se toca mientras suena», bucle, subir, bajar y quitar.
    - «Agregar botón», «Cancelar» y «Guardar». Antes de mandar revisa lo mismo que el motor y avisa.
  - Consejos nuevos: `botonera`, `botonera_vista`, `boton_color`, `boton_tecla`, `boton_repetir` (los demás reusan los del cue). Ayuda `botonera`. Todo en es/en.
- **Pruebas** (`tests/botonera.rs`, 4):
  - tocar otra vez: parar con fundido de 500 ms = 24 000 cuadros, superponer 2 copias, y reiniciar por su tecla en Función;
  - no mueve el cue en espera y el GO sigue andando; Parar todo la para; un archivo que falta avisa;
  - Fundir todo la funde y Mayús+F no;
  - editar: 9 pedidos malos no cambian nada, en Función se rechaza, F2 funciona, deshacer;
  - teclas válidas.
  - **124 pruebas en verde.**
- **Probado en el programa** (volumen maestro −60 dB, no sonó nada):
  - Aplausos se ilumina al tocarlo y se apaga al tocarlo otra vez.
  - Con el editor le puse la tecla 1 y la tecla 1 lo toca; Esc lo para.
  - Arreglos de diseño hechos al verlo: la casilla del bucle quedaba mal puesta y no se veía qué página estaba elegida.
- **Pendiente:** la comprobación previa todavía no revisa los sonidos de la botonera (picos, archivos que faltan); el panel no se puede mover (llega con Vista → Personalizar, Fase 2).

### Onda del audio (etapa 5D, 21/10/2026, trabajando solo)
- **Motor:**
  - `Sesion::pista_para_onda(i)` da el audio ya preparado del cue.
  - `sesion::picos_de_onda(pista, columnas)` da el pico de cada parte (0–1000, todos los canales).
  - La orden `onda {cue}` la calcula en otro hilo, sin tener la sesión tomada (no frena el GO), y responde a las ventanas `{tipo: "onda", cue, archivo, total_s, picos}` con 800 columnas.
  - Va por el canal WebSocket: no hizo falta abrir la CSP de la página a otras peticiones.
- **Ventana:**
  - En el panel de un cue de audio (Edición), la **Onda** reemplaza la nota «Próximamente».
  - Dibuja en color la parte que suena (entre Desde y Hasta) y en gris el resto, con dos líneas en los puntos.
  - Un archivo suave se agranda hasta 8 veces para que se vea su forma.
  - **Clic** = Desde, **Mayús + clic** = Hasta. No marca si el cue está bloqueado porque suena. Se puede deshacer.
  - Mientras el audio no está preparado dice «La onda aparece cuando el audio está preparado». Se guarda por archivo, así no se vuelve a pedir.
  - Consejo `onda` es/en.
- **Prueba:** `la_onda_son_los_picos_del_audio` (sin preparar no hay; el tono de prueba da picos de 450 a 550 sobre 1000).
- **Probado en el programa:** la música de sala se dibujó. Clic al 25 % → Desde 1,99 s; Mayús + clic al 75 % → Hasta 5,99 s; la duración pasó a 0:04.

### «Ver antes» por Auriculares (etapa 5C, 22/10/2026)
- **Sesión:** `ver_antes(i)` hace sonar el cue (audio o sonido de un vídeo) **solo por Auriculares**, como el «Audition» de QLab.
  - Usa `iniciar_voz_en(…, Destino::Auriculares)`, con el volumen, el inicio y fin, los fundidos y el bucle del cue.
  - No toca la sala, el proyector, el cue en espera, «Sonando» ni la protección contra doble GO.
  - Tocarlo otra vez lo para; escuchar otro cue para el anterior. Vale en Edición y en Función.
  - **Sin auriculares elegidos no suena por ningún lado** y avisa cómo elegirlos. Una imagen o un vídeo mudo: aviso.
  - Parar todo y Fundir todo también la paran. Registro ESCUCHA.
  - Estado `escucha` = el cue que se escucha, o null.
- **Canal:** orden `ver_antes {cue}`.
- **Ventana:** botón **🎧 Escuchar por auriculares** / **■ Parar la escucha**, que se atiende al apretar. Está en el panel del cue (Edición), en el próximo cue del panel «En vivo» (Función) y debajo de la vista previa de la Vista Estudio. Consejo `escucha` es/en.
- **Pruebas** (`tests/escucha.rs`, 2):
  - solo manda órdenes a Auriculares, no aparece en «Sonando», no mueve el cue en espera y el GO sigue;
  - tocar otra vez para; imagen = error; Parar todo la para;
  - sin auriculares no se manda nada a ninguna salida.
  - **128 pruebas en verde.**
- **Probado en el programa** (volumen maestro −60 dB):
  - sin auriculares, el aviso;
  - con NVIDIA como Auriculares: «■ Parar la escucha», «0 sonando» en la sala y, al volver a tocar, se para;
  - registro: ESCUCHA «Cue 1 … por Auriculares» y «fin».
- **Subido y comprobado (22/10/2026):** todo lo de la etapa 5B, 5C, 5D, la botonera y el tema fresno (16 commits hasta 2bb864c). Run 39623475936 **verde** (compilación, pruebas, ventana y vulnerabilidades). Demo nueva en `CLOUDE - Herramientas Ota\Otaconvert-Demo-0.0.1` (`Otaconvert.exe` SHA-256 `04f4c05b…`; `--comprobar`: no falta nada). Solo se reemplazaron los archivos del programa: `Show de ejemplo` y `datos` quedaron como estaban.

### Medidor de nivel (etapa 5C, 22/10/2026)
- **Riesgo avisado y aprobado («Procede con el 1»):** toca el motor de audio. Para hacerlo seguro:
  - solo se **mide**;
  - va en la **misma pasada** que ya hacía el mezclador sobre la salida (el último seguro de 0 dB);
  - sin pedir memoria ni esperar a nadie;
  - se lee desde otro hilo.
- **Motor de audio** (`mezclador.rs`):
  - `Estadisticas.pico: [AtomicU32; 2]`: el pico del lado izquierdo y del derecho, en bits de `f32`. Para números positivos el orden de los bits es el del valor, así que sirve `fetch_max`.
  - Se mide **después del limitador**, o sea lo que sale de verdad. En mono, los dos lados son iguales.
  - El último seguro sigue cubriendo **todas** las muestras, también un cuadro incompleto.
  - `tomar_picos()` devuelve el pico desde la última lectura y vuelve a 0.
  - Prueba `el_medidor_ve_el_pico_de_lo_que_sale` (tono a 0,5 → 0,5; leer lo vuelve a 0; volumen a la mitad → 0,25; mono).
  - **`sin_memoria` y el estrés siguen en verde.**
- **Sesión y motor:**
  - `Sonido::niveles()` tiene un valor por defecto vacío, así los sonidos falsos de las pruebas no cambian.
  - `SonidoCpal` da el pico de Principal y de Auriculares, y si el limitador actuó (cuadros limitados nuevos).
  - `Sesion::tomar_niveles()` lo pasa a dB (−90 = silencio).
  - Un hilo «niveles» manda `{tipo: "niveles", principal: {db: [i, d], limitando}, auriculares}` **10 veces por segundo**, aparte del estado. Si todo está en silencio y ya se avisó, no manda nada.
- **Ventana:**
  - En la cabecera, **Sala** (y **Auris** si hay auriculares): dos barras finas, izquierda arriba y derecha abajo, de −60 a 0 dB.
  - Verde hasta −18, amarillo hasta −6 y rojo arriba.
  - Caída suave de 30 dB por segundo y una marca que recuerda el pico 1,5 s.
  - **LIM** se prende en rojo cuando el limitador está bajando el volumen.
  - Si no llegan datos (silencio), las barras caen solas.
  - Consejo `medidor` y ayuda `medidor` (es/en).
- **Probado en el programa:**
  - Con la música de sala a volumen maestro −60 dB (no sonó nada), llegaron unas 10 mediciones por segundo con −81,6 dB, lo esperable para una música suave −60 dB.
  - Se corrigió el redondeo (salía −81,5999…).
  - **No visto con sonido audible**: las barras con nivel alto y LIM quedan por ver con sonido real.
- **129 pruebas en verde.**

## 13. Etapa 6: entrega (instalador y portable), 22/10/2026

**Riesgos explicados antes de empezar** (pedido del usuario):
- El instalador modifica la computadora donde se instala. Por eso: por usuario, sin administrador, todo se borra con el desinstalador, que **nunca borra los shows ni los ajustes**. Y **no se instala en la PC del usuario sin su permiso**.
- Que choque con la V1 instalada: lleva otro nombre y otro AppId.
- El instalador de WebView2 se baja de Microsoft: se exige su **firma digital válida**.
- Sin firma propia sigue el aviso de SmartScreen.
- Subir a GitHub se avisa antes; publicar una Release, solo con permiso aparte.

### Nombre: «Otaconvert V2» (pedido del usuario, 22/10/2026)
- «De ahora en adelante no hablemos más de Otaconvert, sino de Otaconvert V2, tanto al nombrarla como en el nombre del programa, del acceso directo, del instalador, etc.»
- Hecho en todo lo visible:
  - el título de la ventana y el nombre por defecto;
  - Acerca de y Primeros pasos;
  - los títulos de la proyección;
  - los mensajes del motor y del lanzador;
  - `--version`;
  - el informe de la Prueba de hardware;
  - el instalador (AppName, carpeta, accesos directos, `Otaconvert V2.exe`, `Otaconvert-V2-Instalador-<v>.exe`);
  - la portable (`Otaconvert-V2-<v>`, artifact `Otaconvert-V2-Portable`);
  - el LEEME y el manual de usuario;
  - las descripciones de los exe (FileDescription).
- **Excepción a propósito:** el `ProductName` interno de los exe sigue siendo «Otaconvert», porque SignPath exige que coincida con el proyecto pedido, que incluye V1/V1.5 y V2. No se ve al usarlo. Se le explicó al usuario y se le recomendó no cambiar el nombre en SignPath.
- La carpeta de datos `%LOCALAPPDATA%\Otaconvert` no cambia, para no perder ajustes.
- «Otaconvert V1» se sigue llamando así donde se nombra.

### Lanzador sin ventana negra (`crates/lanzador` → `Otaconvert V2.exe`)
- Programa sin consola (`windows_subsystem = "windows"`). Abre `otaconvert-motor.exe` (junto a él) con `CREATE_NO_WINDOW`; los procesos del motor (proyección) heredan su consola oculta. Comprobado: solo se ven las ventanas del programa.
- Los mensajes del motor van a `registro-motor.txt` de la carpeta de datos (el anterior queda como `registro-motor.anterior.txt`).
- Si el motor termina con error, una ventana normal muestra las últimas 15 líneas y dónde está el registro completo.
- Qué abre:
  - sin nada: el «Show de ejemplo» de al lado;
  - algo arrastrado o un `.otashow` con doble clic: eso;
  - opciones (`--…`): tal cual;
  - sin ejemplo: explica cómo abrir un show.
- **Componentes que faltan:** si no está WebView2 (claves de registro de Microsoft) y viene `componentes\MicrosoftEdgeWebView2RuntimeInstallerX64.exe`, una sola ventana ofrece instalarlo («Sí / No»). Usa `/silent /install`: sin internet, por usuario, sin administrador. Si se dice que no, el programa se abre en el navegador. Nunca instala sin preguntar.
- Mensajes en el idioma de Windows (español o inglés).
- `SetDefaultDllDirectories` como el motor; mismos datos de versión, icono y manifiesto.
- Pruebas (2): qué abre según lo que recibe, la carpeta de datos portable, las últimas líneas.
- Probado en la PC del usuario: abre el show con la ventana propia, sin consola; el registro quedó en `datos\`.
- **El motor sigue con consola** a propósito: la Prueba de hardware y el diagnóstico la necesitan.

### Instalador y portable (workflow V2)
- **WebView2:** se baja de `https://go.microsoft.com/fwlink/?linkid=2124701`. La compilación falla si `Get-AuthenticodeSignature` no da `Valid` con sujeto `O=Microsoft Corporation`. Va en `componentes\`.
- **Portable:** carpeta `Otaconvert-V2-<v>` con `Otaconvert V2.exe` (lanzador), `otaconvert-motor.exe`, `Prueba de hardware.exe`, ventana, libmpv, Vulkan, `componentes\`, manual, LEEME, licencias, `portable.txt`, `pruebas\` y «Show de ejemplo». Artifact `Otaconvert-V2-Portable`.
- **Instalador** (`v2/instalador/otaconvert-v2.iss`, Inno Setup 6, como la V1):
  - AppId `{64A53954-3B3A-42BF-B091-4902CF3464BA}`; convive con la V1.
  - Por usuario (`PrivilegesRequired=lowest`), Windows 10 1607+, x64; idiomas español e inglés; muestra la licencia GPLv3.
  - Accesos directos: «Otaconvert V2», «Manual de usuario de Otaconvert V2», «Prueba de hardware de Otaconvert V2» y escritorio opcional.
  - Tarea opcional: **abrir los `.otashow` con Otaconvert V2** (solo HKCU, `Otaconvert2.Show`, se borra al desinstalar).
  - No instala `portable.txt` ni `datos`.
  - Se arma con `choco install innosetup` + `ISCC /DAppVersion /DOrigen` a partir de la carpeta de la portable. Artifact `Otaconvert-V2-Instalador`.
- Las comprobaciones de seguridad (sin runtime de C externo, `/DEPENDENTLOADFLAG:0x800`, datos de versión) ahora también cubren el lanzador.
- **Pendiente:**
  - Subir y ver la compilación en GitHub (avisar antes).
  - **Instalar en la PC del usuario solo con su permiso.**
  - Firma (SignPath).
  - Release pública (con permiso).
  - Que «Archivo → Abrir…» funcione dentro del programa (hoy se abre arrastrando o con doble clic en el `.otashow`).
- **Primera compilación del instalador:**
  - El run 39629173876 falló solo en el instalador: los mensajes por idioma de Inno se escriben `spanish.Nombre`, con punto, y estaban con dos puntos.
  - **Arreglado: run 39630811545 verde.**
- **Lo que armó:**
  - `Otaconvert-V2-Instalador-0.0.1.exe`: **278 MB** (SHA-256 `648bfa7e…`); 204 MB son del instalador de WebView2, con la firma de Microsoft comprobada.
  - Portable `Otaconvert-V2-0.0.1`: **378 MB** con los vídeos de prueba. `Otaconvert V2.exe` (`e530825c…`), `otaconvert-motor.exe` (`6f3851e0…`). `--comprobar`: no falta nada. Datos de versión: ProductName «Otaconvert», FileDescription «Otaconvert V2».
- **Copiados a la PC del usuario**, sin instalar ni tocar la demo vieja que tenía abierta:
  - la portable en `CLOUDE - Herramientas Ota\Otaconvert-V2-0.0.1`;
  - el instalador en `CLOUDE - Herramientas Ota\Otaconvert-V2-Instalador-0.0.1.exe`.
- **Archivo → Abrir… (Ctrl+O)**, en el commit 1c06c10:
  - El diálogo de Windows elige un `.otashow`. Solo se puede en Edición y con nada sonando.
  - El motor comprueba que el show se pueda abrir, guarda el actual, abre el otro con un motor nuevo (la misma salida de mensajes) y termina.
  - Consejo `abrir_show`; atajo en Ayuda.
  - **Sin probar en vivo:** el usuario tenía la demo abierta y se paró la prueba con teclas automáticas para no tocar sus ventanas.

### Vista Estudio: el GO reemplaza, con transición (22/10/2026)
- **Pedido del usuario:** en la Vista Estudio (como OBS) los cues no deben superponerse con el anterior; la superposición es para la Vista Lista (como QLab).
- **Decidido por el usuario:**
  - En la Vista Estudio el GO reemplaza **todo** (audio y vídeo).
  - La transición se elige entre **cuchilla, fundido (con ms ajustables) y blackout**.
  - La botonera y la escucha por auriculares no se tocan.
  - En la Vista Lista nada cambia: los cues se superponen.
- **Experimento antes de tocar la proyección** (`--prueba-cruzado`), con riesgo avisado y aprobado:
  - Fundido cruzado de imagen = la capa nueva como ventana hija «en capas» (`WS_EX_LAYERED`, Windows 8+) con su transparencia de 0 a 255 (`SetLayeredWindowAttributes`).
  - Con la forma de dibujar de mpv por defecto (d3d11 «flip») a la mitad se ve la mezcla exacta, **0 negros** con imágenes y con vídeos reales.
  - Con `d3d11-flip=no` sale **negro**: descartado.
  - **Hallazgo aparte:** al disparar un vídeo nuevo se pierde **1 cuadro al arrancar**, también con el corte de siempre (control: 5 en 6 cambios). No es negro, ya existía y no lo causa el fundido. Revisar más adelante.
- **Archivo del show:** `ajustes.transicion { tipo: corte | fundido | negro, ms }`, por defecto cuchilla con 1000 ms. Los shows viejos abren igual. Validación: ms ≤ 10 000.
- **Proyección** (`proyector.rs`):
  - Orden `mostrar {id, trans, ms}` con estado `Saliendo` (lo anterior sigue a la vista debajo).
  - **Fundido:** la capa nueva sube transparente y llega a entera en `ms`. Al terminar se le quita la transparencia y se retira lo anterior.
  - **Blackout:** lo anterior baja a negro con el brillo de mpv (curva t²) en `ms/2`. Después sube lo nuevo desde negro (brillo −100 → 0, curva (1−t)²) en `ms/2`.
  - Cualquier otra orden (otro GO, Parar todo, Fundir, Pausa) primero salta la transición a su final.
  - Orden nueva del lado de la sesión: `OrdenProyector::MostrarCon { id, transicion }`. Para la cuchilla se sigue usando `Mostrar`.
- **Sesión:**
  - `go_con(reemplazar)`; `go()` = Vista Lista.
  - `reemplazar_anteriores` hace lo siguiente según la transición:
    - lo que suena baja con `Parar` (cuchilla = rampa de parada, fundido = `ms`, blackout = `ms/2`);
    - si lo nuevo es un audio, la imagen anterior va a negro o se funde;
    - lo nuevo entra con un fundido mínimo igual al de la transición.
  - En el blackout el sonido nuevo **espera** a la mitad (`pendientes`, hilo «transiciones» cada 5 ms).
  - Parar todo y Fundir todo vacían lo pendiente; la pausa lo corre.
  - Registro TRANSICION.
  - Ajustes: claves `transicion_tipo` y `transicion_ms`.
- **Ventana:**
  - En la Vista Estudio el GO (botón y Espacio) manda `go {reemplazar: true}`.
  - En Ajustes del show, sección «Vista Estudio: transición»: Transición (Cuchilla / Fundido cruzado / Blackout) y Duración en ms.
  - Consejos `transicion` y `transicion_dur` (es/en). El consejo de cada estilo ahora dice si se superpone o reemplaza.
- **Pruebas:**
  - `tests/transicion.rs`: 6 pruebas (Lista superpone; cuchilla; fundido 48 000 cuadros; blackout espera y Parar todo lo cancela; vídeo con `MostrarCon` y audio después funde la imagen; ajustes y shows viejos). **137 pruebas en verde.**
  - `--prueba-transiciones`, con el proyector de verdad, **todo BIEN** en la PC del usuario:
    - fundido: 74 muestras, 0 negras, la mezcla a la mitad;
    - blackout: pasa por negro y sale el nuevo;
    - cuchilla sin negros;
    - Parar todo en pleno fundido: negro a los 120 ms, y el GO siguiente sale bien;
    - GO en pleno blackout: sale el nuevo.
  - Aprendido: el brillo de mpv oscurece **restando** luz (los canales débiles llegan antes a 0), no escalando.
  - En el programa: el segundo GO de la Vista Estudio reemplazó al primero con un fundido de 1500 ms (registro TRANSICION), con el volumen maestro en −60 dB.
- **Sin probar:** con proyector real, y oír el fundido de sonido (solo se comprobaron las órdenes al mezclador).

### Vista Estudio como OBS: diseño aprobado (22/10/2026)
- **Pedido del usuario:**
  - Que la Vista Estudio se vea como el Modo Estudio de OBS: arriba la pantalla partida, con la **previa a la izquierda** y la **salida (Programa) a la derecha**; abajo, paneles con las escenas (cues), las fuentes de cada escena y lo demás que convenga.
  - **Solo para el modo Estudio**: la Vista Lista no cambia.
  - Después pidió que **cada panel se pueda mover**, un **estilo más 3D y llamativo** y que se sumaran **todas** las sugerencias.
  - Sobre todo quiere la **Multivista** (todas las escenas en cuadrícula), **opcional** y **muy personalizable**: ordenable de cualquier forma; en cada escena se elige si el vídeo se ve en movimiento y si se escucha, eligiendo por dónde sale ese sonido para que **nunca suene en la sala (PA)**.
  - Pidió también que **todo tenga su ayuda** (Modo Ayuda, consejos con ejemplos de escena, Primeros pasos, manual) y que **no suba el uso de hardware ni empeore nada**.
- **Bocetos** (`v2/diseno/`):
  - `boceto-estudio-obs.png`: distribución.
  - `boceto-estudio-obs-2.png`: paneles movibles, mientras se arrastra uno.
  - `boceto-estudio-obs-3.png`: estilo 3D, **aprobado** («perfecto»).
  - `boceto-estudio-multivista.png`: con Multivista, palanca T y Corte rápido.
- **Distribución:**
  - Arriba: **Vista previa** (borde y luz azul) · centro con **GO de transición**, transición elegida y duración · **Programa / al aire** (borde y luz roja que late, cuenta de lo que falta, Fundir y Parar de cada cosa que suena con mini medidor).
  - Abajo, paneles movibles (agarre ⋮⋮, el orden se guarda en el equipo, Vista → Restaurar vista lo devuelve):
    - **Escenas** (lista con fotos y luces de estado);
    - **Fuentes** del cue elegido (vídeo o imagen, su sonido, encuadre, nota, cada una con ⚙);
    - **Mezclador** (medidores LED de Sala y Auriculares, maestro y, más adelante, volumen por cue);
    - **Transiciones**;
    - **Controles** (GO, Pausa, Fundir todo y Parar todo: se mueven pero **nunca se ocultan**);
    - **Botonera**, si está activa;
    - **Multivista** (opcional).
  - Barra de estado abajo: proyector, sala y auriculares en uso.
- **Estilo 3D:** relieve con brillo arriba y sombra abajo, pantallas con marco y reflejo, halos azul y rojo, GO en relieve, botones como teclas, medidores LED en segmentos, faders con perilla metálica. Hecho con capas semitransparentes sobre los colores del tema, para que valga también en Noche, Día y Fresno.
- **Sugerencias aceptadas (todas):**
  1. teclas 1–9 mandan una escena a la Previa;
  2. **Corte rápido** (cuchilla aunque la transición elegida sea otra);
  3. **palanca T** para fundir a mano;
  4. cuenta regresiva en amarillo y luego rojo en los últimos 10 s;
  5. «siguiente sugerido» a la Previa (desactivable);
  6. animación de la transición (barrido de luz y destello en Programa);
  7. **Multivista**;
  8. medidores con pico retenido;
  9. modo cabina a pantalla completa;
  10. aviso si la Previa ya está al aire;
  11. escenas con archivo que falta en rojo con ⚠;
  12. distribuciones de paneles guardables («Mi gala», «Mi obra»).
- **Multivista:**
  - Orden **Libre** (arrastrar), por **Lista**, **Tipo** o **Nombre**; **grupos con título** (Acto 1…) entre los que se arrastran las escenas; tamaño de tarjeta; filtro por tipo.
  - En cada tarjeta, ▶/⏸ (movimiento) y 🎧 (escuchar). Movimiento general: **Todas / Al pasar el ratón / Ninguna**.
  - El **sonido de prueba sale por la salida elegida y nunca por la de la sala**: si se elige la tarjeta de Principal, no se permite.
  - Clic = a la Previa; doble clic = GO.
- **Que no suba el uso de hardware (reglas):**
  1. El proyector y el sonido mandan; las vistas previas usan lo que sobra.
  2. Las tarjetas «en movimiento» no reproducen el vídeo: muestran una **tira de fotos chicas** hecha una vez por archivo, en segundo plano y en proceso aparte (como las miniaturas).
  3. Vídeo real solo en la Vista previa grande y en la tarjeta bajo el ratón, de a uno.
  4. Una sola escucha de prueba a la vez.
  5. **Vista → Rendimiento: Automático / Liviano.** Liviano = sin animaciones ni brillos. Automático = pasa a Liviano y avisa si el proyector pierde cuadros o la tarjeta de sonido se atrasa.
  6. Sin desenfoque de fondo (`backdrop-filter`); la única animación permanente es la luz de «al aire», quieta en Liviano.
  7. **Prueba obligatoria:** función simulada con la Multivista abierta, 0 cuadros perdidos en el proyector y 0 cortes de sonido, o no se sube.
- **Partes** (cada una se prueba antes de seguir):
  1. distribución, estilo 3D y paneles movibles;
  2. palanca T, Corte rápido, avisos y teclas 1–9;
  3. Multivista;
  4. mezclador con volumen por cue (toca el motor de audio: **avisar antes**).
