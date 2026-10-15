#!/usr/bin/env python3
"""Genera la versión portable y el instalador de Otaconvert.

Uso:  python build.py        (resultado en dist/release/)
Requiere: pip install -r requirements.txt pyinstaller   (solo en el equipo donde compilas)
"""
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DIST = ROOT / "dist"
REL = DIST / "release"
VERSION = os.environ.get("OTA_VERSION", "1.0.0").lstrip("v")
WIN, MAC = sys.platform == "win32", sys.platform == "darwin"


def run(cmd, **kw):
    print(">", " ".join(map(str, cmd)))
    subprocess.check_call(cmd, cwd=ROOT, **kw)


def selftest(exe: Path):
    """Ejecuta el programa YA empaquetado y comprueba que convierte a todos los formatos."""
    report = ROOT / "build" / "selftest.txt"
    report.parent.mkdir(exist_ok=True)
    report.unlink(missing_ok=True)
    code = subprocess.run([str(exe), "--selftest", str(report)]).returncode
    print(report.read_text(encoding="utf-8") if report.exists() else "(sin informe)")
    if code != 0:
        sys.exit("La autoprueba falló: el programa empaquetado no puede convertir.")


def installer():
    cands = [shutil.which("iscc")] + [
        Path(os.environ[v]) / sub / "ISCC.exe"
        for v, sub in (("ProgramFiles(x86)", "Inno Setup 6"), ("ProgramFiles", "Inno Setup 6"),
                       ("LOCALAPPDATA", "Programs/Inno Setup 6")) if v in os.environ]
    iscc = next((str(c) for c in cands if c and Path(c).exists()), None)
    if not iscc:
        print("\nAviso: Inno Setup no está instalado, así que no se crea el instalador (la versión portable sí).\n"
              "Descárgalo gratis en https://jrsoftware.org/isdl.php y vuelve a ejecutar este script.")
        return
    run([iscc, f"/DAppVersion={VERSION}", f"/O{REL}", "installer.iss"])


def main():
    shutil.rmtree(DIST, ignore_errors=True)
    shutil.rmtree(ROOT / "build", ignore_errors=True)
    run([sys.executable, "-m", "PyInstaller", "--noconfirm", "--clean", "otaconvert.spec"],
        env=dict(os.environ, OTA_VERSION=VERSION))
    REL.mkdir(parents=True)

    if MAC:
        app = DIST / "Otaconvert.app"
        selftest(app / "Contents" / "MacOS" / "Otaconvert")
        run(["ditto", "-c", "-k", "--keepParent", str(app), str(REL / "Otaconvert-macOS.zip")])
    else:
        folder = DIST / "Otaconvert"
        shutil.copy(ROOT / "THIRD_PARTY.txt", folder)
        selftest(folder / ("Otaconvert.exe" if WIN else "Otaconvert"))
        portable = DIST / "Otaconvert-Portable"       # misma carpeta + marca de modo portable
        shutil.copytree(folder, portable)
        (portable / "portable.txt").write_text(
            "Otaconvert Portable: los ajustes se guardan en esta carpeta.\n"
            "Borra este archivo si prefieres que se guarden en tu usuario.\n", encoding="utf-8")
        shutil.make_archive(str(REL / "Otaconvert-Portable"), "zip" if WIN else "gztar", DIST, "Otaconvert-Portable")
        if WIN:
            installer()
    print("\nListo. Archivos en:", REL)
    for f in sorted(REL.iterdir()):
        print(f"  {f.name}  ({f.stat().st_size / 1e6:.0f} MB)")


if __name__ == "__main__":
    main()
