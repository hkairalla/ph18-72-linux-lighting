from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

APP_ID = "ph18-lighting"


def _configure_wayland_env() -> None:
    """Make GTK/WebKit behave under Hyprland (Wayland, fractional scale, NVIDIA).

    Must run before GTK or WebKit is imported.
    - GDK_SCALE is an integer scale for X11 and stacks badly on Wayland: with
      the compositor at 1.6x and GDK_SCALE=2, clicks land away from the cursor.
      Wayland fractional scaling comes from the compositor, so drop it.
    - The WebKitGTK DMA-BUF renderer crashes on NVIDIA with a Wayland protocol
      error at window creation; the fallback renderer is fine for this UI.
    """
    os.environ.setdefault("PYWEBVIEW_GUI", "gtk")
    if os.environ.get("XDG_SESSION_TYPE") == "wayland" or os.environ.get("WAYLAND_DISPLAY"):
        os.environ.pop("GDK_SCALE", None)
        os.environ.pop("GDK_DPI_SCALE", None)
        os.environ.setdefault("WEBKIT_DISABLE_DMABUF_RENDERER", "1")


_configure_wayland_env()

import webview

try:
    # Sets the Wayland app_id (Hyprland "class"), so window rules can match it
    # instead of the default "main.py".
    from gi.repository import GLib

    GLib.set_prgname(APP_ID)
    GLib.set_application_name("PH18-72 Lighting")
except Exception:
    pass

APP_DIR  = Path(__file__).resolve().parent
REPO_ROOT = APP_DIR.parents[2]
DAEMON_DIR = REPO_ROOT / "daemon"
DAEMON_BIN = DAEMON_DIR / "target" / "debug" / "ph18-lighting-daemon"
UI_INDEX   = APP_DIR / "ui" / "index.html"

# ── Python API (exposed to JS) ───────────────────────────────────────
class Api:
    def __init__(self) -> None:
        self._backend = self._detect_backend()
        self._font_key: object = None
        self._font_family = ""

    def _detect_backend(self) -> str:
        requested = os.environ.get("PH18_UI_BACKEND", "").strip().lower()
        if requested in ("mock", "cargo"):
            return requested
        if shutil.which("cargo") and DAEMON_DIR.exists():
            return "cargo"
        return "mock"

    def get_backend_mode(self) -> str:
        return self._backend

    # ── Daemon commands ───────────────────────────────────────────────
    def run_daemon(self, args: list[str]) -> dict:
        """Run a daemon subcommand. Returns {ok, title, output}."""
        title = str(args[0]) if args else "unknown"

        if self._backend == "mock":
            return {"ok": True, "title": title, "output": f"mock: {' '.join(str(a) for a in args)}"}

        cmd = (
            [str(DAEMON_BIN), *(str(a) for a in args)]
            if DAEMON_BIN.exists()
            else ["cargo", "run", "--quiet", "--", *(str(a) for a in args)]
        )
        result = subprocess.run(
            cmd, cwd=DAEMON_DIR, text=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
        )
        output = "\n".join(filter(None, [result.stdout.strip(), result.stderr.strip()])) or "(no output)"
        return {"ok": result.returncode == 0, "title": title, "output": output}

    def _system_mono_font(self) -> str:
        """The fontconfig `monospace` family, which is what Omarchy's shell draws in and what
        `omarchy font set` changes. Cached until ~/.config/fontconfig/fonts.conf changes."""
        conf = Path.home() / ".config" / "fontconfig" / "fonts.conf"
        try:
            key: object = conf.stat().st_mtime_ns
        except OSError:
            key = 0
        if key != self._font_key:
            try:
                out = subprocess.run(["fc-match", "monospace", "-f", "%{family}\n"], text=True,
                                     stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                     check=False, timeout=3).stdout
                self._font_family = out.splitlines()[0].split(",")[0].strip() if out else ""
            except (OSError, subprocess.SubprocessError):
                self._font_family = ""
            self._font_key = key
        return self._font_family

    def get_theme(self) -> dict:
        """The active Omarchy theme, for the GUI to mirror. {} when there is none
        (not Omarchy, mock mode, unreadable file): the GUI then keeps its own palette.

        Reads ~/.local/state/omarchy/current/theme/colors.toml, which Omarchy rewrites
        whenever the theme changes. Only the chrome is themed; the key colors never are.
        """
        if self._backend == "mock":
            return {}
        theme_dir = Path.home() / ".local" / "state" / "omarchy" / "current"
        try:
            data = tomllib.loads((theme_dir / "theme" / "colors.toml").read_text())
        except (OSError, tomllib.TOMLDecodeError):
            return {}
        colors = {k: v for k, v in data.items()
                  if isinstance(v, str) and len(v) in (4, 7) and v.startswith("#")}
        if "accent" not in colors or "background" not in colors or "foreground" not in colors:
            return {}
        try:
            name = (theme_dir / "theme.name").read_text().strip()
        except OSError:
            name = ""
        return {"name": name, "mode": "light" if data.get("mode") == "light" else "dark", "colors": colors,
                "font": self._system_mono_font()}

    def get_keepalive(self) -> dict:
        """Keep-alive settings plus whether its background service is running."""
        if self._backend == "mock":
            return {"mode": "active", "minutes": 5, "ac_only": True,
                    "screensaver_seconds": 150, "service_active": True}
        cfg: dict = {}
        for line in self._daemon_lines(["get-keepalive"]):
            key, _, value = line.partition("=")
            if key in ("mode",):
                cfg[key] = value
            elif key in ("minutes", "screensaver_seconds"):
                cfg[key] = int(value) if value.isdigit() else 0
            elif key == "ac_only":
                cfg[key] = value == "true"
        if "mode" not in cfg:
            return {}
        try:
            probe = subprocess.run(
                ["systemctl", "--user", "is-active", "ph18-lighting-keepalive.service"],
                text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False, timeout=3)
            cfg["service_active"] = probe.stdout.strip() == "active"
        except Exception:
            cfg["service_active"] = None   # unknown: do not nag
        return cfg

    def _daemon_lines(self, args: list[str]) -> list[str]:
        result = self.run_daemon(args)
        return result["output"].splitlines() if result["ok"] else []

    def get_keyboard_view(self, names: list[str]) -> dict:
        """Colors the keyboard is showing, keyed by UI key name.

        The firmware is write-only, so this is the daemon's persisted state
        ({baseline, overrides}), resolved to per-key colors.
        Returns {baseline: [r, g, b], keys: {name: [r, g, b]}}.
        """
        if self._backend == "mock":
            base = [0, 0, 255]
            return {"baseline": base, "keys": {n: base for n in names} | {"q": [255, 60, 60], "e": [60, 255, 90]}}

        indices: dict[str, int] = {}
        for line in self._daemon_lines(["keyboard-key-indices", *names]):
            name, _, value = line.partition("=")
            if value.isdigit():
                indices[name] = int(value)

        baseline: list[int] | None = None
        overrides: dict[int, list[int]] = {}
        magkeys: list[list[int]] | None = None
        for line in self._daemon_lines(["get-keyboard-state"]):
            if line.startswith("baseline_rgb="):
                baseline = [int(v) for v in line.split("=", 1)[1].split(",")]
            elif line.startswith("magkey="):
                zone, _, rgb = line.split("=", 1)[1].partition(":")
                if magkeys is None:
                    magkeys = [[0, 0, 0] for _ in range(12)]
                magkeys[int(zone)] = [int(v) for v in rgb.split(",")]
            elif line.startswith("override="):
                index, _, rgb = line.split("=", 1)[1].partition(":")
                overrides[int(index)] = [int(v) for v in rgb.split(",")]

        # If the state command failed we know nothing: report no colors rather than
        # painting every key with an invented baseline.
        if baseline is None:
            return {"baseline": None, "keys": {}, "magkeys": None}

        keys = {name: overrides.get(index, baseline) for name, index in indices.items()}
        return {"baseline": baseline, "keys": keys, "magkeys": magkeys}

    def get_animation(self) -> dict:
        """The MagKey animation the background service is running: {mode, speed, epoch_ms, phase0,
        keepalive_mode, service_active}. mode is "none" when nothing is animating. The GUI only
        previews it; the daemon owns the clock, so closing the GUI does not stop it."""
        if self._backend == "mock":
            return {"mode": "none", "speed": 1.0, "epoch_ms": 0, "phase0": 0.0,
                    "keepalive_mode": "active", "service_active": True}
        cfg: dict = {}
        for line in self._daemon_lines(["get-animation"]):
            key, _, value = line.partition("=")
            try:
                if key in ("speed", "phase0"):
                    cfg[key] = float(value)
                elif key == "epoch_ms":
                    cfg[key] = int(value)
                elif key in ("mode", "keepalive_mode"):
                    cfg[key] = value
            except ValueError:
                pass
        if "mode" not in cfg:
            return {}
        try:
            probe = subprocess.run(
                ["systemctl", "--user", "is-active", "ph18-lighting-keepalive.service"],
                text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False, timeout=3)
            cfg["service_active"] = probe.stdout.strip() == "active"
        except Exception:
            cfg["service_active"] = None
        return cfg


def main() -> None:
    api = Api()
    window = webview.create_window(
        title="PH18-72 Lighting",
        url=str(UI_INDEX),
        js_api=api,
        width=1100,
        height=720,
        min_size=(900, 580),
        background_color="#07090e",
        text_select=False,
    )
    webview.start(debug="--debug" in sys.argv)


if __name__ == "__main__":
    main()
