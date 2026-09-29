from __future__ import annotations

import array
import fcntl
import os
import shutil
import subprocess
import sys
import threading
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

# ── HID constants ────────────────────────────────────────────────────
TARGET_HID_ID = "0003:000005AF:0000866A"
INIT_PACKETS  = [
    bytes.fromhex("8800000000000077"),
    bytes.fromhex("b10000000000004e"),
    bytes.fromhex("08020000000000f5"),
    bytes.fromhex("08024f0a3200006a"),
    bytes.fromhex("14000100000000ea"),
    bytes.fromhex("13000008000000e4"),
]
COMMIT_PACKET = bytes.fromhex("08024f0532080166")

_IOC_NRBITS   = 8
_IOC_TYPEBITS = 8
_IOC_SIZEBITS = 14
_IOC_NRSHIFT  = 0
_IOC_TYPESHIFT = _IOC_NRSHIFT  + _IOC_NRBITS
_IOC_SIZESHIFT = _IOC_TYPESHIFT + _IOC_TYPEBITS
_IOC_DIRSHIFT  = _IOC_SIZESHIFT + _IOC_SIZEBITS
_IOC_READ  = 2
_IOC_WRITE = 1

def _ioc(direction: int, type_char: str, number: int, size: int) -> int:
    return (
        (direction << _IOC_DIRSHIFT)
        | (ord(type_char) << _IOC_TYPESHIFT)
        | (number << _IOC_NRSHIFT)
        | (size << _IOC_SIZESHIFT)
    )

def _hidiocsfeature(length: int) -> int:
    return _ioc(_IOC_READ | _IOC_WRITE, "H", 0x06, length)

def _find_ff02() -> Path:
    for hidraw in sorted(Path("/sys/class/hidraw").glob("hidraw*")):
        uevent = hidraw / "device" / "uevent"
        if not uevent.exists():
            continue
        fields = dict(
            line.split("=", 1)
            for line in uevent.read_text().splitlines()
            if "=" in line
        )
        if fields.get("HID_ID") != TARGET_HID_ID:
            continue
        descriptor = (hidraw / "device" / "report_descriptor").read_bytes()
        if descriptor.startswith(bytes.fromhex("0602ff")):
            return Path("/dev") / hidraw.name
    raise FileNotFoundError("ff02 hidraw node not found for 05af:866a")

def _send_feature(node: Path, payload: bytes) -> None:
    buf = array.array("B", b"\x00" + payload)
    with open(node, "rb+", buffering=0) as f:
        fcntl.ioctl(f, _hidiocsfeature(len(buf)), buf, True)

def _build_magkey_frame(emitters: list[list[int]]) -> bytes:
    """emitters: 12 × [r, g, b]"""
    frame = bytearray(64)
    for i, (r, g, b) in enumerate(emitters):
        frame[i * 4 + 2] = r
        frame[i * 4 + 3] = g
        frame[(i + 1) * 4] = b
    return bytes(frame)


# ── Python API (exposed to JS) ───────────────────────────────────────
class Api:
    def __init__(self) -> None:
        self._backend = self._detect_backend()
        self._hid_node: Path | None = None
        self._hid_ready = False
        self._lock = threading.Lock()

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

    # ── Direct HID frame send (animation loop) ────────────────────────
    def send_magkey_frame(self, emitters: list[list[int]]) -> str:
        """Send 64-byte MagKey frame directly. Called at ~25fps from JS animation loop."""
        if self._backend == "mock":
            return "ok"

        try:
            with self._lock:
                if not self._hid_ready:
                    self._hid_node = _find_ff02()
                    for pkt in INIT_PACKETS:
                        _send_feature(self._hid_node, pkt)
                    self._hid_ready = True

                payload = _build_magkey_frame(emitters)
                with open(self._hid_node, "wb", buffering=0) as f:
                    os.write(f.fileno(), payload)
                _send_feature(self._hid_node, COMMIT_PACKET)
            return "ok"
        except Exception as e:
            self._hid_ready = False  # force re-init on next call
            return f"error: {e}"


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
