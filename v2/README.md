# Otaconvert V2

Reproductor de cues de audio, vídeo e imágenes para teatro. Solo Windows 10 y 11 de 64 bits.

**Estado:** Fase 0, prueba técnica del motor. Todavía no es un programa usable.

- `crates/motor`: motor en Rust (audio con cpal/WASAPI; vídeo con libmpv más adelante).
- `diseno/`: boceto aprobado de la interfaz.
- Compila en GitHub Actions con `.github/workflows/v2.yml`. La V1 (carpeta superior) no depende de esto.

Licencia: GPLv3 o posterior (ver `LICENSE`), con el permiso adicional de `../LICENSE-EXCEPTION-WEBVIEW2.txt` para `otaconvert-ventana.exe`.
