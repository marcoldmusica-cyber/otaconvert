#!/usr/bin/env python3
"""Otaconvert 1.5: conversor de audio y vídeo por lotes y normalizador de volumen (FFmpeg + CustomTkinter)."""
from __future__ import annotations

import functools
import json
import math
import os
import queue
import re
import shutil
import statistics
import subprocess
import sys
import threading
from collections import deque
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from pathlib import Path
from tkinter import Canvas, PhotoImage, filedialog, messagebox

import customtkinter as ctk

try:
    from tkinterdnd2 import DND_FILES, TkinterDnD
    HAS_DND = True
except Exception:  # sin arrastrar y soltar la app sigue funcionando
    HAS_DND = False

APP_VERSION = "1.5.0"
CREDIT = "Desarrollado por @marcold.ok"

# ───────────────────────────── Ajustes: audio ─────────────────────────────

AUDIO_EXT = {
    ".mp3", ".wav", ".flac", ".ogg", ".oga", ".opus", ".m4a", ".aac", ".wma", ".aiff", ".aif",
    ".aifc", ".ape", ".wv", ".mka", ".weba", ".webm", ".mp4", ".m4b", ".m4r", ".mov", ".mkv",
    ".avi", ".caf", ".amr", ".ac3", ".dts", ".3gp", ".mpc", ".tta", ".au", ".snd", ".mp2",
    ".spx", ".gsm", ".ra",
}

# formato: (extensión, {calidad: argumentos de ffmpeg}, calidad por defecto)
FORMATS = {
    "WAV": (".wav", {"16-bit": ["-c:a", "pcm_s16le"], "24-bit": ["-c:a", "pcm_s24le"],
                     "32-bit float": ["-c:a", "pcm_f32le"]}, "16-bit"),
    "MP3": (".mp3", {f"{b} kbps": ["-c:a", "libmp3lame", "-b:a", f"{b}k"]
                     for b in (128, 192, 256, 320)}, "320 kbps"),
    "FLAC": (".flac", {"Automática": ["-c:a", "flac"],
                       "16-bit": ["-c:a", "flac", "-sample_fmt", "s16"],
                       "24-bit": ["-c:a", "flac", "-sample_fmt", "s32", "-bits_per_raw_sample", "24"]},
             "Automática"),
    "OGG": (".ogg", {"Media": ["-c:a", "libvorbis", "-q:a", "4"],
                     "Alta": ["-c:a", "libvorbis", "-q:a", "6"],
                     "Máxima": ["-c:a", "libvorbis", "-q:a", "9"]}, "Alta"),
    "M4A": (".m4a", {f"{b} kbps": ["-c:a", "aac", "-b:a", f"{b}k"] for b in (128, 192, 256)}, "256 kbps"),
    "OPUS": (".opus", {f"{b} kbps": ["-c:a", "libopus", "-b:a", f"{b}k"]
                       for b in (64, 96, 128, 192)}, "128 kbps"),
    "AIFF": (".aiff", {"16-bit": ["-c:a", "pcm_s16be"], "24-bit": ["-c:a", "pcm_s24be"]}, "16-bit"),
}
RATES = {"Original": None, "44.1 kHz": 44100, "48 kHz": 48000, "96 kHz": 96000}

# ───────────────────────────── Ajustes: vídeo ─────────────────────────────

VIDEO_EXT = {
    ".mp4", ".m4v", ".mov", ".mkv", ".avi", ".webm", ".wmv", ".asf", ".flv", ".f4v", ".mpg", ".mpeg",
    ".m2v", ".ts", ".mts", ".m2ts", ".vob", ".3gp", ".3g2", ".ogv", ".mxf", ".divx", ".dv", ".gif",
}

COPY, NOAUDIO = "Copiar original", "Sin audio"

# códec de vídeo: (codificador de FFmpeg que necesita, explicación corta)
VCODECS = {
    "H.264": ("libx264", "El más compatible: televisores, proyectores, celulares y la web."),
    "H.265": ("libx265", "Igual calidad que H.264 en la mitad de espacio. Algunos equipos viejos no lo leen."),
    "VP9": ("libvpx-vp9", "Pensado para la web: archivos livianos, pero tarda más en convertir."),
    "ProRes": ("prores_ks", "Para edición: calidad máxima y archivos muy grandes."),
    "MPEG-4": ("mpeg4", "Para reproductores y televisores muy antiguos."),
    COPY: (None, "Sin recodificar el vídeo: instantáneo y sin pérdida. Calidad y resolución no se aplican."),
}
ACODECS = {
    "AAC": "aac", "MP3": "libmp3lame", "Opus": "libopus", "Vorbis": "libvorbis", "FLAC": "flac",
    "PCM sin compresión": "pcm_s24le", COPY: None, NOAUDIO: None,
}
# contenedor: (extensión, códecs de vídeo posibles, códecs de audio posibles); el primero es el habitual
CONTAINERS = {
    "MP4": (".mp4", ["H.264", "H.265", COPY], ["AAC", "MP3", COPY, NOAUDIO]),
    "MOV": (".mov", ["H.264", "H.265", "ProRes", COPY], ["AAC", "PCM sin compresión", COPY, NOAUDIO]),
    "MKV": (".mkv", ["H.264", "H.265", "VP9", COPY],
            ["AAC", "Opus", "FLAC", "MP3", "PCM sin compresión", COPY, NOAUDIO]),
    "WEBM": (".webm", ["VP9"], ["Opus", "Vorbis", NOAUDIO]),
    "AVI": (".avi", ["MPEG-4", "H.264", COPY], ["MP3", "PCM sin compresión", COPY, NOAUDIO]),
}
QUALITIES = ["Máxima", "Alta", "Media", "Liviana"]
RESOLUTIONS = {"Original": None, "4K (2160p)": 2160, "1080p": 1080, "720p": 720, "480p": 480}

# ───────────────────────────── Ajustes: normalizar ─────────────────────────────

TRUE_PEAK = -1.0  # el pico real nunca supera −1 dB
TARGETS = {"−14 LUFS": -14.0, "−16 LUFS": -16.0, "−18 LUFS": -18.0, "−23 LUFS": -23.0, "Promedio": None}
TARGET_HINTS = {
    "−14 LUFS": "Fuerte, como las plataformas de música. A las pistas muy suaves les puede hacer falta limitador.",
    "−16 LUFS": "Recomendado: buen volumen sin apretar el sonido.",
    "−18 LUFS": "Más margen para pistas con mucha dinámica.",
    "−23 LUFS": "Estándar de TV (EBU R128): el más suave y con más margen.",
    "Promedio": "Iguala todas al volumen típico del grupo (el de la pista del medio): ninguna cambia demasiado.",
}
SAME = "Igual que el original"
NORM_OUT = {
    SAME: None,
    "WAV 24-bit": (".wav", ["-c:a", "pcm_s24le"]),
    "WAV 16-bit": (".wav", ["-c:a", "pcm_s16le"]),
    "FLAC 24-bit": (".flac", ["-c:a", "flac", "-sample_fmt", "s32", "-bits_per_raw_sample", "24"]),
    "MP3 320 kbps": (".mp3", ["-c:a", "libmp3lame", "-b:a", "320k"]),
    "M4A 256 kbps": (".m4a", ["-c:a", "aac", "-b:a", "256k"]),
    "AIFF 24-bit": (".aiff", ["-c:a", "pcm_s24be"]),
}

NOWINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)


def app_dir() -> Path:
    return Path(sys.executable).parent if getattr(sys, "frozen", False) else Path(__file__).resolve().parent


def resource(name: str) -> Path:
    return Path(getattr(sys, "_MEIPASS", Path(__file__).resolve().parent)) / "assets" / name


# Versión portable: si existe "portable.txt" junto al programa, los ajustes se guardan en su carpeta.
PORTABLE = (app_dir() / "portable.txt").exists()
CFG = app_dir() / "otaconvert-config.json" if PORTABLE else Path.home() / ".otaconvert.json"

# ───────────────────────────── Diseño ─────────────────────────────

BG, CARD, CARD2, HOVER, BORDER = "#0E1117", "#151921", "#1B2029", "#252B37", "#262C38"
TEXT, MUTED, DIM = "#E9ECF2", "#9098A8", "#5D6677"
ACCENT, ACCENT_H, OK, ERR = "#6F80FF", "#5C6DEE", "#58D69A", "#F27A74"
FONT = {"win32": "Segoe UI", "darwin": "SF Pro Display"}.get(sys.platform, "Helvetica")


@functools.lru_cache(maxsize=None)
def font(size: int = 13, weight: str = "normal") -> ctk.CTkFont:
    return ctk.CTkFont(family=FONT, size=size, weight=weight)


# ───────────────────────────── FFmpeg ─────────────────────────────

def find_ffmpeg() -> str | None:
    try:
        import imageio_ffmpeg
        exe = imageio_ffmpeg.get_ffmpeg_exe()
        if exe and os.path.exists(exe):
            return exe
    except Exception:
        pass
    return shutil.which("ffmpeg")


FFMPEG = find_ffmpeg()
BASE = ["-hide_banner", "-nostdin", "-y"]
PROGRESS = ["-progress", "pipe:1", "-nostats"]


@functools.lru_cache(maxsize=1)
def encoders() -> frozenset:
    """Codificadores que trae este FFmpeg (para no ofrecer un códec que no puede usar)."""
    if not FFMPEG:
        return frozenset()
    try:
        r = subprocess.run([FFMPEG, "-hide_banner", "-encoders"], capture_output=True, text=True,
                           stdin=subprocess.DEVNULL, encoding="utf-8", errors="replace",
                           creationflags=NOWINDOW, timeout=30)
        names = set()
        for line in r.stdout.splitlines():
            parts = line.split()
            if len(parts) >= 2 and len(parts[0]) == 6 and parts[0][0] in "VAS" and parts[1] != "=":
                names.add(parts[1])
        return frozenset(names)
    except Exception:
        return frozenset()


def vcodec_ok(name: str) -> bool:
    enc = VCODECS[name][0]
    return enc is None or enc in encoders()


def acodec_ok(name: str) -> bool:
    enc = ACODECS[name]
    return enc is None or enc in encoders()


def audio_cmd(src: Path, dst: Path, args: list[str], rate: int | None = None, progress: bool = True) -> list[str]:
    cmd = [FFMPEG, *BASE, "-i", str(src), "-vn", "-map", "0:a:0", "-map_metadata", "0", *args]
    if rate:
        cmd += ["-ar", str(rate)]
    return cmd + (PROGRESS if progress else []) + [str(dst)]


def video_args(container: str, vc: str, ac: str, quality: str, res: str) -> list[str]:
    """Argumentos de FFmpeg para convertir un vídeo (sin entrada ni salida)."""
    a = ["-map", "0:v:0"]
    if ac != NOAUDIO:
        a += ["-map", "0:a:0?"]  # el «?» deja convertir vídeos que no tienen sonido
    a += ["-map_metadata", "0"]
    if vc == COPY:
        a += ["-c:v", "copy"]
    else:
        lvl = QUALITIES.index(quality) if quality in QUALITIES else 1
        if vc == "H.264":
            a += ["-c:v", "libx264", "-preset", "medium", "-crf", str((16, 19, 23, 28)[lvl]), "-pix_fmt", "yuv420p"]
        elif vc == "H.265":
            a += ["-c:v", "libx265", "-preset", "medium", "-crf", str((18, 22, 26, 30)[lvl]), "-pix_fmt", "yuv420p",
                  "-x265-params", "log-level=error"]
            if container in ("MP4", "MOV"):
                a += ["-tag:v", "hvc1"]  # para que lo abran también los equipos de Apple
        elif vc == "VP9":
            a += ["-c:v", "libvpx-vp9", "-b:v", "0", "-crf", str((24, 30, 34, 40)[lvl]), "-deadline", "good",
                  "-cpu-used", "4", "-row-mt", "1", "-pix_fmt", "yuv420p"]
        elif vc == "ProRes":  # Máxima = HQ, Alta = 422, Media = LT, Liviana = Proxy
            a += ["-c:v", "prores_ks", "-profile:v", str((3, 2, 1, 0)[lvl]), "-pix_fmt", "yuv422p10le",
                  "-vendor", "apl0"]
        elif vc == "MPEG-4":
            a += ["-c:v", "mpeg4", "-q:v", str((2, 3, 5, 8)[lvl]), "-pix_fmt", "yuv420p"]
            if container == "AVI":
                a += ["-vtag", "xvid"]
        vf = []
        h = RESOLUTIONS.get(res)
        if h:  # baja la resolución sin estirar nunca y respetando los vídeos verticales
            vf.append(f"scale=w='if(gte(iw,ih),-2,min(iw,{h}))':h='if(gte(iw,ih),min(ih,{h}),-2)':flags=lanczos")
        vf.append("scale=trunc(iw/2)*2:trunc(ih/2)*2")  # estos códecs necesitan medidas pares
        a += ["-vf", ",".join(vf)]
    if ac == NOAUDIO:
        a += ["-an"]
    elif ac == COPY:
        a += ["-c:a", "copy"]
    elif ac == "AAC":
        a += ["-c:a", "aac", "-b:a", "256k"]
    elif ac == "MP3":
        a += ["-c:a", "libmp3lame", "-b:a", "320k"]
    elif ac == "Opus":
        a += ["-c:a", "libopus", "-b:a", "192k"]
    elif ac == "Vorbis":
        a += ["-c:a", "libvorbis", "-q:a", "6"]
    elif ac == "FLAC":
        a += ["-c:a", "flac"]
    elif ac == "PCM sin compresión":
        a += ["-c:a", "pcm_s16le" if container == "AVI" else "pcm_s24le"]
    if container in ("MP4", "MOV"):
        a += ["-movflags", "+faststart"]  # empieza a reproducirse antes de leerlo entero
    return a


def video_cmd(src: Path, dst: Path, args: list[str], progress: bool = True) -> list[str]:
    return [FFMPEG, *BASE, "-i", str(src), *args] + (PROGRESS if progress else []) + [str(dst)]


def loud_analyze_cmd(src: Path, progress: bool = True) -> list[str]:
    f = f"loudnorm=I=-16:TP={TRUE_PEAK}:LRA=11:print_format=json"
    return [FFMPEG, *BASE, "-i", str(src), "-vn", "-map", "0:a:0", "-af", f] + \
           (PROGRESS if progress else []) + ["-f", "null", "-"]


def loud_render_cmd(src: Path, dst: Path, meas: dict, target: float, out_args: list[str], rate: int,
                    progress: bool = True) -> list[str]:
    """Segunda pasada: aplica la ganancia exacta (modo lineal, sin tocar la dinámica).
    Si así el pico pasara de −1 dB, el filtro usa un limitador suave solo donde hace falta."""
    lra = min(50.0, max(11.0, float(meas["input_lra"]) + 1.0))  # nunca menor que el rango medido
    offset = meas["target_offset"] if abs(target - (-16.0)) < 0.05 else "0"
    f = (f"loudnorm=I={target:.1f}:TP={TRUE_PEAK}:LRA={lra:.1f}:measured_I={meas['input_i']}:"
         f"measured_TP={meas['input_tp']}:measured_LRA={meas['input_lra']}:"
         f"measured_thresh={meas['input_thresh']}:offset={offset}:linear=true:print_format=json")
    # loudnorm trabaja internamente a 192 kHz: se vuelve a la frecuencia original
    return [FFMPEG, *BASE, "-i", str(src), "-vn", "-map", "0:a:0", "-map_metadata", "0", "-af", f,
            *out_args, "-ar", str(rate)] + (PROGRESS if progress else []) + [str(dst)]


def parse_loudnorm(lines: list[str]) -> dict | None:
    end = next((i for i in range(len(lines) - 1, -1, -1) if lines[i].strip() == "}"), None)
    if end is None:
        return None
    start = next((i for i in range(end, -1, -1) if lines[i].strip() == "{"), None)
    if start is None:
        return None
    try:
        return json.loads("\n".join(lines[start:end + 1]))
    except ValueError:
        return None


_DUR = re.compile(r"Duration:\s*(\d+):(\d+):(\d+(?:\.\d+)?)")
_AUD = re.compile(r"Stream #[^:]+:\d+[^:]*: Audio: ([^,\s]+)[^,]*, (\d+) Hz, [^,]+, ([a-z0-9]+)(?: \((\d+) bit\))?")
_VID = re.compile(r"Stream #[^:]+:\d+[^:]*: Video: ([^,\s]+)")
_NOISE = re.compile(r"^(frame|fps|stream_\d+_\d+_q|bitrate|total_size|out_time\w*|dup_frames|"
                    r"drop_frames|speed|progress)=")


def probe(src: Path) -> dict:
    """Duración, frecuencia y formato de muestra del primer audio, y si hay vídeo."""
    info = {"dur": None, "rate": None, "depth": None, "codec": None, "video": False, "vcodec": None}
    try:
        r = subprocess.run([FFMPEG, "-hide_banner", "-i", str(src)], capture_output=True, text=True,
                           stdin=subprocess.DEVNULL, encoding="utf-8", errors="replace",
                           creationflags=NOWINDOW, timeout=30)
        err = r.stderr
    except Exception:
        return info
    if m := _DUR.search(err):
        info["dur"] = int(m[1]) * 3600 + int(m[2]) * 60 + float(m[3])
    if m := _AUD.search(err):
        info["codec"], info["rate"] = m[1], int(m[2])
        sfmt, bits = m[3], m[4]
        if m[1].startswith("pcm_f") or (sfmt.startswith("flt") and m[1].startswith("pcm")):
            info["depth"] = "float"
        elif bits == "16" or sfmt.startswith("s16") or m[1].startswith("pcm_s16"):
            info["depth"] = 16
        elif bits or sfmt.startswith(("s24", "s32")) or m[1].startswith(("pcm_s24", "pcm_s32")):
            info["depth"] = 24
    if m := _VID.search(err):
        info["video"], info["vcodec"] = True, m[1]
    return info


def adapt_cmd(cmd: list[str], out: Path, info: dict) -> list[str]:
    """Al copiar H.264 o H.265 de un MP4/MKV a un AVI, el flujo necesita otro empaquetado interno."""
    if (out.suffix.lower() == ".avi" and "-c:v" in cmd and cmd[cmd.index("-c:v") + 1] == "copy"
            and info.get("vcodec") in ("h264", "hevc")):
        return cmd[:-1] + ["-bsf:v", f"{info['vcodec']}_mp4toannexb"] + cmd[-1:]
    return cmd


def run_ffmpeg(cmd: list[str], dur: float | None, on_progress=None, on_proc=None) -> tuple[int, list[str]]:
    """Ejecuta FFmpeg informando el avance (0 a 1). Devuelve el código de salida y sus últimas líneas."""
    lines: deque[str] = deque(maxlen=80)
    rc = -1
    try:
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL,
                                text=True, encoding="utf-8", errors="replace", creationflags=NOWINDOW)
        if on_proc:
            on_proc(proc)
        last = 0.0
        for line in proc.stdout:
            line = line.strip()
            if line.startswith(("out_time_us=", "out_time_ms=")):
                try:
                    t = int(line.split("=")[1]) / 1e6
                except ValueError:
                    continue
                if on_progress and dur and (p := min(0.999, t / dur)) - last >= 0.01:
                    last = p
                    on_progress(p)
            elif line and not _NOISE.match(line):
                lines.append(line)
        rc = proc.wait()
    except Exception as exc:  # noqa: BLE001
        lines.append(str(exc))
    return rc, list(lines)


def friendly_error(tail: list[str]) -> str:
    t = " ".join(tail).lower()
    if "0:v:0" in t and "matches no streams" in t:
        return "El archivo no tiene pista de vídeo"
    if "matches no streams" in t or "does not contain any stream" in t or "output file does not contain" in t:
        return "El archivo no tiene pista de audio"
    if ("could not find tag for codec" in t or "not currently supported in container" in t
            or "incompatible with output codec" in t or "could not write header" in t):
        return "Ese códec no entra en este formato: elige un códec en lugar de «Copiar original»"
    if "invalid data found" in t:
        return "Archivo dañado o formato no compatible"
    if "permission denied" in t:
        return "Sin permiso para escribir en la carpeta"
    if "no such file" in t:
        return "No se encuentra el archivo"
    if "no space left" in t:
        return "No queda espacio en el disco"
    return (tail[-1] if tail else "Error desconocido")[:90]


def same_format(src: Path, depth) -> tuple[str, list[str]]:
    """Formato de salida «igual que el original» al normalizar; lo desconocido sale en WAV 24-bit."""
    ext = src.suffix.lower()
    if ext == ".wav":
        return ".wav", ["-c:a", {16: "pcm_s16le", "float": "pcm_f32le"}.get(depth, "pcm_s24le")]
    if ext == ".flac":
        return ".flac", (["-c:a", "flac", "-sample_fmt", "s16"] if depth == 16 else
                         ["-c:a", "flac", "-sample_fmt", "s32", "-bits_per_raw_sample", "24"])
    if ext in (".aiff", ".aif", ".aifc"):
        return ext, ["-c:a", "pcm_s16be" if depth == 16 else "pcm_s24be"]
    if ext == ".mp3":
        return ".mp3", ["-c:a", "libmp3lame", "-b:a", "320k"]
    if ext in (".m4a", ".aac"):
        return ext, ["-c:a", "aac", "-b:a", "256k"]
    if ext in (".ogg", ".oga"):
        return ext, ["-c:a", "libvorbis", "-q:a", "6"]
    if ext == ".opus":
        return ".opus", ["-c:a", "libopus", "-b:a", "192k"]
    return ".wav", ["-c:a", "pcm_s24le"]


def out_rate(out_args: list[str], rate: int | None) -> int:
    rate = rate or 48000
    if "libopus" in out_args:
        return 48000
    if "libmp3lame" in out_args:
        return min(rate, 48000)
    return min(rate, 192000)


def group_target(levels: list[float]) -> float:
    return round(min(-5.0, max(-70.0, statistics.median(levels))), 1)


def lufs(v) -> str:
    return f"{float(v):.1f}".replace("-", "−")


# ───────────────────────────── Utilidades ─────────────────────────────

def human(n: float) -> str:
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024 or unit == "GB":
            return f"{n:.0f} {unit}" if unit == "B" else f"{n:.1f} {unit}"
        n /= 1024
    return ""


def ellipsize(s: str, n: int) -> str:
    return s if len(s) <= n else s[: n // 2 - 1] + "…" + s[-(n - n // 2):]


def plural(n: int, one: str, many: str) -> str:
    return f"{n} {one if n == 1 else many}"


def unique_path(path: Path, reserved: set[Path]) -> Path:
    """Nunca pisa un archivo existente ni otro destino del mismo lote."""
    p, i = path, 1
    while p.exists() or p in reserved:
        p = path.with_name(f"{path.stem} ({i}){path.suffix}")
        i += 1
    reserved.add(p)
    return p


def collect(paths, exts: set[str]) -> list[Path]:
    out: list[Path] = []
    for raw in paths:
        p = Path(raw)
        if p.is_dir():
            out += sorted(f for f in p.rglob("*") if f.is_file() and not f.name.startswith(".")
                          and f.suffix.lower() in exts)
        elif p.is_file():
            out.append(p)
    return out


def open_folder(path: Path) -> None:
    try:
        if sys.platform == "win32":
            os.startfile(path)  # type: ignore[attr-defined]
        else:
            subprocess.Popen(["open" if sys.platform == "darwin" else "xdg-open", str(path)])
    except Exception:
        pass


def load_cfg() -> dict:
    try:
        cfg = json.loads(CFG.read_text())
        return cfg if isinstance(cfg, dict) else {}
    except Exception:
        return {}


def part_path(out: Path) -> Path:
    return out.with_name(out.stem + ".part" + out.suffix)


@dataclass
class Item:
    path: Path
    size: int
    status: str = "pending"  # pending | queued | working | analyzed | done | error | cancelled
    progress: float = 0.0
    out: Path | None = None
    sig: tuple | None = None
    proc: subprocess.Popen | None = None
    row: "Row | None" = None
    cmd: list = field(default_factory=list)
    info: dict = field(default_factory=dict)
    meas: dict | None = None
    note: str = ""


BUSY = ("queued", "working", "analyzed")

# ───────────────────────────── Widgets ─────────────────────────────


class Wave(Canvas):
    """Marca de Otaconvert: barras de onda. Late mientras se convierte."""
    BASE = (0.38, 0.72, 1.0, 0.58, 0.84, 0.46)

    def __init__(self, master, w, h, bg, bar=5):
        super().__init__(master, width=w, height=h, bg=bg, highlightthickness=0, bd=0)
        self.w, self.h, self.bar, self.t, self.on = w, h, bar, 0.0, False
        self.draw(self.BASE)

    def draw(self, heights):
        self.delete("all")
        step = self.w / len(heights)
        for i, k in enumerate(heights):
            half = max(0, self.h * k / 2 - self.bar / 2)
            x = step * (i + 0.5)
            self.create_line(x, self.h / 2 - half, x, self.h / 2 + half, width=self.bar,
                             capstyle="round", fill=ACCENT)

    def spin(self, on: bool):
        was, self.on = self.on, on
        if on and not was:
            self._tick()
        elif not on:
            self.draw(self.BASE)

    def _tick(self):
        if self.on:
            self.t += 0.35
            self.draw([0.3 + 0.7 * abs(math.sin(self.t + i * 0.8)) for i in range(len(self.BASE))])
            self.after(70, self._tick)


def ghost(master, text, cmd, w=96):
    return ctk.CTkButton(master, text=text, command=cmd, width=w, height=34, corner_radius=10,
                         fg_color="transparent", border_width=1, border_color=BORDER,
                         hover_color=HOVER, text_color=TEXT, text_color_disabled=DIM, font=font(13))


def segmented(master, values, command):
    return ctk.CTkSegmentedButton(
        master, values=values, command=command, height=34, corner_radius=10, font=font(12, "bold"),
        fg_color=CARD2, selected_color=ACCENT, selected_hover_color=ACCENT, unselected_color=CARD2,
        unselected_hover_color=HOVER, text_color=TEXT, text_color_disabled=DIM)


def option_menu(master, values, width, command=None):
    return ctk.CTkOptionMenu(master, values=values, width=width, height=34, corner_radius=10, command=command,
                             font=font(13), dropdown_font=font(13), fg_color=CARD2, button_color=CARD2,
                             button_hover_color=HOVER, dropdown_fg_color=CARD2, dropdown_hover_color=HOVER,
                             text_color=TEXT, text_color_disabled=DIM, dropdown_text_color=TEXT)


def group(master, caption, padx=(0, 0)):
    """Un control con su título encima, como en la V1."""
    f = ctk.CTkFrame(master, fg_color="transparent")
    f.pack(side="left", padx=padx, anchor="s")
    ctk.CTkLabel(f, text=caption, font=font(12), text_color=MUTED).pack(anchor="w", pady=(0, 6))
    return f


class Row(ctk.CTkFrame):
    def __init__(self, master, item: Item, label: str, on_remove):
        super().__init__(master, fg_color=CARD2, corner_radius=12)
        self.columnconfigure(1, weight=1)
        ext = (item.path.suffix.lstrip(".").upper() or "?")[:4]
        ctk.CTkLabel(self, text=ext, width=50, height=28, corner_radius=8, fg_color=CARD,
                     text_color=MUTED, font=font(11, "bold")).grid(row=0, column=0, padx=12, pady=12)
        info = ctk.CTkFrame(self, fg_color="transparent")
        info.grid(row=0, column=1, sticky="w")
        ctk.CTkLabel(info, text=ellipsize(item.path.name, 56), font=font(13), text_color=TEXT,
                     anchor="w", height=20).pack(anchor="w")
        self.base_meta = ellipsize(str(item.path.parent), 64)
        self.meta = ctk.CTkLabel(info, text=self.base_meta, font=font(11), text_color=DIM, anchor="w", height=16)
        self.meta.pack(anchor="w")
        self.size = ctk.CTkLabel(self, text=human(item.size), font=font(12), text_color=DIM,
                                 width=70, anchor="e")
        self.size.grid(row=0, column=2, padx=(8, 4))
        self.bar = ctk.CTkProgressBar(self, width=96, height=4, fg_color=BORDER, progress_color=ACCENT)
        self.bar.grid(row=0, column=3, padx=(8, 0))
        self.bar.grid_remove()
        self.state = ctk.CTkLabel(self, width=84, anchor="e", font=font(12))
        self.state.grid(row=0, column=4, padx=(8, 4))
        ctk.CTkButton(self, text="✕", width=28, height=28, corner_radius=8, fg_color="transparent",
                      hover_color=HOVER, text_color=DIM, font=font(12),
                      command=lambda: on_remove(item)).grid(row=0, column=5, padx=(0, 10))
        self.pending(label)

    def _set(self, text, color):
        self.state.configure(text=text, text_color=color)

    def pending(self, label):
        self.bar.grid_remove()
        self._set(label, DIM)

    def queued(self):
        self.bar.grid_remove()
        self.meta.configure(text=self.base_meta, text_color=DIM)
        self._set("En cola", MUTED)

    def working(self, p: float, note: str | None = None):
        self.bar.set(p)
        self.bar.grid()
        self._set(f"{int(p * 100)}%" if p else "…", ACCENT)
        if note:
            self.meta.configure(text=note, text_color=MUTED)

    def progress(self, p):
        self.bar.set(p)
        self._set(f"{int(p * 100)}%", ACCENT)

    def done(self, out: Path, note: str = ""):
        self.bar.grid_remove()
        self._set("Listo", OK)
        try:
            self.size.configure(text=human(out.stat().st_size))
        except OSError:
            pass
        text = (note + " · " if note else "") + "Guardado como " + out.name
        self.meta.configure(text=ellipsize(text, 76), text_color=MUTED)

    def error(self, msg):
        self.bar.grid_remove()
        self._set("Error", ERR)
        self.meta.configure(text=ellipsize(msg, 76), text_color=ERR)

    def cancelled(self):
        self.bar.grid_remove()
        self._set("Cancelado", MUTED)


# ───────────────────────────── Pestañas ─────────────────────────────


class Tab:
    """Una pestaña: su lista de archivos, su zona vacía y sus ajustes. Comparten ventana y destino."""
    key = title = go_text = done_one = done_many = ""
    exts: set[str] = set()
    empty_title = empty_text = pick_title = ""
    filetype = "Audio y vídeo"

    def __init__(self, app: "App"):
        self.app = app
        self.items: list[Item] = []
        self.known: set[Path] = set()

    # se llama una vez para construir la zona de la lista y el panel de ajustes
    def build(self, stage, panel):
        self.empty = ctk.CTkFrame(stage, fg_color="transparent")
        self.empty.grid(row=0, column=0, sticky="nsew", padx=8, pady=8)
        inner = ctk.CTkFrame(self.empty, fg_color="transparent")
        inner.place(relx=0.5, rely=0.47, anchor="center")
        icon = Wave(inner, 110, 60, CARD, bar=8)
        icon.pack(pady=(0, 22))
        t1 = ctk.CTkLabel(inner, text=self.empty_title, font=font(20, "bold"), text_color=TEXT)
        t1.pack()
        t2 = ctk.CTkLabel(inner, text=self.empty_text, font=font(13), text_color=MUTED, justify="center")
        t2.pack(pady=(8, 20))
        ctk.CTkButton(inner, text="Elegir archivos", command=self.app.pick_files, width=170, height=40,
                      corner_radius=12, fg_color=ACCENT, hover_color=ACCENT_H, text_color="#FFFFFF",
                      font=font(14, "bold")).pack()
        for w in (self.empty, inner, icon, t1, t2):
            w.bind("<Button-1>", lambda e: self.app.pick_files())
        self.listf = ctk.CTkScrollableFrame(stage, fg_color="transparent", corner_radius=14,
                                            scrollbar_button_color=BORDER, scrollbar_button_hover_color=HOVER)
        self.listf.grid(row=0, column=0, sticky="nsew", padx=8, pady=8)
        self.settings = ctk.CTkFrame(panel, fg_color="transparent")
        self.settings.grid(row=0, column=0, sticky="ew", padx=20, pady=(18, 0))
        self.build_settings(self.settings)

    def show(self, on: bool):
        if on:
            self.settings.grid()
            self.refresh_view()
        else:
            for w in (self.empty, self.listf, self.settings):
                w.grid_remove()

    def refresh_view(self):
        if self.items:
            self.empty.grid_remove()
            self.listf.grid()
        else:
            self.listf.grid_remove()
            self.empty.grid()

    def relabel(self):
        for it in self.items:
            if it.status == "pending":
                it.row.pending(self.label())

    # a definir en cada pestaña
    def build_settings(self, parent): ...
    def label(self) -> str: return ""
    def signature(self) -> tuple: return ()
    def plan(self, todo: list[Item], out_dir: Path | None) -> None: ...
    def workers(self) -> int: return min(8, max(2, (os.cpu_count() or 4) - 1))
    def lock(self, on: bool): ...
    def save(self) -> dict: return {}
    def load(self, cfg: dict): ...


class AudioTab(Tab):
    key, title, go_text = "audio", "Audio", "Convertir"
    done_one, done_many = "archivo convertido", "archivos convertidos"
    exts = AUDIO_EXT
    empty_title = "Suelta aquí tus archivos de audio"
    empty_text = "MP3, WAV, FLAC, OGG, M4A, WMA, AIFF, OPUS y muchos más.\nTambién puedes soltar carpetas enteras."
    pick_title = "Selecciona archivos de audio"

    def build_settings(self, parent):
        row = ctk.CTkFrame(parent, fg_color="transparent")
        row.pack(fill="x")
        self.fmt = segmented(group(row, "Convertir a"), list(FORMATS), self.on_format)
        self.fmt.pack()
        self.r_menu = option_menu(group(row, "Frecuencia"), list(RATES), 120)
        self.r_menu.master.pack_configure(side="right")
        self.q_menu = option_menu(group(row, "Calidad"), ["-"], 140)
        self.q_menu.master.pack_configure(side="right", padx=(0, 18))
        self.q_menu.pack()
        self.r_menu.pack()

    def on_format(self, fmt: str):
        _, opts, default = FORMATS[fmt]
        self.q_menu.configure(values=list(opts))
        self.q_menu.set(default)
        self.relabel()

    def label(self):
        return self.fmt.get()

    def signature(self):
        return (self.fmt.get(), self.q_menu.get(), self.r_menu.get())

    def plan(self, todo, out_dir):
        fmt, qual = self.fmt.get(), self.q_menu.get()
        ext, opts, _ = FORMATS[fmt]
        rate = RATES[self.r_menu.get()]
        if rate and fmt == "OPUS":
            rate = 48000
        elif rate and fmt == "MP3":
            rate = min(rate, 48000)
        reserved: set[Path] = set()
        for it in todo:
            it.out = unique_path((out_dir or it.path.parent) / (it.path.stem + ext), reserved)
            it.cmd = audio_cmd(it.path, part_path(it.out), list(opts[qual]), rate)

    def lock(self, on):
        st = "disabled" if on else "normal"
        for w in (self.fmt, self.q_menu, self.r_menu):
            w.configure(state=st)

    def save(self):
        return {"fmt": self.fmt.get(), "quality": self.q_menu.get(), "rate": self.r_menu.get()}

    def load(self, cfg):
        fmt = cfg.get("fmt") if cfg.get("fmt") in FORMATS else "WAV"
        self.fmt.set(fmt)
        self.on_format(fmt)
        if cfg.get("quality") in FORMATS[fmt][1]:
            self.q_menu.set(cfg["quality"])
        if cfg.get("rate") in RATES:
            self.r_menu.set(cfg["rate"])


class VideoTab(Tab):
    key, title, go_text = "video", "Vídeo", "Convertir"
    done_one, done_many = "vídeo convertido", "vídeos convertidos"
    exts = VIDEO_EXT
    empty_title = "Suelta aquí tus vídeos"
    empty_text = "MP4, MOV, MKV, AVI, WEBM, WMV y muchos más.\nTambién puedes soltar carpetas enteras."
    pick_title = "Selecciona vídeos"
    filetype = "Vídeos"

    def build_settings(self, parent):
        row1 = ctk.CTkFrame(parent, fg_color="transparent")
        row1.pack(fill="x")
        self.fmt = segmented(group(row1, "Convertir a"), list(CONTAINERS), self.on_container)
        self.fmt.pack()
        self.res = option_menu(group(row1, "Resolución"), list(RESOLUTIONS), 130)
        self.res.master.pack_configure(side="right")
        self.qual = option_menu(group(row1, "Calidad"), QUALITIES, 120)
        self.qual.master.pack_configure(side="right", padx=(0, 18))
        self.qual.set("Alta")
        self.qual.pack()
        self.res.pack()

        row2 = ctk.CTkFrame(parent, fg_color="transparent")
        row2.pack(fill="x", pady=(14, 0))
        self.vc = option_menu(group(row2, "Códec de vídeo"), ["-"], 170, self.on_codec)
        self.vc.pack()
        self.ac = option_menu(group(row2, "Códec de audio"), ["-"], 190, lambda _: self.relabel())
        self.ac.master.pack_configure(padx=(18, 0))
        self.ac.pack()
        self.hint = ctk.CTkLabel(row2, text="", font=font(12), text_color=DIM, justify="left", anchor="w",
                                 wraplength=440)
        self.hint.pack(side="left", padx=(22, 0), anchor="s", pady=(0, 8))

    def on_container(self, name: str):
        _, vlist, alist = CONTAINERS[name]
        vlist = [c for c in vlist if vcodec_ok(c)]
        alist = [c for c in alist if acodec_ok(c)]
        cur_v, cur_a = self.vc.get(), self.ac.get()
        self.vc.configure(values=vlist)
        self.vc.set(cur_v if cur_v in vlist else vlist[0])
        self.ac.configure(values=alist)
        self.ac.set(cur_a if cur_a in alist else alist[0])
        self.on_codec(self.vc.get())

    def on_codec(self, name: str):
        self.hint.configure(text=VCODECS[name][1])
        if not self.app.running:
            st = "disabled" if name == COPY else "normal"
            self.qual.configure(state=st)
            self.res.configure(state=st)
        self.relabel()

    def label(self):
        return self.fmt.get()

    def signature(self):
        return (self.fmt.get(), self.vc.get(), self.ac.get(), self.qual.get(), self.res.get())

    def plan(self, todo, out_dir):
        container = self.fmt.get()
        ext = CONTAINERS[container][0]
        args = video_args(container, self.vc.get(), self.ac.get(), self.qual.get(), self.res.get())
        reserved: set[Path] = set()
        for it in todo:
            it.out = unique_path((out_dir or it.path.parent) / (it.path.stem + ext), reserved)
            it.cmd = video_cmd(it.path, part_path(it.out), args)

    def workers(self):
        if self.vc.get() == COPY:
            return 4  # copiar no recodifica: casi solo lee y escribe el disco
        return max(1, min(3, (os.cpu_count() or 4) // 6))  # cada vídeo ya usa todos los núcleos

    def lock(self, on):
        st = "disabled" if on else "normal"
        for w in (self.fmt, self.vc, self.ac):
            w.configure(state=st)
        st2 = "disabled" if on or self.vc.get() == COPY else "normal"
        self.qual.configure(state=st2)
        self.res.configure(state=st2)

    def save(self):
        return {"fmt": self.fmt.get(), "vc": self.vc.get(), "ac": self.ac.get(),
                "quality": self.qual.get(), "res": self.res.get()}

    def load(self, cfg):
        fmt = cfg.get("fmt") if cfg.get("fmt") in CONTAINERS else "MP4"
        self.fmt.set(fmt)
        if cfg.get("vc") in VCODECS:
            self.vc.set(cfg["vc"])
        if cfg.get("ac") in ACODECS:
            self.ac.set(cfg["ac"])
        self.on_container(fmt)
        if cfg.get("quality") in QUALITIES:
            self.qual.set(cfg["quality"])
        if cfg.get("res") in RESOLUTIONS:
            self.res.set(cfg["res"])


class NormTab(Tab):
    key, title, go_text = "norm", "Normalizar", "Normalizar"
    done_one, done_many = "pista normalizada", "pistas normalizadas"
    exts = AUDIO_EXT
    empty_title = "Suelta aquí las pistas a igualar"
    empty_text = ("Todas quedan al mismo volumen percibido.\n"
                  "Los originales no se tocan: se guarda una copia de cada una.")
    pick_title = "Selecciona las pistas a normalizar"

    def build_settings(self, parent):
        row1 = ctk.CTkFrame(parent, fg_color="transparent")
        row1.pack(fill="x")
        self.target = segmented(group(row1, "Volumen final"), list(TARGETS), self.on_target)
        self.target.pack()
        self.outfmt = option_menu(group(row1, "Guardar como"), list(NORM_OUT), 200, lambda _: self.relabel())
        self.outfmt.master.pack_configure(side="right")
        self.outfmt.pack()
        self.hint = ctk.CTkLabel(parent, text="", font=font(12), text_color=DIM, anchor="w", justify="left")
        self.hint.pack(fill="x", pady=(12, 0))

    def on_target(self, name: str):
        self.hint.configure(text=f"{TARGET_HINTS[name]}  El pico nunca pasa de −1 dB.")
        self.relabel()

    def label(self):
        return self.target.get()

    def signature(self):
        return (self.target.get(), self.outfmt.get())

    def plan(self, todo, out_dir):
        choice = NORM_OUT[self.outfmt.get()]
        reserved: set[Path] = set()
        for it in todo:
            ext = choice[0] if choice else same_format(it.path, None)[0]
            # junto al original lleva «(normalizado)»; en otra carpeta conserva el nombre
            stem = it.path.stem if out_dir else f"{it.path.stem} (normalizado)"
            it.out = unique_path((out_dir or it.path.parent) / (stem + ext), reserved)
            it.meas, it.note = None, ""

    def out_args(self, it: Item) -> list[str]:
        choice = NORM_OUT[self.outfmt.get()]
        return list(choice[1]) if choice else same_format(it.path, it.info.get("depth"))[1]

    def lock(self, on):
        st = "disabled" if on else "normal"
        self.target.configure(state=st)
        self.outfmt.configure(state=st)

    def save(self):
        return {"target": self.target.get(), "out": self.outfmt.get()}

    def load(self, cfg):
        t = cfg.get("target") if cfg.get("target") in TARGETS else "−16 LUFS"
        self.target.set(t)
        self.on_target(t)
        self.outfmt.set(cfg.get("out") if cfg.get("out") in NORM_OUT else SAME)


# ───────────────────────────── Aplicación ─────────────────────────────

BaseWindow = ctk.CTk
if HAS_DND:
    class BaseWindow(ctk.CTk, TkinterDnD.DnDWrapper):  # type: ignore[no-redef]
        pass


class App(BaseWindow):
    def __init__(self):
        super().__init__()
        self.dnd = False
        if HAS_DND:
            try:
                self.TkdndVersion = TkinterDnD._require(self)
                self.dnd = True
            except Exception:
                pass
        self.title("Otaconvert")
        self.after(300, self.set_icon)  # CTk pone su icono a los ~200 ms en Windows; lo reemplazamos después
        self.geometry("960x740")
        self.minsize(920, 680)
        self.configure(fg_color=BG)

        self.q: queue.Queue = queue.Queue()
        self.running = False
        self.job_tab: Tab | None = None
        self.todo: list[Item] = []
        self.finished = 0
        self.phase2 = False
        self.target_used: float | None = None
        self.pool: ThreadPoolExecutor | None = None
        self.cancel_evt = threading.Event()
        self.last_dir: Path | None = None

        cfg = load_cfg()
        d = cfg.get("out_dir")
        self.out_dir = Path(d) if isinstance(d, str) and d and Path(d).is_dir() else None

        self.tabs = {t.key: t for t in (AudioTab(self), VideoTab(self), NormTab(self))}
        self._build()
        audio_cfg = cfg.get("audio") if isinstance(cfg.get("audio"), dict) else cfg  # ajustes de la V1
        self.tabs["audio"].load(audio_cfg)
        for key in ("video", "norm"):
            self.tabs[key].load(cfg.get(key) if isinstance(cfg.get(key), dict) else {})
        self.tab: Tab = self.tabs["audio"]
        self.select(cfg.get("tab") if cfg.get("tab") in self.tabs else "audio")
        self.update_dest()

        if self.dnd:
            self.drop_target_register(DND_FILES)
            self.dnd_bind("<<DropEnter>>", lambda e: (self.stage.configure(border_color=ACCENT), e.action)[1])
            self.dnd_bind("<<DropLeave>>", lambda e: (self.stage.configure(border_color=BORDER), e.action)[1])
            self.dnd_bind("<<Drop>>", self.on_drop)
        self.protocol("WM_DELETE_WINDOW", self.on_close)
        if not FFMPEG:
            self.info.configure(text="No se encontró FFmpeg. Instálalo con: pip install imageio-ffmpeg",
                                text_color=ERR)

    def set_icon(self):
        try:
            if sys.platform == "win32":
                self.iconbitmap(str(resource("otaconvert.ico")))
            else:
                self._icon = PhotoImage(file=str(resource("otaconvert.png")))
                self.iconphoto(True, self._icon)
        except Exception:
            pass

    # ── interfaz ──
    def _build(self):
        self.columnconfigure(0, weight=1)
        self.rowconfigure(1, weight=1)

        head = ctk.CTkFrame(self, fg_color="transparent")
        head.grid(row=0, column=0, sticky="ew", padx=28, pady=(22, 12))
        head.columnconfigure(3, weight=1)
        self.logo = Wave(head, 34, 30, BG, bar=4)
        self.logo.grid(row=0, column=0)
        ctk.CTkLabel(head, text="Otaconvert", font=font(21, "bold"), text_color=TEXT).grid(
            row=0, column=1, padx=(10, 0))

        pill = ctk.CTkFrame(head, fg_color=CARD, corner_radius=12, border_width=1, border_color=BORDER)
        pill.grid(row=0, column=2, padx=(26, 0))
        self.tab_btns = {}
        for i, t in enumerate(self.tabs.values()):
            b = ctk.CTkButton(pill, text=t.title, width=96, height=30, corner_radius=9, font=font(13),
                              fg_color="transparent", hover_color=CARD2, text_color=MUTED,
                              text_color_disabled=DIM, command=lambda k=t.key: self.select(k))
            b.grid(row=0, column=i, padx=(3 if i == 0 else 0, 3), pady=3)
            self.tab_btns[t.key] = b

        self.count = ctk.CTkLabel(head, text="", font=font(13), text_color=MUTED)
        self.count.grid(row=0, column=3, sticky="e", padx=(0, 14))
        self.btn_add = ghost(head, "Añadir", self.pick_files, 90)
        self.btn_add.grid(row=0, column=4, padx=(0, 8))
        self.btn_clear = ghost(head, "Vaciar lista", self.clear, 110)
        self.btn_clear.grid(row=0, column=5)

        # zona principal: vacía (soltar) o lista de archivos, una por pestaña
        self.stage = ctk.CTkFrame(self, fg_color=CARD, corner_radius=18, border_width=1, border_color=BORDER)
        self.stage.grid(row=1, column=0, sticky="nsew", padx=28)
        self.stage.columnconfigure(0, weight=1)
        self.stage.rowconfigure(0, weight=1)

        # panel de ajustes: la parte de arriba cambia con la pestaña, la de abajo es común
        panel = ctk.CTkFrame(self, fg_color=CARD, corner_radius=18, border_width=1, border_color=BORDER)
        panel.grid(row=2, column=0, sticky="ew", padx=28, pady=(14, 6))
        panel.columnconfigure(0, weight=1)
        for t in self.tabs.values():
            t.build(self.stage, panel)
            t.show(False)

        bot = ctk.CTkFrame(panel, fg_color="transparent")
        bot.grid(row=1, column=0, sticky="ew", padx=20, pady=(18, 20))
        bot.columnconfigure(2, weight=1)
        self.dest_btn = ghost(bot, "", self.pick_dest, 210)
        self.dest_btn.grid(row=0, column=0)
        self.dest_x = ctk.CTkButton(bot, text="✕", width=30, height=34, corner_radius=10,
                                    fg_color="transparent", hover_color=HOVER, text_color=MUTED,
                                    text_color_disabled=BORDER, font=font(12), command=self.reset_dest)
        self.dest_x.grid(row=0, column=1, padx=(4, 0))

        status = ctk.CTkFrame(bot, fg_color="transparent")
        status.grid(row=0, column=2, sticky="ew", padx=20)
        status.columnconfigure(0, weight=1)
        self.info = ctk.CTkLabel(status, text="", font=font(13), text_color=MUTED, anchor="w")
        self.info.grid(row=0, column=0, sticky="w")
        self.open_btn = ctk.CTkButton(status, text="Abrir carpeta", width=100, height=24, corner_radius=8,
                                      fg_color="transparent", hover_color=HOVER, text_color=ACCENT,
                                      font=font(12, "bold"),
                                      command=lambda: self.last_dir and open_folder(self.last_dir))
        self.open_btn.grid(row=0, column=1, sticky="e")
        self.open_btn.grid_remove()
        self.total = ctk.CTkProgressBar(status, height=6, fg_color=CARD2, progress_color=CARD2)
        self.total.grid(row=1, column=0, columnspan=2, sticky="ew", pady=(8, 0))
        self.total_on = False

        self.go = ctk.CTkButton(bot, text="Convertir", width=170, height=44, corner_radius=12,
                                fg_color=ACCENT, hover_color=ACCENT_H, text_color="#FFFFFF",
                                text_color_disabled=DIM, font=font(15, "bold"), command=self.toggle)
        self.go.grid(row=0, column=3)

        ctk.CTkLabel(self, text=CREDIT, font=font(11), text_color=DIM).grid(
            row=3, column=0, sticky="e", padx=32, pady=(0, 10))

    def select(self, key: str):
        if self.running:
            return
        self.tab.show(False)
        self.tab = self.tabs[key]
        for k, b in self.tab_btns.items():
            on = k == key
            b.configure(fg_color=HOVER if on else "transparent", text_color=TEXT if on else MUTED,
                        font=font(13, "bold" if on else "normal"))
        self.tab.show(True)
        self.go.configure(text=self.tab.go_text)
        self.refresh()

    # ── destino ──
    def pick_dest(self):
        d = filedialog.askdirectory(title="Carpeta de salida", initialdir=str(self.out_dir or Path.home()))
        if d:
            self.out_dir = Path(d)
            self.update_dest()

    def reset_dest(self):
        self.out_dir = None
        self.update_dest()

    def update_dest(self):
        if self.out_dir:
            self.dest_btn.configure(text="Guardar en " + ellipsize(self.out_dir.name or str(self.out_dir), 18))
            self.dest_x.grid()
        else:
            self.dest_btn.configure(text="Guardar junto al original")
            self.dest_x.grid_remove()

    # ── archivos ──
    def pick_files(self):
        if self.running:
            return
        patterns = tuple(f"*{e}" for e in sorted(self.tab.exts))
        files = filedialog.askopenfilenames(title=self.tab.pick_title,
                                            filetypes=[(self.tab.filetype, patterns), ("Todos los archivos", "*.*")])
        if files:
            self.add_paths(files)

    def on_drop(self, e):
        self.stage.configure(border_color=BORDER)
        if not self.running:
            self.add_paths(self.tk.splitlist(e.data))
        return e.action

    def add_paths(self, paths):
        tab = self.tab
        for p in collect(paths, tab.exts):
            try:
                key = p.resolve()
                size = p.stat().st_size
            except OSError:
                continue
            if key in tab.known:
                continue
            tab.known.add(key)
            it = Item(p, size)
            it.row = Row(tab.listf, it, tab.label(), self.remove)
            it.row.pack(fill="x", pady=3, padx=2)
            tab.items.append(it)
        self.refresh()

    def remove(self, it: Item):
        if self.running and it.status in BUSY:
            return
        tab = next(t for t in self.tabs.values() if it in t.items)
        it.row.destroy()
        tab.items.remove(it)
        tab.known.discard(it.path.resolve())
        self.refresh()

    def clear(self):
        if self.running:
            return
        for it in self.tab.items:
            it.row.destroy()
        self.tab.items.clear()
        self.tab.known.clear()
        self.refresh()

    def refresh(self):
        tab = self.tab
        tab.refresh_view()
        n = len(tab.items)
        self.count.configure(text=f"{plural(n, 'archivo', 'archivos')}, {human(sum(i.size for i in tab.items))}"
                             if n else "")
        if not self.running:
            ready = bool(n and FFMPEG)
            self.go.configure(state="normal" if ready else "disabled", fg_color=ACCENT if ready else CARD2)
            self.btn_clear.configure(state="normal" if n else "disabled")
            self.set_total(0)
            self.open_btn.grid_remove()
            if FFMPEG:
                verb = "normalizar" if tab is self.tabs["norm"] else "convertir"
                self.info.configure(text=f"Listo para {verb}" if n else "Añade archivos para empezar",
                                    text_color=MUTED)

    def set_total(self, v: float):
        on = v > 0
        if on != self.total_on:
            self.total_on = on
            self.total.configure(progress_color=ACCENT if on else CARD2)
        self.total.set(v)

    # ── conversión ──
    def toggle(self):
        self.cancel() if self.running else self.start()

    def start(self):
        tab = self.tab
        sig = (tab.key, *tab.signature(), str(self.out_dir or ""))
        todo = [i for i in tab.items if not (i.status == "done" and i.sig == sig)]
        if todo and isinstance(tab, NormTab) and TARGETS[tab.target.get()] is None:
            todo = list(tab.items)  # la meta del grupo depende de todas las pistas: se rehacen todas
        if not todo:
            self.info.configure(text="Todo está listo con estos ajustes", text_color=MUTED)
            return
        tab.plan(todo, self.out_dir)
        for it in todo:
            it.sig, it.status, it.progress = sig, "queued", 0.0
            it.row.queued()

        self.running, self.job_tab, self.todo, self.finished = True, tab, todo, 0
        self.phase2, self.target_used = False, None
        self.cancel_evt = threading.Event()
        self.pool = ThreadPoolExecutor(max_workers=tab.workers())
        work = self.analyze_worker if isinstance(tab, NormTab) else self.convert_worker
        for it in todo:
            self.pool.submit(work, it)

        self.go.configure(text="Cancelar", fg_color=HOVER, hover_color=BORDER)
        self.btn_clear.configure(state="disabled")
        self.btn_add.configure(state="disabled")
        self.dest_btn.configure(state="disabled")
        self.dest_x.configure(state="disabled")
        for k, b in self.tab_btns.items():
            if k != tab.key:
                b.configure(state="disabled")
        tab.lock(True)
        self.open_btn.grid_remove()
        self.logo.spin(True)
        self.save_cfg()
        self.after(80, self.poll)

    def cancel(self):
        self.cancel_evt.set()
        self.kill_procs()
        self.go.configure(text="Cancelando…", state="disabled")

    def kill_procs(self):
        for it in self.todo:
            p = it.proc
            if p and p.poll() is None:
                try:
                    p.terminate()
                except OSError:
                    pass

    def _set_proc(self, it: Item):
        def on_proc(p):
            it.proc = p
            if self.cancel_evt.is_set():  # se canceló justo mientras arrancaba
                try:
                    p.terminate()
                except OSError:
                    pass
        return on_proc

    # Los workers se ejecutan en hilos: nunca tocan la interfaz, solo envían eventos a la cola.
    def convert_worker(self, it: Item):
        if self.cancel_evt.is_set():
            self.q.put(("cancel", it))
            return
        self.q.put(("start", it, None))
        info = probe(it.path)
        tmp = part_path(it.out)
        rc, tail = run_ffmpeg(adapt_cmd(it.cmd, it.out, info), info["dur"], lambda p: self.q.put(("progress", it, p)), self._set_proc(it))
        self._finish_file(it, tmp, rc, tail)

    def analyze_worker(self, it: Item):
        if self.cancel_evt.is_set():
            self.q.put(("cancel", it))
            return
        self.q.put(("start", it, "Midiendo el volumen…"))
        it.info = probe(it.path)
        rc, tail = run_ffmpeg(loud_analyze_cmd(it.path), it.info["dur"],
                              lambda p: self.q.put(("progress", it, p * 0.4)), self._set_proc(it))
        if self.cancel_evt.is_set():
            self.q.put(("cancel", it))
            return
        meas = parse_loudnorm(tail) if rc == 0 else None
        if not meas:
            self.q.put(("error", it, friendly_error(tail) if rc else "No se pudo medir el volumen"))
            return
        try:
            level = float(meas["input_i"])
        except (KeyError, ValueError):
            level = -math.inf
        if not math.isfinite(level) or level < -60:
            self.q.put(("error", it, "La pista está casi en silencio: no se normaliza"))
            return
        it.meas = meas
        self.q.put(("analyzed", it))

    def render_worker(self, it: Item, target: float, args: list[str]):
        if self.cancel_evt.is_set():
            self.q.put(("cancel", it))
            return
        self.q.put(("start", it, "Normalizando…"))
        tmp = part_path(it.out)
        cmd = loud_render_cmd(it.path, tmp, it.meas, target, args, out_rate(args, it.info.get("rate")))
        rc, tail = run_ffmpeg(cmd, it.info["dur"], lambda p: self.q.put(("progress", it, 0.4 + p * 0.6)),
                              self._set_proc(it))
        if rc == 0:
            res = parse_loudnorm(tail) or {}
            note = f"{lufs(it.meas['input_i'])} → {lufs(res.get('output_i', target))} LUFS"
            if res.get("normalization_type") == "dynamic":
                note += " · con limitador suave"
            it.note = note
        self._finish_file(it, tmp, rc, tail)

    def _finish_file(self, it: Item, tmp: Path, rc: int, tail: list[str]):
        if self.cancel_evt.is_set():
            tmp.unlink(missing_ok=True)
            self.q.put(("cancel", it))
        elif rc == 0 and tmp.exists():
            try:
                os.replace(tmp, it.out)
                self.q.put(("done", it))
            except OSError as exc:
                tmp.unlink(missing_ok=True)
                self.q.put(("error", it, friendly_error([str(exc)])))
        else:
            tmp.unlink(missing_ok=True)
            self.q.put(("error", it, friendly_error(tail)))

    def poll(self):
        try:
            while True:
                kind, it, *rest = self.q.get_nowait()
                if kind == "start":
                    it.status = "working"
                    it.row.working(it.progress, rest[0])
                elif kind == "progress":
                    it.progress = rest[0]
                    it.row.progress(rest[0])
                elif kind == "analyzed":
                    it.status, it.progress = "analyzed", 0.4
                    it.row.progress(0.4)
                elif kind == "done":
                    it.status, it.progress = "done", 1.0
                    it.row.done(it.out, it.note)
                    self.finished += 1
                elif kind == "error":
                    it.status, it.progress, it.sig = "error", 1.0, None
                    it.row.error(rest[0])
                    self.finished += 1
                elif kind == "cancel":
                    it.status, it.progress, it.sig = "cancelled", 1.0, None
                    it.row.cancelled()
                    self.finished += 1
        except queue.Empty:
            pass

        total = len(self.todo)
        if isinstance(self.job_tab, NormTab) and not self.phase2:
            waiting = [i for i in self.todo if i.status == "analyzed"]
            if len(waiting) + self.finished >= total:
                self.start_phase2(waiting)
        self.set_total(sum(i.progress for i in self.todo) / total)
        if self.finished >= total:
            self.finish()
            return
        if not self.cancel_evt.is_set():
            if isinstance(self.job_tab, NormTab):
                if self.phase2:
                    text = f"Normalizando {self.finished} de {total} a {lufs(self.target_used)} LUFS"
                else:
                    text = f"Midiendo el volumen {sum(i.status in ('analyzed', 'error') for i in self.todo)} de {total}"
            else:
                text = f"Convirtiendo {self.finished} de {total}"
            self.info.configure(text=text, text_color=MUTED)
        self.after(80, self.poll)

    def start_phase2(self, ready: list[Item]):
        """Con todas las pistas medidas se decide el volumen final y se escriben las copias."""
        self.phase2 = True
        if not ready:
            return
        if self.cancel_evt.is_set():
            for it in ready:
                self.q.put(("cancel", it))
            return
        target = TARGETS[self.job_tab.target.get()]
        if target is None:  # volumen típico del grupo: la mediana no se deja arrastrar por una pista extrema
            target = group_target([float(i.meas["input_i"]) for i in ready])
        self.target_used = target
        for it in ready:  # los ajustes se leen aquí, en el hilo de la ventana (Tk no admite otros hilos)
            self.pool.submit(self.render_worker, it, target, self.job_tab.out_args(it))

    def finish(self):
        self.running = False
        tab = self.job_tab
        self.pool.shutdown(wait=False)
        ok = [i for i in self.todo if i.status == "done"]
        bad = sum(i.status == "error" for i in self.todo)
        stopped = self.cancel_evt.is_set()
        parts = [plural(len(ok), tab.done_one, tab.done_many)] if ok else []
        if ok and isinstance(tab, NormTab) and self.target_used is not None:
            parts[0] += f" a {lufs(self.target_used)} LUFS"
        if bad:
            parts.append(plural(bad, "con error", "con errores"))
        if stopped:
            parts.insert(0, "Cancelado")
        color = OK if ok and not bad and not stopped else (ERR if bad else MUTED)
        self.info.configure(text=", ".join(parts) or "No se terminó ningún archivo", text_color=color)
        self.go.configure(text=tab.go_text, state="normal", fg_color=ACCENT, hover_color=ACCENT_H)
        self.btn_clear.configure(state="normal")
        self.btn_add.configure(state="normal")
        self.dest_btn.configure(state="normal")
        self.dest_x.configure(state="normal")
        for b in self.tab_btns.values():
            b.configure(state="normal")
        tab.lock(False)
        self.logo.spin(False)
        if ok:
            self.last_dir = self.out_dir or ok[0].out.parent
            self.open_btn.grid()

    # ── ciclo de vida ──
    def save_cfg(self):
        data = {"tab": self.tab.key, "out_dir": str(self.out_dir) if self.out_dir else ""}
        for key, t in self.tabs.items():
            data[key] = t.save()
        data.update(self.tabs["audio"].save())  # también arriba, por si se vuelve a abrir la V1
        try:
            CFG.write_text(json.dumps(data))
        except OSError:
            pass

    def on_close(self):
        if self.running:
            if not messagebox.askyesno("Otaconvert", "Hay un trabajo en curso. ¿Cancelar y salir?"):
                return
            self.cancel_evt.set()
            self.kill_procs()
        self.save_cfg()
        self.destroy()


# ───────────────────────────── Autoprueba ─────────────────────────────

def selftest(report: str | None = None) -> int:
    """Prueba con el FFmpeg incluido: audio a los 7 formatos, vídeo con cada códec y la normalización
    (comprueba que las pistas queden de verdad al volumen pedido). Uso: --selftest [informe.txt]"""
    import tempfile

    lines, ok = [f"Otaconvert {APP_VERSION}", f"FFmpeg: {FFMPEG}"], bool(FFMPEG)

    def check(name: str, good: bool, extra: str = ""):
        nonlocal ok
        ok &= good
        lines.append(f"{name}: {'OK' if good else 'FALLO'}{'  ' + extra if extra else ''}")

    def ff(args: list[str]) -> bool:
        r = subprocess.run([FFMPEG, *BASE, "-loglevel", "error", *args], capture_output=True,
                           stdin=subprocess.DEVNULL, creationflags=NOWINDOW)
        return r.returncode == 0

    if FFMPEG:
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            # audio: un tono convertido a cada formato
            tone = d / "tono.wav"
            ff(["-f", "lavfi", "-i", "sine=frequency=440:duration=1", "-c:a", "pcm_s16le", str(tone)])
            for name, (ext, opts, default) in FORMATS.items():
                dst = d / f"audio{ext}"
                rc, _ = run_ffmpeg(audio_cmd(tone, dst, opts[default], progress=False), None)
                check(f"Audio {name}", rc == 0 and dst.exists() and dst.stat().st_size > 0)

            # vídeo: medidas impares a propósito, con sonido
            clip = d / "clip.mkv"
            ff(["-f", "lavfi", "-i", "testsrc2=size=322x182:rate=25:duration=2", "-f", "lavfi",
                "-i", "sine=frequency=330:duration=2", "-shortest", "-c:v", "libx264", "-pix_fmt", "yuv444p",
                "-c:a", "aac", str(clip)])
            for container, (ext, vlist, alist) in CONTAINERS.items():
                for vc in [c for c in vlist if vcodec_ok(c)]:
                    ac = next(c for c in alist if acodec_ok(c) and c not in (COPY, NOAUDIO))
                    if vc == COPY:
                        ac = COPY
                        if container == "AVI":  # H.264 con AAC del clip: el AAC no va en AVI
                            ac = "MP3"
                    dst = d / f"video-{container}-{vc.replace('.', '')}{ext}".replace(" ", "")
                    cmd = video_cmd(clip, dst, video_args(container, vc, ac, "Media", "Original"), progress=False)
                    rc, tail = run_ffmpeg(adapt_cmd(cmd, dst, probe(clip)), None)
                    info = probe(dst) if dst.exists() else {}
                    good = rc == 0 and info.get("video") and info.get("rate")
                    check(f"Vídeo {container} {vc} + {ac}", bool(good), "" if good else friendly_error(tail))
            dst = d / "video-720.mp4"
            run_ffmpeg(video_cmd(clip, dst, video_args("MP4", "H.264", "AAC", "Media", "720p"), progress=False), None)
            r = subprocess.run([FFMPEG, "-hide_banner", "-i", str(dst)], capture_output=True, text=True,
                               stdin=subprocess.DEVNULL, creationflags=NOWINDOW)
            check("Vídeo sin estirar (720p sobre 322x182 queda 322x182)", "322x182" in r.stderr)

            # normalizar: dos pistas a volúmenes muy distintos, con dinámica (tono que sube y baja)
            pistas = []
            for name, vol in (("suave", 0.05), ("fuerte", 0.6)):
                p = d / f"{name}.wav"
                ff(["-f", "lavfi", "-i", "sine=frequency=500:duration=12", "-af",
                    f"volume='{vol}*(0.6+0.4*sin(2*PI*t/4))':eval=frame", "-c:a", "pcm_s24le", str(p)])
                pistas.append(p)
            for label, target in (("−16 LUFS", -16.0), ("−23 LUFS", -23.0), ("Promedio", None)):
                meas = []
                for p in pistas:
                    rc, tail = run_ffmpeg(loud_analyze_cmd(p, progress=False), None)
                    meas.append(parse_loudnorm(tail))
                goal = target if target is not None else group_target([float(m["input_i"]) for m in meas])
                results = []
                for p, m in zip(pistas, meas):
                    out = d / f"{p.stem}-norm.wav"
                    args = same_format(p, probe(p)["depth"])[1]
                    run_ffmpeg(loud_render_cmd(p, out, m, goal, args, 48000, progress=False), None)
                    rc, tail = run_ffmpeg(loud_analyze_cmd(out, progress=False), None)
                    res = parse_loudnorm(tail) or {}
                    results.append((float(res.get("input_i", 0)), float(res.get("input_tp", 99)), probe(out)))
                good = all(abs(i - goal) <= 1.0 and tp <= TRUE_PEAK + 0.5 and info["rate"] == 48000
                           and info["depth"] == 24 for i, tp, info in results)
                check(f"Normalizar {label}", good,
                      ", ".join(f"{lufs(i)} LUFS pico {lufs(tp)}" for i, tp, _ in results) + f" (meta {lufs(goal)})")
    try:  # la interfaz y el arrastrar/soltar degradan con elegancia, no hacen fallar la prueba
        root = ctk.CTk()
        root.withdraw()
        dnd = False
        if HAS_DND:
            TkinterDnD._require(root)
            dnd = True
        root.destroy()
        lines.append(f"Interfaz: OK, arrastrar y soltar: {'sí' if dnd else 'no'}")
    except Exception as exc:  # noqa: BLE001
        lines.append(f"Interfaz: no verificada ({exc})")
    text = "\n".join(lines)
    if report:
        Path(report).write_text(text, encoding="utf-8")
    elif sys.stdout:
        print(text)
    return 0 if ok else 1


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--selftest":
        sys.exit(selftest(sys.argv[2] if len(sys.argv) > 2 else None))
    if len(sys.argv) > 1 and sys.argv[1] == "--version":
        if sys.stdout:
            print(f"Otaconvert {APP_VERSION} · {CREDIT}")
        sys.exit(0)
    ctk.set_appearance_mode("dark")
    App().mainloop()
