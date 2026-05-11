# Project Context

Linux lighting control for the Acer Predator Helios 18 PH18-72. Public repo;
reverse-engineering scratchwork lives under git-ignored `testing/`.

## Stack at a glance

- **UI**: PyWebView HTML/CSS/JS in [app/src/ph18_72_lighting_ui/ui/](app/src/ph18_72_lighting_ui/ui/).
  Python shell at [app/src/ph18_72_lighting_ui/main.py](app/src/ph18_72_lighting_ui/main.py).
- **Daemon**: Rust CLI in [daemon/src/main.rs](daemon/src/main.rs). One binary,
  one subcommand per operation. UI shells out per command.
- **Hardware**: HID `05af:866a` (keyboard + MagKey) and `0d62:ba51` (Darfon
  cover logo). udev rule in [packaging/99-ph18-72-lighting.rules](packaging/99-ph18-72-lighting.rules)
  gives the user hidraw access without sudo.

## What works

- MagKey RGB (whole / pattern / per-key / per-zone).
- Cover Logo (whole + left/middle/right segments + brightness).
- Main keyboard whole-board color: baseline = off / blue / red / green.
- Per-key keyboard colors (with the state-aware repaint model below).

## Per-key keyboard model

The firmware silently ignores `report84` per-key writes when the keyboard is
in its default dynamic-animation mode. The only known mode-transition out
of dynamic is the ff02 commit33 whole-board sweep used by
`set-main-keyboard-{blue,red,green}` / `set-keyboard-baseline`.

So every keyboard operation does a full-board repaint: ff02 anchor with the
current baseline, then `report84`/`report86=0x01` for each per-key override.
The daemon persists `{baseline, overrides}` in
`~/.cache/ph18-lighting/keyboard-state` between invocations, so per-key
changes stack across separate commands.

See [docs/PROTOCOL_NOTES.md](docs/PROTOCOL_NOTES.md) for the deeper firmware
quirks (`report82/84/86` semantics, ff02 commit33 word table, stubborn
indices).

## Conventions

- Hardware writes belong only in the Rust daemon; the UI sends semantic CLI
  commands. Don't introduce new packet bytes in JS/Python.
- Don't commit anything under `testing/`. It's git-ignored and contains
  imported research scripts and captures.
- When adding a new keyboard CLI command, route it through the
  `repaint_keyboard(state)` helper so accumulated state stays consistent.
- The four "stubborn" indices `[25, 66, 71, 98]` need an explicit `report84`
  follow-up after every ff02 anchor; the daemon handles this — don't
  bypass it.

## Open work

- UI surfaces baseline selection, clear-key, and reset aren't built yet — the
  user-facing controls only know `set-keyboard-key`. New daemon commands
  exist; the UI needs panels.
- The UI's keyboard layout lists `right_ctrl` but the daemon's key map
  doesn't ([daemon/src/main.rs](daemon/src/main.rs) `keyboard_key_index`).
  Clicking it errors. Either add it to the daemon map (if hardware allows)
  or remove from the UI.
- Base Logo and Infinity Mirror surfaces are still unsolved; HID captures
  inconclusive, WMI/ACPI not yet explored.
- ff02 commit33 only has known-working words for `{off, blue, red, green}`.
  No general RGB→word formula yet.

## Hardware quick reference

| HID | Role | Notes |
| --- | --- | --- |
| `05af:866a` ff02 endpoint | Main keyboard mode transitions + MagKey LED map | Selected by report descriptor prefix `06 02 ff` |
| `05af:866a` vendor LED endpoint | `report82/84/85/86` per-key writes | Selected by report descriptor containing `85 82` and `85 83` |
| `0d62:ba51` | Darfon cover logo | Multiple transport variants attempted per write |
