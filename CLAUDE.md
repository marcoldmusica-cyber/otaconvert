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
- Cuando el usuario dice **"el manual"** o "el registro del proyecto" se refiere a **este archivo (CLAUDE.md)**. Toda decisión, regla y opción nueva se anota aquí; lo hecho y probado (resultados, pruebas, compilaciones) va en `v2/REGISTRO.md` (sección 10).
- **Manual de usuario** (creado el 15/10/2026): `v2/manual/manual-usuario.html` + `v2/manual/manual-usuario.pdf`, con la estética del programa (el PDF y la impresión usan **siempre el tema oscuro normal**, nunca fondo blanco: decisión del usuario). **Regla:** cada cambio (correcciones de esta versión y versiones futuras) se anota en CLAUDE.md o `v2/REGISTRO.md` (sección 10) **y** en el manual de usuario, y después se regenera el PDF con `v2/manual/generar-pdf.ps1` (usa Edge, que viene con Windows). Lo que aún no funciona se marca "Próximamente" y lo que sí, "Ya funciona".
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

## 10. Registro de desarrollo (lo hecho y probado)

El historial detallado (Fase 0 y etapas 1 a 6 de la Fase 1: qué se hizo, pruebas, resultados, huellas SHA-256, compilaciones y pendientes de cada parte) está en **`v2/REGISTRO.md`**. **Antes de tocar una parte, leer su sección ahí.** Lo nuevo que se haga y pruebe se anota al final de ese archivo; las reglas y decisiones nuevas, aquí.

Estado al 22/10/2026: Fase 1 casi completa (etapas 1–6 hechas: show, audio, interfaz, vídeo y mapeo, edición, instalador y portable; Vista Estudio como OBS). Pendiente grande: **escenas con varias fuentes** (cambia el formato del show y la proyección: avisar antes; plan al final de `v2/REGISTRO.md`).
