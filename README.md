# Otaconvert

Conversor de audio por lotes. Arrastra muchos archivos, elige el formato y conviértelos todos de una vez.

## Lo que recibe quien lo usa (sin instalar nada más)

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
- Windows puede mostrar "Windows protegió su PC" la primera vez, porque el programa no está firmado digitalmente
  (la firma cuesta dinero). Se abre con *Más información → Ejecutar de todas formas*.
- En Mac, la primera vez: clic derecho sobre la app → *Abrir*.
- Para ejecutarlo desde el código (desarrollo): `pip install -r requirements.txt` y `python otaconvert.py`.
- Comprobación rápida del programa empaquetado: `Otaconvert.exe --selftest informe.txt`.
