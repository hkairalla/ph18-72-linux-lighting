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
| MagKey 3.0 / WASD overlay | Working | HID `05af:866a` ff02 LED-map path |
| Cover Logo | Working | HID `0d62:ba51` Darfon short-command path |
| Main keyboard whole-color blue | Working | HID `05af:866a` ff02 commit33 path |
| Main keyboard per-key | Not working visibly | `report84` writes are accepted but do not visibly latch |
| Base Logo | Unknown | HID captures inconclusive; WMI/ACPI remains possible |
| Infinity Mirror | Unknown | HID captures inconclusive; WMI/ACPI remains possible |

See [docs/HARDWARE_STATUS.md](docs/HARDWARE_STATUS.md) for the current support table.

## Repo Layout

```text
daemon/     Rust hardware daemon scaffold
docs/       Architecture and hardware support notes
```

The PySide6/QML frontend will be added after the daemon API is real enough to bind to. Keeping the frontend out for now avoids committing fake UI behavior.

## Development

The daemon is currently a minimal scaffold:

```bash
cd daemon
cargo run -- inventory
```

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
