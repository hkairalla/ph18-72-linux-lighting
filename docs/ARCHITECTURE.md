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

## Controller Triage First

The daemon should inventory available controller paths before running mutating lighting commands.

| Path | Current Use |
| --- | --- |
| HID `05af:866a` | Main keyboard and MagKey testing first. |
| HID `0d62:ba51` | Cover Logo testing first. |
| Acer WMI/ACPI | Read-only triage for Base Logo / Infinity Mirror. |
| Kernel LED class | Usually brightness/status only, not RGB. |
| Platform profile | Performance/fan mode, not direct RGB color. |

The frontend should use this inventory result to mark unsupported or unknown surfaces instead of blindly issuing commands.

## Build Phases

Build this in narrow layers so each new confirmed command can slot into the daemon without rewriting the stack.

### Phase 1: Rust CLI First

Start with a Rust CLI that wraps only confirmed behavior:

- `inventory`
- `set-magkeys`
- `set-cover-logo`
- `set-main-keyboard-blue`
- `restore-known-good`

Why:

- Fastest path to a stable hardware boundary.
- Easy to add new commands as we discover them.
- Lets us keep all packet construction in one place.
- Gives us something testable before we add IPC or UI.

### Phase 2: Simple Testing UI

Build a very small PySide6/QML testing UI after the CLI can drive known surfaces.

The testing UI should be intentionally utilitarian:

- buttons for known-good commands
- color controls for MagKeys and Cover Logo
- a status panel that shows detected controllers
- a manual command surface only for known-safe operations

This UI should call the Rust layer through a stable interface, not reimplement packet logic.

### Phase 3: Daemon + IPC

Once the command model feels stable, promote the Rust backend into a long-running daemon and expose D-Bus methods.

At that point the UI becomes a real app instead of a test harness.

### Phase 4: Unknown Surface Research

Only after the basic stack is stable should we add:

- Base Logo investigation
- Infinity Mirror investigation
- WMI/ACPI read-only probes
- new packet families

That keeps risky discovery work from destabilizing the known-good commands.

## Recommended Approach

The best path is:

1. Build the real Rust command layer now.
2. Add a small testing UI on top of that real command layer.
3. Grow the daemon/API from the same code, instead of building a throwaway test stack.

That gives us one hardware implementation and one gradual path upward. Each newly discovered command becomes:

- a Rust packet builder/backend method
- a CLI command
- optionally a test UI control
- later a daemon/API method

This is the cleanest way to keep momentum without duplicating logic.

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
