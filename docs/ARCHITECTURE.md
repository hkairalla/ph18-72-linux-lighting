# Architecture

Hardware access should stay behind a daemon boundary.

```text
QML UI
  -> Python / PySide6 app
  -> Rust daemon
  -> HID and future WMI/ACPI hardware backends
```

## Layers

| Layer | Responsibility |
| --- | --- |
| QML | Future visual controls, color pickers, zone layout, presets, status display. |
| Python / PySide6 | Future app shell, tray/menu integration, user settings, D-Bus client, QML model binding. |
| Rust daemon | Device discovery, permissions, HID writes, future WMI/ACPI probes, profile application, safety checks. |
| Hardware backends | `/dev/hidraw*`, `/sys`, and possibly WMI/ACPI for unresolved lighting surfaces. |

## Boundary Rule

The UI should request semantic operations:

- `SetMagKeys`
- `SetCoverLogo`
- `SetMainKeyboardColor`
- `RestoreKnownGood`

The UI should not know raw packet bytes. Packet construction belongs in the daemon.

## IPC

Use D-Bus for the first production interface once daemon operations are implemented:

- System daemon: `org.ph18.Lighting`
- Object path: `/org/ph18/Lighting`
- Interface: `org.ph18.Lighting1`

The daemon can also expose a CLI for debugging and packaging smoke tests.

## Safety

- Do not run the GUI as root.
- Keep all privileged hardware access in the daemon.
- Validate RGB ranges and surface names in the daemon.
- Keep unknown WMI/ACPI probing read-only until a method is understood.
