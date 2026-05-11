# Architecture

Hardware access stays behind a daemon boundary.

```text
PyWebView UI (HTML/CSS/JS in app/src/ph18_72_lighting_ui/ui/)
  -> Python shell (pywebview, app/src/ph18_72_lighting_ui/main.py)
  -> Rust daemon CLI (daemon/, invoked per-command via subprocess)
  -> HID and future WMI/ACPI hardware backends
```

## Layers

| Layer | Responsibility |
| --- | --- |
| Web UI | Visual controls, color pickers, zone layout, presets, command output history. |
| Python shell | App entry, daemon process invocation, command result plumbing. |
| Rust daemon | Device discovery, HID writes, persistent keyboard state, packet construction. |
| Hardware backends | `/dev/hidraw*`, `/sys`, and possibly WMI/ACPI for unresolved lighting surfaces. |

## Boundary Rule

The UI requests semantic operations through daemon CLI commands:

- `set-keyboard-baseline --color {off|blue|red|green|R,G,B}`
- `set-keyboard-key --key K --red R --green G --blue B`
- `clear-keyboard-key --key K`
- `reset-keyboard`
- `get-keyboard-state`
- `set-magkey-key`, `set-magkey-whole-key`, `set-magkeys-pattern`, `set-magkey-zones`
- `set-cover-logo --segment {left,middle,right}`
- `set-cover-logo-brightness --level 0..100`
- `inventory`

The UI does not know raw packet bytes. Packet construction belongs in the
daemon.

## Daemon-Side State

The daemon keeps a small persisted state file at
`~/.cache/ph18-lighting/keyboard-state`:

```text
baseline=blue
override=39:255,0,0
override=41:0,255,0
```

Keyboard commands split into two paths:

- **Baseline / reset / repaint** runs a full-board repaint: ff02 commit33
  sweep with the current baseline word, then `report84`+`report86=1` for
  each per-key override.
- **`set-keyboard-key` and `clear-keyboard-key`** are fast-path: a single
  `report84`+`report86=1` for the target key. No ff02 anchor. They assume
  the firmware is already in a static frame (set by an earlier baseline
  command this session). If the firmware drifted back to dynamic, the
  write is silently absorbed; recovery is any baseline / repaint command.

The ff02 anchor is needed because per-key writes are inert against the
firmware's default dynamic animation; only the ff02 commit33 sweep flips
the firmware into a static frame. See
[PROTOCOL_NOTES.md](PROTOCOL_NOTES.md).

## Controller Triage

The daemon inventories available controller paths before mutating commands:

| Path | Current Use |
| --- | --- |
| HID `05af:866a` | Main keyboard (ff02 + `report82/84/86`) and MagKey (ff02 LED-map). |
| HID `0d62:ba51` | Cover Logo. |
| Acer WMI/ACPI | Read-only triage for Base Logo / Infinity Mirror. Not yet wired up. |
| Kernel LED class | Usually brightness/status only, not RGB. |
| Platform profile | Performance/fan mode, not direct RGB color. |

## Build Phases

This stack is built in narrow layers so each new confirmed command can slot
into the daemon without rewriting everything above it.

### Phase 1 — Rust CLI (current)

The Rust CLI wraps confirmed behavior:

- `inventory`
- `set-keyboard-baseline`, `set-keyboard-key`, `clear-keyboard-key`,
  `reset-keyboard`, `get-keyboard-state`, `set-main-keyboard-{blue,red,green}`
- `set-magkeys` family (whole / pattern / per-key / per-zone)
- `set-cover-logo` and `set-cover-logo-brightness`

### Phase 2 — Testing UI (current)

A small PyWebView HTML/CSS/JS UI sits on top of the CLI for hands-on testing.
It is intentionally utilitarian: panels per surface, command/output history,
and a keyboard grid that maps to daemon commands. It is not the final app
shell.

### Phase 3 — Long-running daemon + IPC (future)

Once the command model feels stable, promote the Rust backend into a
long-running daemon and expose D-Bus methods. Suggested names:

- System daemon: `org.ph18.Lighting`
- Object path: `/org/ph18/Lighting`
- Interface: `org.ph18.Lighting1`

The CLI stays around for debugging and packaging smoke tests.

### Phase 4 — Unknown surface research (future)

Only after the basic stack is stable:

- Base Logo investigation
- Infinity Mirror investigation
- WMI/ACPI read-only probes
- New packet families

This keeps risky discovery work from destabilizing the known-good commands.

## Safety

- The UI does not run as root.
- All privileged hardware access stays in the daemon.
- The daemon validates surface names and RGB ranges.
- Unknown WMI/ACPI probing should stay read-only until a method is understood.
