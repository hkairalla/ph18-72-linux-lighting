# PH18-72 Linux Area Lighting Control

Linux lighting control for the **Acer Predator Helios 18 PH18-72**.

This repo is the clean public product repo. Reverse-engineering captures, scratch scripts, imported repos, and packet logs are intentionally kept out of Git.

Planned architecture:

```text
QML UI
  -> Python / PySide6 frontend
  -> Rust daemon
  -> HID and future WMI/ACPI hardware backends
```

## Current Hardware Status

| Surface | Status | Backend |
| --- | --- | --- |
| MagKey 3.0 / WASD overlay | Under re-verification | HID `05af:866a` ff02 LED-map path |
| Cover Logo | Under re-verification | HID `0d62:ba51` Darfon short-command path |
| Main keyboard whole-color blue | Under re-verification | HID `05af:866a` ff02 commit33 path |
| Main keyboard per-key | Experimental / unstable | `report84` writes are accepted but do not visibly latch reliably |
| Base Logo | Unknown | HID captures inconclusive; WMI/ACPI remains possible |
| Infinity Mirror | Unknown | HID captures inconclusive; WMI/ACPI remains possible |

See [docs/HARDWARE_STATUS.md](docs/HARDWARE_STATUS.md) for the current support table and [docs/PROTOCOL_NOTES.md](docs/PROTOCOL_NOTES.md) for sanitized protocol notes.

## Repo Layout

```text
app/        PySide6/QML testing UI
daemon/     Rust hardware daemon scaffold
docs/       Architecture and hardware support notes
```

The first UI is a testing harness, not the final app shell. It is meant to help us exercise confirmed commands, inspect command/output history, and keep new controller work visible as we go.

## Development

Prerequisites for local development:

```bash
sudo apt install python3-pip python3.12-venv cargo
```

The daemon is currently a minimal scaffold:

```bash
cd daemon
cargo run -- inventory
```

The testing UI can be installed locally with:

```bash
cd app
python3 -m venv .venv
source .venv/bin/activate
pip install -e .
ph18-72-lighting-ui
```

The UI supports two backend modes:

```bash
# Mock mode: no cargo, no hardware required
PH18_UI_BACKEND=mock ph18-72-lighting-ui

# Real mode: requires Rust/cargo and will run daemon commands
PH18_UI_BACKEND=cargo ph18-72-lighting-ui
```

For local desktop testing without `sudo`, install the included `udev` rule:

```bash
sudo cp packaging/99-ph18-72-lighting.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw
```

If your session does not pick up the new ACLs immediately, log out and back in once.

The current rule is intentionally permissive for local development and should be tightened before broader packaging or release.

Mock mode is the easiest way to test the layout, command history, and controller menus while we are still building the daemon.

Shortcut commands:

```bash
make daemon-inventory
make ui-mock
make ui-real
make dev-ui
```

For the day-to-day dev loop, prefer:

```bash
make dev-ui
```

That target rebuilds the Rust daemon first and then launches the UI, which helps avoid testing against a stale backend binary.

## Design Notes

- The GUI should not run as root.
- The Rust daemon should be the only layer that writes to hardware.
- The UI should call semantic commands such as `SetMagKeys`, `SetCoverLogo`, and `RestoreKnownGood`.
- Raw HID packet construction belongs in the daemon.
- Unknown WMI/ACPI paths should stay read-only until we understand them.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the intended application structure.

## Related Work

- Acer PredatorSense: https://www.acer.com/us-en/predator/predatorsense
- Acer Predator WMI/kernel module lineage: https://github.com/JafarAkhondali/acer-predator-turbo-and-rgb-keyboard-linux-module
- Linuwu-Sense: https://github.com/0x7375646F/Linuwu-Sense
