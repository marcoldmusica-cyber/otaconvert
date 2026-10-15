@echo off
setlocal
cd /d "%~dp0"
echo.
echo  ===== Otaconvert: creando instalador y version portable =====
echo.

where py >nul 2>nul && (set PY=py -3) || (where python >nul 2>nul && (set PY=python) || (
  echo  Necesitas tener Python 3.10 o superior instalado SOLO en este equipo para compilar.
  echo  Descargalo de https://www.python.org/downloads/ y marca "Add python.exe to PATH".
  pause & exit /b 1))

%PY% -m venv .buildenv || goto :error
call .buildenv\Scripts\activate.bat
python -m pip install --upgrade pip --quiet
pip install -r requirements.txt pyinstaller --quiet || goto :error
python build.py || goto :error

echo.
echo  Listo. Tus archivos estan en la carpeta  dist\release
start "" "dist\release"
pause
exit /b 0

:error
echo.
echo  Algo fallo. Revisa los mensajes de arriba.
pause
exit /b 1
