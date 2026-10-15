#!/usr/bin/env python3
"""Otaconvert: conversor de audio por lotes (FFmpeg + CustomTkinter)."""
from __future__ import annotations

import functools
import json
import math
import os
import queue
import re
import shutil
import struct
import subprocess
import sys
import threading
from collections import deque
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path
from tkinter import Canvas, PhotoImage, filedialog, messagebox

import customtkinter as ctk

try:
    from tkinterdnd2 import DND_FILES, TkinterDnD
    HAS_DND = True
except Exception:  # sin arrastrar y soltar la app sigue funcionando
    HAS_DND = False

# ───────────────────────────── Ajustes ─────────────────────────────

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


# ───────────────────────────── Utilidades ─────────────────────────────

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
def ffmpeg_cmd(src: Path, dst: Path, args: list[str], rate: int | None = None, progress: bool = True) -> list[str]:
    cmd = [FFMPEG, "-hide_banner", "-nostdin", "-y", "-i", str(src), "-vn", "-map", "0:a:0",
           "-map_metadata", "0", *args]
    if rate:
        cmd += ["-ar", str(rate)]
    if progress:
        cmd += ["-progress", "pipe:1", "-nostats"]
    return cmd + [str(dst)]


_DUR = re.compile(r"Duration:\s*(\d+):(\d+):(\d+(?:\.\d+)?)")
_NOISE = re.compile(r"^(frame|fps|stream_\d+_\d+_q|bitrate|total_size|out_time\w*|dup_frames|"
                    r"drop_frames|speed|progress)=")


def probe_duration(src: Path) -> float | None:
    try:
        r = subprocess.run([FFMPEG, "-hide_banner", "-i", str(src)], capture_output=True, text=True, stdin=subprocess.DEVNULL,
                           encoding="utf-8", errors="replace", creationflags=NOWINDOW, timeout=30)
        m = _DUR.search(r.stderr)
        return int(m[1]) * 3600 + int(m[2]) * 60 + float(m[3]) if m else None
    except Exception:
        return None


def friendly_error(tail: list[str]) -> str:
    t = " ".join(tail).lower()
    if "matches no streams" in t or "does not contain any stream" in t:
        return "El archivo no tiene pista de audio"
    if "invalid data found" in t:
        return "Archivo dañado o formato no compatible"
    if "permission denied" in t:
        return "Sin permiso para escribir en la carpeta"
    if "no such file" in t:
        return "No se encuentra el archivo"
    return (tail[-1] if tail else "Error desconocido")[:90]


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


def collect(paths) -> list[Path]:
    out: list[Path] = []
    for raw in paths:
        p = Path(raw)
        if p.is_dir():
            out += sorted(f for f in p.rglob("*") if f.is_file() and not f.name.startswith(".")
                          and f.suffix.lower() in AUDIO_EXT)
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
        return json.loads(CFG.read_text())
    except Exception:
        return {}


@dataclass
class Item:
    path: Path
    size: int
    status: str = "pending"  # pending | queued | working | done | error | cancelled
    progress: float = 0.0
    out: Path | None = None
    sig: tuple | None = None
    proc: subprocess.Popen | None = None
    row: "Row | None" = None


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


class Row(ctk.CTkFrame):
    def __init__(self, master, item: Item, fmt: str, on_remove):
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
        self.pending(fmt)

    def _set(self, text, color):
        self.state.configure(text=text, text_color=color)

    def pending(self, fmt):
        self.bar.grid_remove()
        self._set(fmt, DIM)

    def queued(self):
        self.bar.grid_remove()
        self.meta.configure(text=self.base_meta, text_color=DIM)
        self._set("En cola", MUTED)

    def working(self):
        self.bar.set(0)
        self.bar.grid()
        self._set("…", ACCENT)

    def progress(self, p):
        self.bar.set(p)
        self._set(f"{int(p * 100)}%", ACCENT)

    def done(self, out: Path):
        self.bar.grid_remove()
        self._set("Listo", OK)
        try:
            self.size.configure(text=human(out.stat().st_size))
        except OSError:
            pass
        self.meta.configure(text=f"Guardado como {ellipsize(out.name, 56)}", text_color=MUTED)

    def error(self, msg):
        self.bar.grid_remove()
        self._set("Error", ERR)
        self.meta.configure(text=msg, text_color=ERR)

    def cancelled(self):
        self.bar.grid_remove()
        self._set("Cancelado", MUTED)


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
        self.geometry("940x700")
        self.minsize(900, 620)
        self.configure(fg_color=BG)

        self.items: list[Item] = []
        self.known: set[Path] = set()
        self.q: queue.Queue = queue.Queue()
        self.running = False
        self.todo: list[Item] = []
        self.finished = 0
        self.pool: ThreadPoolExecutor | None = None
        self.cancel_evt = threading.Event()
        self.last_dir: Path | None = None

        cfg = load_cfg()
        d = cfg.get("out_dir")
        self.out_dir = Path(d) if d and Path(d).is_dir() else None

        self._build()
        fmt = cfg.get("fmt") if cfg.get("fmt") in FORMATS else "WAV"
        self.fmt.set(fmt)
        self.on_format(fmt)
        if cfg.get("quality") in FORMATS[fmt][1]:
            self.q_menu.set(cfg["quality"])
        if cfg.get("rate") in RATES:
            self.r_menu.set(cfg["rate"])
        self.update_dest()
        self.refresh()

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
        head.columnconfigure(2, weight=1)
        self.logo = Wave(head, 34, 30, BG, bar=4)
        self.logo.grid(row=0, column=0)
        ctk.CTkLabel(head, text="Otaconvert", font=font(21, "bold"), text_color=TEXT).grid(
            row=0, column=1, padx=(10, 0))
        self.count = ctk.CTkLabel(head, text="", font=font(13), text_color=MUTED)
        self.count.grid(row=0, column=2, sticky="e", padx=(0, 14))
        self.btn_add = ghost(head, "Añadir", self.pick_files, 90)
        self.btn_add.grid(row=0, column=3, padx=(0, 8))
        self.btn_clear = ghost(head, "Vaciar lista", self.clear, 110)
        self.btn_clear.grid(row=0, column=4)

        # zona principal: vacía (soltar) o lista de archivos
        self.stage = ctk.CTkFrame(self, fg_color=CARD, corner_radius=18, border_width=1, border_color=BORDER)
        self.stage.grid(row=1, column=0, sticky="nsew", padx=28)
        self.stage.columnconfigure(0, weight=1)
        self.stage.rowconfigure(0, weight=1)

        self.empty = ctk.CTkFrame(self.stage, fg_color="transparent")
        self.empty.grid(row=0, column=0, sticky="nsew", padx=8, pady=8)
        inner = ctk.CTkFrame(self.empty, fg_color="transparent")
        inner.place(relx=0.5, rely=0.47, anchor="center")
        icon = Wave(inner, 110, 60, CARD, bar=8)
        icon.pack(pady=(0, 22))
        t1 = ctk.CTkLabel(inner, text="Suelta aquí tus archivos de audio", font=font(20, "bold"), text_color=TEXT)
        t1.pack()
        t2 = ctk.CTkLabel(inner, text="MP3, WAV, FLAC, OGG, M4A, WMA, AIFF, OPUS y muchos más.\n"
                                      "También puedes soltar carpetas enteras.",
                          font=font(13), text_color=MUTED, justify="center")
        t2.pack(pady=(8, 20))
        ctk.CTkButton(inner, text="Elegir archivos", command=self.pick_files, width=170, height=40,
                      corner_radius=12, fg_color=ACCENT, hover_color=ACCENT_H, text_color="#FFFFFF",
                      font=font(14, "bold")).pack()
        for w in (self.empty, inner, icon, t1, t2):
            w.bind("<Button-1>", lambda e: self.pick_files())

        self.listf = ctk.CTkScrollableFrame(self.stage, fg_color="transparent", corner_radius=14,
                                            scrollbar_button_color=BORDER, scrollbar_button_hover_color=HOVER)
        self.listf.grid(row=0, column=0, sticky="nsew", padx=8, pady=8)
        self.listf.grid_remove()

        # panel de ajustes
        panel = ctk.CTkFrame(self, fg_color=CARD, corner_radius=18, border_width=1, border_color=BORDER)
        panel.grid(row=2, column=0, sticky="ew", padx=28, pady=(14, 26))
        panel.columnconfigure(0, weight=1)

        top = ctk.CTkFrame(panel, fg_color="transparent")
        top.grid(row=0, column=0, sticky="ew", padx=20, pady=(18, 0))
        top.columnconfigure(1, weight=1)

        def group(col, caption):
            f = ctk.CTkFrame(top, fg_color="transparent")
            f.grid(row=0, column=col, sticky="w" if col == 0 else "e", padx=(18 if col == 3 else 0, 0))
            ctk.CTkLabel(f, text=caption, font=font(12), text_color=MUTED).pack(anchor="w", pady=(0, 6))
            return f

        self.fmt = ctk.CTkSegmentedButton(
            group(0, "Convertir a"), values=list(FORMATS), command=self.on_format, height=34,
            corner_radius=10, font=font(12, "bold"), fg_color=CARD2, selected_color=ACCENT,
            selected_hover_color=ACCENT, unselected_color=CARD2, unselected_hover_color=HOVER,
            text_color=TEXT)
        self.fmt.pack()

        def menu(col, caption, values, w):
            m = ctk.CTkOptionMenu(group(col, caption), values=values, width=w, height=34, corner_radius=10,
                                  font=font(13), dropdown_font=font(13), fg_color=CARD2,
                                  button_color=CARD2, button_hover_color=HOVER, dropdown_fg_color=CARD2,
                                  dropdown_hover_color=HOVER, text_color=TEXT, dropdown_text_color=TEXT)
            m.pack()
            return m

        self.q_menu = menu(2, "Calidad", ["-"], 140)
        self.r_menu = menu(3, "Frecuencia", list(RATES), 120)

        bot = ctk.CTkFrame(panel, fg_color="transparent")
        bot.grid(row=1, column=0, sticky="ew", padx=20, pady=(18, 20))
        bot.columnconfigure(2, weight=1)
        self.dest_btn = ghost(bot, "", self.pick_dest, 210)
        self.dest_btn.grid(row=0, column=0)
        self.dest_x = ctk.CTkButton(bot, text="✕", width=30, height=34, corner_radius=10,
                                    fg_color="transparent", hover_color=HOVER, text_color=MUTED,
                                    font=font(12), command=self.reset_dest)
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

    # ── ajustes ──
    def on_format(self, fmt: str):
        _, opts, default = FORMATS[fmt]
        self.q_menu.configure(values=list(opts))
        self.q_menu.set(default)
        for it in self.items:
            if it.status == "pending":
                it.row.pending(fmt)

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
        patterns = tuple(f"*{e}" for e in sorted(AUDIO_EXT))
        files = filedialog.askopenfilenames(title="Selecciona archivos de audio",
                                            filetypes=[("Audio y vídeo", patterns), ("Todos los archivos", "*.*")])
        if files:
            self.add_paths(files)

    def on_drop(self, e):
        self.stage.configure(border_color=BORDER)
        self.add_paths(self.tk.splitlist(e.data))
        return e.action

    def add_paths(self, paths):
        fmt = self.fmt.get()
        for p in collect(paths):
            key = p.resolve()
            if key in self.known:
                continue
            try:
                size = p.stat().st_size
            except OSError:
                continue
            self.known.add(key)
            it = Item(p, size)
            it.row = Row(self.listf, it, fmt, self.remove)
            it.row.pack(fill="x", pady=3, padx=2)
            self.items.append(it)
        self.refresh()

    def remove(self, it: Item):
        if self.running and it.status in ("queued", "working"):
            return
        it.row.destroy()
        self.items.remove(it)
        self.known.discard(it.path.resolve())
        self.refresh()

    def clear(self):
        if self.running:
            return
        for it in self.items:
            it.row.destroy()
        self.items.clear()
        self.known.clear()
        self.refresh()

    def refresh(self):
        n = len(self.items)
        if n:
            self.empty.grid_remove()
            self.listf.grid()
        else:
            self.listf.grid_remove()
            self.empty.grid()
        self.count.configure(text=f"{plural(n, 'archivo', 'archivos')}, {human(sum(i.size for i in self.items))}"
                             if n else "")
        if not self.running:
            ready = bool(n and FFMPEG)
            self.go.configure(state="normal" if ready else "disabled", fg_color=ACCENT if ready else CARD2)
            self.btn_clear.configure(state="normal" if n else "disabled")
            self.set_total(0)
            self.open_btn.grid_remove()
            if FFMPEG:
                self.info.configure(text="Listo para convertir" if n else "Añade archivos para empezar",
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
        fmt, qual, rate_label = self.fmt.get(), self.q_menu.get(), self.r_menu.get()
        ext, opts, _ = FORMATS[fmt]
        sig = (fmt, qual, rate_label, str(self.out_dir or ""))
        todo = [i for i in self.items if not (i.status == "done" and i.sig == sig)]
        if not todo:
            self.info.configure(text="Todo está convertido con estos ajustes", text_color=MUTED)
            return

        rate = RATES[rate_label]
        if rate and fmt == "OPUS":
            rate = 48000
        elif rate and fmt == "MP3":
            rate = min(rate, 48000)
        args = list(opts[qual])

        reserved: set[Path] = set()
        for it in todo:
            folder = self.out_dir or it.path.parent
            it.out = unique_path(folder / (it.path.stem + ext), reserved)
            it.sig, it.status, it.progress = sig, "queued", 0.0
            it.row.queued()

        self.running, self.todo, self.finished = True, todo, 0
        self.cancel_evt = threading.Event()
        self.pool = ThreadPoolExecutor(max_workers=min(8, max(2, (os.cpu_count() or 4) - 1)))
        for it in todo:
            self.pool.submit(self.worker, it, args, rate)

        self.go.configure(text="Cancelar", fg_color=HOVER, hover_color=BORDER)
        self.btn_clear.configure(state="disabled")
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

    def worker(self, it: Item, args: list[str], rate: int | None):
        """Se ejecuta en un hilo: nunca toca la interfaz, solo envía eventos a la cola."""
        if self.cancel_evt.is_set():
            self.q.put(("cancel", it))
            return
        self.q.put(("start", it))
        dur = probe_duration(it.path)
        tmp = it.out.with_name(it.out.stem + ".part" + it.out.suffix)
        cmd = ffmpeg_cmd(it.path, tmp, args, rate)
        tail: deque[str] = deque(maxlen=12)
        rc = -1
        try:
            it.proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL, text=True,
                                       encoding="utf-8", errors="replace", creationflags=NOWINDOW)
            last = 0.0
            for line in it.proc.stdout:
                line = line.strip()
                if line.startswith(("out_time_us=", "out_time_ms=")):
                    try:
                        t = int(line.split("=")[1]) / 1e6
                    except ValueError:
                        continue
                    if dur and (p := min(0.99, t / dur)) - last >= 0.01:
                        last = p
                        self.q.put(("progress", it, p))
                elif line and not _NOISE.match(line):
                    tail.append(line)
            rc = it.proc.wait()
        except Exception as exc:  # noqa: BLE001
            tail.append(str(exc))

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
            self.q.put(("error", it, friendly_error(list(tail))))

    def poll(self):
        try:
            while True:
                kind, it, *rest = self.q.get_nowait()
                if kind == "start":
                    it.status = "working"
                    it.row.working()
                elif kind == "progress":
                    it.progress = rest[0]
                    it.row.progress(rest[0])
                elif kind == "done":
                    it.status, it.progress = "done", 1.0
                    it.row.done(it.out)
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
        self.set_total(sum(i.progress for i in self.todo) / total)
        if self.finished >= total:
            self.finish()
        else:
            if not self.cancel_evt.is_set():
                self.info.configure(text=f"Convirtiendo {self.finished} de {total}", text_color=MUTED)
            self.after(80, self.poll)

    def finish(self):
        self.running = False
        self.pool.shutdown(wait=False)
        ok = [i for i in self.todo if i.status == "done"]
        bad = sum(i.status == "error" for i in self.todo)
        stopped = self.cancel_evt.is_set()
        parts = [plural(len(ok), "archivo convertido", "archivos convertidos")] if ok else []
        if bad:
            parts.append(plural(bad, "con error", "con errores"))
        if stopped:
            parts.insert(0, "Cancelado")
        color = OK if ok and not bad and not stopped else (ERR if bad else MUTED)
        self.info.configure(text=", ".join(parts) or "No se convirtió ningún archivo", text_color=color)
        self.go.configure(text="Convertir", state="normal", fg_color=ACCENT, hover_color=ACCENT_H)
        self.btn_clear.configure(state="normal")
        self.logo.spin(False)
        if ok:
            self.last_dir = self.out_dir or ok[0].out.parent
            self.open_btn.grid()

    # ── ciclo de vida ──
    def save_cfg(self):
        try:
            CFG.write_text(json.dumps({"fmt": self.fmt.get(), "quality": self.q_menu.get(),
                                       "rate": self.r_menu.get(),
                                       "out_dir": str(self.out_dir) if self.out_dir else ""}))
        except OSError:
            pass

    def on_close(self):
        if self.running:
            if not messagebox.askyesno("Otaconvert", "Hay una conversión en curso. ¿Cancelar y salir?"):
                return
            self.cancel_evt.set()
            self.kill_procs()
        self.save_cfg()
        self.destroy()


def selftest(report: str | None = None) -> int:
    """Convierte un tono a todos los formatos con el FFmpeg incluido. Uso: --selftest [informe.txt]"""
    import tempfile
    import wave
    lines, ok = [f"FFmpeg: {FFMPEG}"], bool(FFMPEG)
    if FFMPEG:
        with tempfile.TemporaryDirectory() as d:
            src = Path(d) / "tono.wav"
            with wave.open(str(src), "wb") as w:
                w.setnchannels(1)
                w.setsampwidth(2)
                w.setframerate(44100)
                w.writeframes(b"".join(struct.pack("<h", int(12000 * math.sin(2 * math.pi * 440 * i / 44100)))
                                       for i in range(44100)))
            for name, (ext, opts, default) in FORMATS.items():
                dst = Path(d) / f"salida{ext}"
                r = subprocess.run(ffmpeg_cmd(src, dst, opts[default], progress=False), capture_output=True,
                                   stdin=subprocess.DEVNULL, creationflags=NOWINDOW)
                good = r.returncode == 0 and dst.exists() and dst.stat().st_size > 0
                ok &= good
                lines.append(f"{name}: {'OK' if good else 'FALLO'}")
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
    ctk.set_appearance_mode("dark")
    App().mainloop()
