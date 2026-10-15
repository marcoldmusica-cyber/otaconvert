# PyInstaller: empaqueta Otaconvert con Python, la interfaz y FFmpeg dentro (sin instalar nada más).
import os
import sys
from PyInstaller.utils.hooks import collect_all

VERSION = os.environ.get("OTA_VERSION", "1.0.0").lstrip("v")
ICON = {"win32": "assets/otaconvert.ico", "darwin": "assets/otaconvert.icns"}.get(sys.platform)

datas, binaries, hiddenimports = [("assets", "assets")], [], []
for pkg in ("customtkinter", "tkinterdnd2", "imageio_ffmpeg"):  # imageio_ffmpeg trae el ejecutable de FFmpeg
    d, b, h = collect_all(pkg)
    datas += d
    binaries += b
    hiddenimports += h

a = Analysis(["otaconvert.py"], binaries=binaries, datas=datas, hiddenimports=hiddenimports,
             excludes=["numpy", "matplotlib", "scipy", "pandas", "IPython", "pytest"])
pyz = PYZ(a.pure)
exe = EXE(pyz, a.scripts, [], exclude_binaries=True, name="Otaconvert", console=False, icon=ICON, upx=False)
coll = COLLECT(exe, a.binaries, a.datas, name="Otaconvert", upx=False)

if sys.platform == "darwin":
    app = BUNDLE(coll, name="Otaconvert.app", icon=ICON, bundle_identifier="app.otaconvert",
                 info_plist={"CFBundleShortVersionString": VERSION, "NSHighResolutionCapable": True})
