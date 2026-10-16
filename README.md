# Otaconvert

Herramientas libres y gratuitas para técnicos de teatro, para Windows.

- **Otaconvert V1** (terminada): conversor de audio por lotes. Arrastra muchos archivos, elige el formato y conviértelos todos de una vez.
- **Otaconvert V2** (en desarrollo, carpeta [`v2/`](v2/)): reproductor de cues de audio, vídeo e imágenes para funciones de teatro, al estilo de QLab o MultiPlay. Ya hay una herramienta descargable: la **[Prueba de hardware](https://github.com/marcoldmusica-cyber/otaconvert/releases/tag/prueba-hardware-0.0.1)**, que revisa si un equipo está listo para hacer funciones (sonido, pantallas y vídeo hasta 4K).

Hecho por amor al arte. **Nadie se salva solo.**

## Descargas

Todas las descargas están en [Releases](https://github.com/marcoldmusica-cyber/otaconvert/releases), con su huella SHA-256 para comprobarlas. Funcionan sin internet. Windows 10 u 11 de 64 bits.

## Contacto

Desarrollado por **@marcold.ok** (Instagram). Escríbeme para sugerir mejoras, reportar errores o contar cómo lo usas en tu teatro.

## Licencia

Todo el repositorio es software libre bajo la **GNU General Public License v3.0** (ver [`LICENSE`](LICENSE)), con un permiso adicional (sección 7) para enlazar la ventana de la V2 con el cargador de WebView2 de Microsoft (ver [`LICENSE-EXCEPTION-WEBVIEW2.txt`](LICENSE-EXCEPTION-WEBVIEW2.txt)). Las piezas de terceros y sus licencias están en [`THIRD_PARTY.txt`](THIRD_PARTY.txt) (V1) y en la versión [`dependencias-2026-10-15`](https://github.com/marcoldmusica-cyber/otaconvert/releases/tag/dependencias-2026-10-15) (V2).

## Code signing policy

*Política de firma de código.*

Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org). *(Solicitada; se aplicará cuando SignPath Foundation la apruebe. Hasta entonces los ejecutables no están firmados.)*

- **Qué se firma:** solo los ejecutables compilados automáticamente por GitHub Actions a partir del código de este repositorio ([`.github/workflows/`](.github/workflows/)). Nunca archivos compilados a mano.
- **Roles del equipo:**
  - Committers and reviewers (pueden modificar el repositorio y revisan todo cambio de otras personas): [marcoldmusica-cyber](https://github.com/marcoldmusica-cyber)
  - Approvers (aprueban cada pedido de firma): [marcoldmusica-cyber](https://github.com/marcoldmusica-cyber)
- Todos los miembros usan autenticación de dos factores en GitHub y en SignPath.

## Privacy policy

*Privacidad.*

This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it.

Otaconvert no envía datos a ningún lado: no tiene telemetría, no se conecta a internet y no abre puertos de red. Las funciones de red futuras (por ejemplo, disparar el GO desde un celular) vendrán apagadas y solo funcionarán si el usuario las activa.

## Desinstalar

- **Instalador (V1):** Configuración de Windows → Aplicaciones → Otaconvert → Desinstalar.
- **Portable y Prueba de hardware:** borrar la carpeta. No dejan nada en el registro de Windows.

---

## Otaconvert V1: lo que recibe quien lo usa (sin instalar nada más)

| Archivo | Para qué |
|---|---|
| `Otaconvert-Setup.exe` | Instalador de Windows. No pide permisos de administrador. Crea acceso directo y desinstalador. |
| `Otaconvert-Portable.zip` | Versión portable: descomprime en cualquier carpeta o pendrive y abre `Otaconvert.exe`. Los ajustes se guardan en esa misma carpeta. |
| `Otaconvert-macOS.zip` | Versión para Mac. |

Todo lleva dentro Python, la interfaz y FFmpeg. No hay que descargar nada más.

## Cómo generar esos archivos (una sola vez)

### Opción A: sin instalar nada en tu PC (GitHub)
1. Crea un repositorio en github.com y sube el contenido de esta carpeta (incluida `.github`).
2. Entra en **Actions → Compilar Otaconvert → Run workflow**.
3. A los pocos minutos, descarga los archivos desde la ejecución terminada (sección *Artifacts*).
   Si en cambio creas una etiqueta `v1.0.0`, se publican solos en la pestaña **Releases**, listos para compartir.

### Opción B: en tu propio PC con Windows
1. Instala Python 3.10 o superior (https://www.python.org/downloads/, marca *Add python.exe to PATH*).
2. Para obtener también el instalador, instala Inno Setup (gratis): https://jrsoftware.org/isdl.php
3. Haz doble clic en `build_windows.bat`. Los archivos aparecen en `dist\release`.

## Notas
- Windows puede mostrar "Windows protegió su PC" la primera vez, porque el programa todavía no está firmado digitalmente
  (ver *Code signing policy* arriba). Se abre con *Más información → Ejecutar de todas formas*.
- En Mac, la primera vez: clic derecho sobre la app → *Abrir*.
- Para ejecutarlo desde el código (desarrollo): `pip install -r requirements.txt` y `python otaconvert.py`.
- Comprobación rápida del programa empaquetado: `Otaconvert.exe --selftest informe.txt`.
