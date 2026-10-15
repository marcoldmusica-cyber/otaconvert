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

**Estado (15/10/2026):** solo planificación y boceto. No hay código de la V2. Boceto interactivo (Edición/Función/Ayuda): `v2/diseno/boceto-pantalla-principal.html`, publicado en https://claude.ai/artifact/REKcem8j1vWFoAJPcNqAA1. Doc de mapeo: https://claude.ai/code/artifact/b6be72d1-fcde-48b6-b0c2-4d14c79abc8c (su contenido está replicado aquí).

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

Atajos propuestos en el boceto (no confirmados por el usuario): Espacio GO, Esc Parar todo, P Pausa, F Fundir todo, ↑↓ mover cue en espera, H Ayuda.

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
Mezcla en float 32; limitador maestro transparente activo por defecto; rampas de 5–10 ms en todo inicio, parada, pausa, salto y cambio de volumen; medición de picos al preparar (la comprobación previa marca > 0 dB, opción de normalizar); **sin remuestreo en vivo** (todo preparado a la frecuencia real de la tarjeta, leída sola); loops sin hueco. Si el teatro trabaja a otra frecuencia, re-preparación en segundo plano con barra; sin luz verde hasta terminar.

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

### Licencia (pendiente del usuario, no bloquea)
mpv es GPLv2+ (LGPL con opción de compilación). ASIO con doble licencia desde octubre de 2025 (se sumó GPLv3). Recomendación dada: **GPLv3, gratuita**. Alternativa: cerrado + acuerdo ASIO + mpv LGPL.

---

## 8. Hoja de ruta V2

- **Fase 0 — Prueba técnica del motor y Prueba de hardware.** Prototipo Rust (WASAPI/ASIO + libmpv) en procesos separados. Prueba de hardware con informe estándar (tarjeta, canales, frecuencias, cortes, latencia, tono por salida, estrés, vídeo 1080p 60 s contando cuadros perdidos, GPU por salida, cambio 44,1/48 midiendo chasquidos y deriva). Pruebas automáticas de cientos de cambios de cue (cuadros negros, discontinuidades, picos): si una falla, no se avanza. Pruebas de horas. Verificar vigilante < 1 s, runtime de C, D3D11VA con vuelta a CPU, ASIO opcional. Lista de equipos probados. Ideal: notebook con dos GPU.
- **Fase 1 — Reproductor mínimo útil.** Lista de cues; GO con espacio; volumen; fade in/out; loop; inicio/fin sobre onda; parar/pausar/pánico; elección de tarjeta; modos con Tab e indicador; guardar/abrir show; preescucha; cues de audio, vídeo e imagen; *Proyector* con 4 puntos; miniatura; *Principal* + *Auriculares* y perfiles; pestañas, paletas, Vista básica (paneles, letra, Modo noche); **Ayuda para principiantes**. Justo después: pestaña Convertir.
- **Fase 2 —** Cue de Fade; esperas y auto-continuar; Grupos; Playlist; teclas rápidas; Vista → Personalizar, perfiles de operador, botones a medida.
- **Fase 3 — Multisalida.** Matriz de ruteo, pan, medidores, maestro; varias pantallas; varias superficies, máscaras, patrones, foto de fondo.
- **Fase 4 — Vídeo avanzado.** Capas, transiciones, texto; malla, fusión de bordes, 4 puntos animado.
- **Fase 5 — Control externo.** GO desde celular por Wi-Fi local; MIDI/OSC; MSC (GO, STOP, LOAD).
- **Siempre:** guardado automático, comprobación previa, modo respaldo.
- **Fuera de V2:** 25 tipos de cue, DMX, scripting, plugins, edición de audio avanzada, colaboración en línea, mapeo 3D.

## 9. Preguntas abiertas (no inventar respuestas)

1. ¿Puede correr la Prueba de hardware en 2–3 teatros o equipos?
2. Mapeo: ¿sobre qué figuras proyecta más (planas, siluetas, curvas, volumen)?
3. ¿La fusión de bordes puede esperar a la Fase 4?
4. Licencia: ¿GPLv3 o cerrado?
5. V1: ¿lista virtualizada para miles de archivos?
6. Menor: actualizar acciones del workflow (Node 20).
7. ¿Aprueba el boceto y los atajos propuestos?
8. **Funciones esenciales propuestas el 15/10/2026, pendientes de aprobación** (revisión de huecos frente a QLab, SCS y MultiPlay):
   - Fase 1: deshacer/rehacer en Edición; notas por cue (lo que ve el operador antes del GO); numeración libre de cues (1, 1.5, 2A) para coincidir con el libreto; parar o fundir un solo cue desde el panel "Sonando"; ir a cue y empezar un cue desde un punto (ensayos); protección contra doble GO (tiempo mínimo entre GOs); volumen maestro y medidor de nivel simple; "Reunir medios" (copiar todos los archivos a la carpeta del show); registro de la función (qué se disparó y cuándo, y cualquier error).
   - Decisiones: idiomas de la interfaz (¿español e inglés como la V1?); ¿la V2 solo para Windows o también macOS?; exportar/imprimir la hoja de cues (Fase 2).
   - Técnico: el archivo del show lleva número de versión para abrir shows viejos en versiones nuevas.

## 10. Siguiente paso

El usuario debe ver y aprobar el boceto (`v2/diseno/`). Después, **Fase 0**. No empezar la Fase 0 sin aprobación, salvo que lo pida.
