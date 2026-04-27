from __future__ import annotations

import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from PySide6.QtCore import QAbstractListModel, QByteArray, Property, QModelIndex, QObject, Qt, Signal, Slot
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine


APP_DIR = Path(__file__).resolve().parent
REPO_ROOT = APP_DIR.parents[2]
DAEMON_DIR = REPO_ROOT / "daemon"
DAEMON_BIN = DAEMON_DIR / "target" / "debug" / "ph18-lighting-daemon"
DEMO_SCRIPT = REPO_ROOT / "testing" / "python" / "magkey_demo.py"
SAFE_KEYBOARD_PATCH_KEYS = {"5", "semicolon", "keypad_6", "arrow_down"}


@dataclass
class CommandRecord:
    title: str
    command: str
    packets: str
    output: str
    ok: bool


class HistoryModel(QAbstractListModel):
    TitleRole = Qt.UserRole + 1
    CommandRole = Qt.UserRole + 2
    PacketsRole = Qt.UserRole + 3
    OutputRole = Qt.UserRole + 4
    OkRole = Qt.UserRole + 5

    def __init__(self) -> None:
        super().__init__()
        self._items: list[CommandRecord] = []

    def rowCount(self, parent: QModelIndex = QModelIndex()) -> int:
        if parent.isValid():
            return 0
        return len(self._items)

    def data(self, index: QModelIndex, role: int = Qt.DisplayRole):
        if not index.isValid():
            return None
        item = self._items[index.row()]
        if role == self.TitleRole:
            return item.title
        if role == self.CommandRole:
            return item.command
        if role == self.PacketsRole:
            return item.packets
        if role == self.OutputRole:
            return item.output
        if role == self.OkRole:
            return item.ok
        return None

    def roleNames(self) -> dict[int, QByteArray]:
        return {
            self.TitleRole: QByteArray(b"title"),
            self.CommandRole: QByteArray(b"command"),
            self.PacketsRole: QByteArray(b"packets"),
            self.OutputRole: QByteArray(b"output"),
            self.OkRole: QByteArray(b"ok"),
        }

    def prepend(self, record: CommandRecord) -> None:
        self.beginInsertRows(QModelIndex(), 0, 0)
        self._items.insert(0, record)
        self.endInsertRows()


class LightingUiModel(QObject):
    statusChanged = Signal()
    backendReadyChanged = Signal()
    selectedPanelChanged = Signal()
    backendModeChanged = Signal()
    coverLogoStateChanged = Signal()
    magkeyStateChanged = Signal()
    historyLogChanged = Signal()
    animationRunningChanged = Signal()

    def __init__(self) -> None:
        super().__init__()
        self._status = "Ready"
        self._selected_panel = "Main Keyboard"
        self._history = HistoryModel()
        self._history_log = ""
        self._backend_mode = self._detect_backend_mode()
        self._animation_process: subprocess.Popen | None = None
        self._magkey_state: dict[str, tuple[int, int, int]] = {
            "w": (0, 0, 0),
            "a": (0, 0, 0),
            "s": (0, 0, 0),
            "d": (0, 0, 0),
        }
        self._cover_logo_segment = "all"
        self._cover_logo_state: dict[str, tuple[int, int, int]] = {
            "left": (24, 32, 40),
            "middle": (24, 32, 40),
            "right": (24, 32, 40),
        }

    def _detect_backend_mode(self) -> str:
        forced = os.environ.get("PH18_UI_BACKEND", "").strip().lower()
        if forced in {"mock", "cargo"}:
            return forced
        if shutil.which("cargo") is not None and DAEMON_DIR.exists():
            return "cargo"
        return "mock"

    @Property(str, notify=statusChanged)
    def status(self) -> str:
        return self._status

    @Property(bool, notify=backendReadyChanged)
    def backendReady(self) -> bool:
        return self._backend_mode == "cargo"

    @Property(str, notify=backendModeChanged)
    def backendMode(self) -> str:
        return self._backend_mode

    @Property(str, notify=selectedPanelChanged)
    def selectedPanel(self) -> str:
        return self._selected_panel

    @selectedPanel.setter
    def selectedPanel(self, value: str) -> None:
        if self._selected_panel == value:
            return
        self._selected_panel = value
        self.selectedPanelChanged.emit()

    @Property(QObject, constant=True)
    def history(self) -> QObject:
        return self._history

    @Property(str, notify=historyLogChanged)
    def historyLog(self) -> str:
        return self._history_log

    @Property(int, notify=coverLogoStateChanged)
    def coverLogoStateVersion(self) -> int:
        return sum(sum(color) for color in self._cover_logo_state.values())

    @Property(int, notify=magkeyStateChanged)
    def magkeyStateVersion(self) -> int:
        return sum(sum(color) for color in self._magkey_state.values())

    def _mock_output(self, title: str, args: list[str]) -> str:
        command_name = args[0]
        lines = [
            "mode=mock",
            f"title={title}",
            f"command={command_name}",
        ]
        if command_name == "inventory":
            lines.extend(
                [
                    "hid.jingmold=05af:866a",
                    "hid.darfon=0d62:ba51",
                    "wmi=todo-read-only-triage",
                    "surface.main_keyboard=functional",
                    "surface.magkeys=functional",
                    "surface.cover_logo=functional",
                    "surface.base_logo=in-development",
                    "surface.infinity_mirror=in-development",
                ]
            )
        elif command_name == "restore-known-good":
            lines.extend(
                [
                    "action=restore-known-good",
                    "controller=05af:866a",
                    "controller=0d62:ba51",
                    "note=mock run for known-good restore",
                ]
            )
        elif command_name == "set-main-keyboard-blue":
            lines.extend(
                [
                    "action=set-main-keyboard-blue",
                    "controller=05af:866a",
                    "path=ff02_commit33",
                    "word=ff0000ff",
                ]
            )
        elif command_name == "set-main-keyboard-red":
            lines.extend(
                [
                    "action=set-main-keyboard-red",
                    "controller=05af:866a",
                    "path=ff02_commit33",
                    "word=0000ff00",
                    "note=experimental red-ish keyboard test word",
                ]
            )
        elif command_name == "set-main-keyboard-green":
            lines.extend(
                [
                    "action=set-main-keyboard-green",
                    "controller=05af:866a",
                    "path=ff02_commit33",
                    "word=000000ff",
                    "note=experimental green keyboard test word",
                ]
            )
        elif command_name == "set-keyboard-key":
            lines.extend(
                [
                    "action=set-keyboard-key",
                    "controller=05af:866a",
                    "path=report84_report86",
                    f"key={args[args.index('--key') + 1]}",
                    f"rgb={args[args.index('--red') + 1]},{args[args.index('--green') + 1]},{args[args.index('--blue') + 1]}",
                    "note=experimental per-key keyboard path",
                ]
            )
        elif command_name == "set-magkeys":
            lines.extend(
                [
                    "action=set-magkeys",
                    "controller=05af:866a",
                    "path=ff02_ledmap_commit",
                    f"rgb={args[-1]}",
                ]
            )
        elif command_name == "set-magkeys-pattern":
            lines.extend(
                [
                    "action=set-magkeys-pattern",
                    "controller=05af:866a",
                    "path=ff02_ledmap_commit",
                    f"w={args[args.index('--w') + 1]}",
                    f"a={args[args.index('--a') + 1]}",
                    f"s={args[args.index('--s') + 1]}",
                    f"d={args[args.index('--d') + 1]}",
                ]
            )
            if "--safe-magkeys" in args:
                lines.append("mode=safe-magkeys")
        elif command_name == "set-magkey-key":
            lines.extend(
                [
                    "action=set-magkey-key",
                    "controller=05af:866a",
                    "path=ff02_ledmap_commit",
                    f"key={args[args.index('--key') + 1]}",
                    f"rgb={args[args.index('--red') + 1]},{args[args.index('--green') + 1]},{args[args.index('--blue') + 1]}",
                ]
            )
        elif command_name == "set-cover-logo":
            segment = "all"
            if "--segment" in args:
                segment = args[args.index("--segment") + 1]
            lines.extend(
                [
                    "action=set-cover-logo",
                    "controller=0d62:ba51",
                    "path=darfon_short_packets",
                    f"segment={segment}",
                    f"rgb={args[args.index('--red') + 1]},{args[args.index('--green') + 1]},{args[args.index('--blue') + 1]}",
                ]
            )
        elif command_name == "set-cover-logo-brightness":
            lines.extend(
                [
                    "action=set-cover-logo-brightness",
                    "controller=0d62:ba51",
                    "path=darfon_short_packets",
                    f"level={args[args.index('--level') + 1]}",
                ]
            )
        else:
            lines.append("note=no mock output available")
        return "\n".join(lines)

    def _extract_packet_preview(self, output: str) -> str:
        packet_lines: list[str] = []
        for line in output.splitlines():
            lowered = line.lower()
            if (
                "packet=" in lowered
                or lowered.startswith("payload=")
                or lowered.startswith("commit=")
                or lowered.startswith("prelude")
            ):
                packet_lines.append(line)
        return "\n".join(packet_lines)

    def _append_history_log(self, record: CommandRecord) -> None:
        sections = [
            f"[{record.title}] {'OK' if record.ok else 'FAIL'}",
            record.command,
        ]
        if record.packets:
            sections.extend(["Packet Preview:", record.packets])
        sections.extend(["Output:", record.output])
        block = "\n".join(sections)
        self._history_log = f"{block}\n\n{self._history_log}".strip()
        self.historyLogChanged.emit()

    def _run_daemon_command(self, title: str, args: list[str]) -> bool:
        command = [str(DAEMON_BIN), *args] if DAEMON_BIN.exists() else ["cargo", "run", "--quiet", "--", *args]
        pretty = " ".join(command)
        if self._backend_mode == "mock":
            output = self._mock_output(title, args)
            record = CommandRecord(
                title=title,
                command=f"mock {' '.join(args)}",
                packets=self._extract_packet_preview(output),
                output=output,
                ok=True,
            )
            self._history.prepend(record)
            self._append_history_log(record)
            self._status = "Mock backend command recorded"
            self.statusChanged.emit()
            return True

        completed = subprocess.run(
            command,
            cwd=DAEMON_DIR,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        output_parts = []
        if completed.stdout.strip():
            output_parts.append(completed.stdout.strip())
        if completed.stderr.strip():
            output_parts.append(completed.stderr.strip())
        output = "\n".join(output_parts) if output_parts else "(no output)"
        ok = completed.returncode == 0
        record = CommandRecord(
            title=title,
            command=pretty,
            packets=self._extract_packet_preview(output),
            output=output,
            ok=ok,
        )
        self._history.prepend(record)
        self._append_history_log(record)
        self._status = "Last command succeeded" if ok else "Last command failed"
        self.statusChanged.emit()
        return ok

    @Slot()
    def runInventory(self) -> None:
        self._run_daemon_command("Inventory", ["inventory"])

    @Slot()
    def restoreKnownGood(self) -> None:
        self._run_daemon_command("Restore Known Good", ["restore-known-good"])

    @Slot()
    def setMainKeyboardBlue(self) -> None:
        self._run_daemon_command("Main Keyboard Blue", ["set-main-keyboard-blue"])

    @Slot()
    def setMainKeyboardRed(self) -> None:
        self._run_daemon_command("Main Keyboard Red", ["set-main-keyboard-red"])

    @Slot()
    def setMainKeyboardGreen(self) -> None:
        self._run_daemon_command("Main Keyboard Green", ["set-main-keyboard-green"])

    @Slot(str, int, int, int)
    def setKeyboardKeyColor(self, key: str, red: int, green: int, blue: int) -> None:
        if key not in SAFE_KEYBOARD_PATCH_KEYS:
            record = CommandRecord(
                title=f"Keyboard Key {key}",
                command="(blocked)",
                packets="",
                output=(
                    "General per-key keyboard writes are not stable yet on PH18-72. "
                    "Only the stubborn correction keys are enabled for now: 5, semicolon, keypad_6, arrow_down."
                ),
                ok=False,
            )
            self._history.prepend(record)
            self._append_history_log(record)
            self._status = "Blocked unsafe per-key keyboard write"
            self.statusChanged.emit()
            return

        self._run_daemon_command(
            f"Keyboard Key {key}",
            [
                "set-keyboard-key",
                "--key",
                key,
                "--red",
                str(red),
                "--green",
                str(green),
                "--blue",
                str(blue),
            ],
        )

    @Slot()
    def setMagkeysBlue(self) -> None:
        for key in self._magkey_state:
            self._magkey_state[key] = (0, 0, 255)
        self.magkeyStateChanged.emit()
        self._run_daemon_command("MagKeys Blue", ["set-magkeys", "--all", "0,0,255"])

    @Slot()
    def setMagkeysRed(self) -> None:
        for key in self._magkey_state:
            self._magkey_state[key] = (255, 0, 0)
        self.magkeyStateChanged.emit()
        self._run_daemon_command("MagKeys Red", ["set-magkeys", "--all", "255,0,0"])

    @Slot()
    def setMagkeysGreen(self) -> None:
        for key in self._magkey_state:
            self._magkey_state[key] = (0, 255, 0)
        self.magkeyStateChanged.emit()
        self._run_daemon_command("MagKeys Green", ["set-magkeys", "--all", "0,255,0"])

    @Slot()
    def setMagkeysOff(self) -> None:
        for key in self._magkey_state:
            self._magkey_state[key] = (0, 0, 0)
        self.magkeyStateChanged.emit()
        self._run_daemon_command("MagKeys Off", ["set-magkeys", "--all", "0,0,0"])

    @Slot(str, int, int, int)
    def setMagkeyKeyColor(self, key: str, red: int, green: int, blue: int) -> None:
        self._magkey_state[key] = canonicalize_magkey_rgb(red, green, blue)
        self.magkeyStateChanged.emit()
        w = ",".join(str(component) for component in self._magkey_state["w"])
        a = ",".join(str(component) for component in self._magkey_state["a"])
        s = ",".join(str(component) for component in self._magkey_state["s"])
        d = ",".join(str(component) for component in self._magkey_state["d"])
        self._run_daemon_command(
            "MagKeys Pattern",
            [
                "set-magkeys-pattern",
                "--w",
                w,
                "--a",
                a,
                "--s",
                s,
                "--d",
                d,
            ],
        )

    @Slot(str, str)
    def setMagkeyNamedColor(self, key: str, name: str) -> None:
        colors = {
            "off": (0, 0, 0),
            "red": (255, 0, 0),
            "green": (0, 255, 0),
            "blue": (0, 0, 255),
        }
        color = colors.get(name, (0, 0, 0))
        for other_key in self._magkey_state:
            self._magkey_state[other_key] = (0, 0, 0)
        self._magkey_state[key] = color
        self.magkeyStateChanged.emit()
        self._run_daemon_command(
            f"MagKey {key.upper()} {name.title()}",
            [
                "set-magkeys-pattern",
                "--w",
                ",".join(str(component) for component in self._magkey_state["w"]),
                "--a",
                ",".join(str(component) for component in self._magkey_state["a"]),
                "--s",
                ",".join(str(component) for component in self._magkey_state["s"]),
                "--d",
                ",".join(str(component) for component in self._magkey_state["d"]),
            ],
        )

    @Slot(str, result=str)
    def magkeyButtonColor(self, key: str) -> str:
        return rgb_to_hex(self._magkey_state.get(key, (24, 32, 40)))

    @Slot(str, result=str)
    def magkeyButtonTextColor(self, key: str) -> str:
        color = self._magkey_state.get(key, (24, 32, 40))
        brightness = (color[0] * 299 + color[1] * 587 + color[2] * 114) / 1000
        return "#10151a" if brightness > 140 else "#eef2f7"

    @Slot()
    def setCoverLogoBlue(self) -> None:
        self.setCoverLogoColor("all", 0, 0, 255)

    @Slot(str, int, int, int)
    def setCoverLogoColor(self, segment: str, red: int, green: int, blue: int) -> None:
        args = [
            "set-cover-logo",
            "--red",
            str(red),
            "--green",
            str(green),
            "--blue",
            str(blue),
        ]
        if segment != "all":
            args.extend(["--segment", segment])
        ok = self._run_daemon_command(f"Cover Logo {segment}", args)
        if not ok:
            return
        color = (
            max(0, min(255, red)),
            max(0, min(255, green)),
            max(0, min(255, blue)),
        )
        if segment == "all":
            for zone in self._cover_logo_state:
                self._cover_logo_state[zone] = color
        else:
            self._cover_logo_state[segment] = color
        self.coverLogoStateChanged.emit()

    @Slot(int)
    def setCoverLogoBrightness(self, level: int) -> None:
        self._run_daemon_command(
            "Cover Logo Brightness",
            ["set-cover-logo-brightness", "--level", str(max(0, min(100, level)))],
        )

    @Slot(str, result=str)
    def coverLogoButtonColor(self, segment: str) -> str:
        if segment == "all":
            colors = list(self._cover_logo_state.values())
            if all(color == colors[0] for color in colors[1:]):
                return rgb_to_hex(colors[0])
            return "#2f3943"
        return rgb_to_hex(self._cover_logo_state.get(segment, (24, 32, 40)))

    @Slot(str, result=str)
    def coverLogoButtonTextColor(self, segment: str) -> str:
        if segment == "all":
            colors = list(self._cover_logo_state.values())
            if not all(color == colors[0] for color in colors[1:]):
                return "#eef2f7"
            color = colors[0]
        else:
            color = self._cover_logo_state.get(segment, (24, 32, 40))
        brightness = (color[0] * 299 + color[1] * 587 + color[2] * 114) / 1000
        return "#10151a" if brightness > 140 else "#eef2f7"

    @Property(bool, notify=animationRunningChanged)
    def animationRunning(self) -> bool:
        return self._animation_process is not None and self._animation_process.poll() is None

    @Slot(str)
    def startMagkeyAnimation(self, mode: str) -> None:
        self.stopMagkeyAnimation()
        if not DEMO_SCRIPT.exists():
            self._status = f"Demo script not found: {DEMO_SCRIPT}"
            self.statusChanged.emit()
            return
        self._animation_process = subprocess.Popen(
            [sys.executable, str(DEMO_SCRIPT), "--mode", mode],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        self._status = f"Animation running: {mode}"
        self.statusChanged.emit()
        self.animationRunningChanged.emit()

    @Slot()
    def stopMagkeyAnimation(self) -> None:
        if self._animation_process is not None:
            self._animation_process.terminate()
            try:
                self._animation_process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self._animation_process.kill()
            self._animation_process = None
        self._status = "Animation stopped"
        self.statusChanged.emit()
        self.animationRunningChanged.emit()

    @Slot(str, int, int, int, int, int, int, int, int, int)
    def setMagkeyZones(
        self,
        key: str,
        left_r: int, left_g: int, left_b: int,
        top_r: int, top_g: int, top_b: int,
        right_r: int, right_g: int, right_b: int,
    ) -> None:
        self._run_daemon_command(
            f"MagKey {key.upper()} Zones",
            [
                "set-magkey-zones",
                "--key", key,
                "--left", f"{left_r},{left_g},{left_b}",
                "--top", f"{top_r},{top_g},{top_b}",
                "--right", f"{right_r},{right_g},{right_b}",
            ],
        )

    @Slot()
    def noteUnimplemented(self) -> None:
        record = CommandRecord(
            title="Unimplemented Surface",
            command="(no command)",
            packets="",
            output="This surface is still in development. No controller command is sent yet.",
            ok=False,
        )
        self._history.prepend(record)
        self._append_history_log(record)
        self._status = "In-development surface selected"
        self.statusChanged.emit()


def canonicalize_magkey_rgb(red: int, green: int, blue: int) -> tuple[int, int, int]:
    if red <= 0 and green <= 0 and blue <= 0:
        return (0, 0, 0)
    if red >= green and red >= blue:
        return (255, 0, 0)
    if blue >= green:
        return (0, 0, 255)
    return (0, 255, 0)


def rgb_to_hex(color: tuple[int, int, int]) -> str:
    return "#{:02x}{:02x}{:02x}".format(*color)


def main() -> int:
    app = QGuiApplication(sys.argv)
    engine = QQmlApplicationEngine()
    model = LightingUiModel()
    engine.rootContext().setContextProperty("lightingUiModel", model)
    qml_path = APP_DIR / "qml" / "Main.qml"
    engine.load(qml_path)
    if not engine.rootObjects():
        return 1
    return app.exec()


if __name__ == "__main__":
    raise SystemExit(main())
