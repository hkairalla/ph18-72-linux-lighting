# Project Context

Linux lighting control for the Acer Predator Helios 18 PH18-72. Public repo;
reverse-engineering scratchwork lives under git-ignored `testing/`.

## Stack at a glance

- **UI**: PyWebView HTML/CSS/JS in [app/src/ph18_72_lighting_ui/ui/](app/src/ph18_72_lighting_ui/ui/).
  Python shell at [app/src/ph18_72_lighting_ui/main.py](app/src/ph18_72_lighting_ui/main.py).
- **Daemon**: Rust CLI in [daemon/src/main.rs](daemon/src/main.rs). One binary,
  one subcommand per operation. UI shells out per command.
- **Hardware**: HID `05af:866a` (keyboard + MagKey) and `0d62:ba51` (Darfon
  cover logo). udev rule in [packaging/70-ph18-72-lighting.rules](packaging/70-ph18-72-lighting.rules)
  gives the user hidraw access without sudo.

## What works

- MagKey RGB (whole / pattern / per-key / per-zone).
- Cover Logo (whole + left/middle/right segments + brightness).
- Main keyboard whole-board color: any 24-bit RGB baseline (named presets `off`/`blue`/`red`/`green` are aliases).
- Per-key keyboard colors (with the state-aware repaint model below).

## MagKey persistence

The daemon saves the last MagKey frame as `magkey=<zone>:r,g,b` lines in the
same state file and re-sends it after every ff02 sweep (which would otherwise
wipe WASD). `set-magkey-emitters --colors <36 values>` is the UI's static-apply
path; animation frames use the direct Python path and are not saved.

## Keep-alive (light sleep timer)

The firmware sleeps the keyboard + WASD lights 30 s after the last key press on the
laptop keyboard. A MagKey frame write wakes them and resets that timer; a per-key
(`report84`) write does neither. `daemon/src/keepalive.rs` uses this: the
`keepalive-run` loop (user service `ph18-lighting-keepalive`) re-sends the saved
MagKey frame every 20 s according to `~/.config/ph18-lighting/keepalive.conf`
(`off | active | always | timeout`, `minutes`, `ac_only`; CLI `set-keepalive` /
`get-keepalive`). Idle time comes from `daemon/src/idle.rs` (Wayland ext-idle-notify v2,
input idle, ignores inhibitors). Every MagKey write touches
`~/.cache/ph18-lighting/last-magkey-write`; the loop skips its own write if one
happened in the last 15 s (the UI's live animation streams frames directly).
The decision rule is a pure function (`should_write`) with unit tests.

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
quirks (`report82/84/86` semantics, ff02 commit33 word table).

## Conventions

- Hardware writes belong only in the Rust daemon; the UI sends semantic CLI
  commands. Don't introduce new packet bytes in JS/Python.
- Don't commit anything under `testing/`. It's git-ignored and contains
  imported research scripts and captures.
- When adding a new keyboard CLI command, decide first whether it needs a
  full ff02 anchor or whether the fast path (single `report84`+`report86=1`
  per key) is enough. Mutating a single key is fast-path; baseline /
  reset / repaint are full anchor.

## Theming (Omarchy)

The GUI mirrors the active Omarchy theme. `Api.get_theme()` (main.py) reads
`~/.local/state/omarchy/current/theme/colors.toml` (+ `theme.name`); `applyTheme()` (app.js) maps it
onto the CSS variables by role (page / sidebar / card / hover; light themes are ordered differently)
and re-reads every 3 s and on focus. Only the chrome is themed: lit-key colors are the real keyboard
colors and never change, and status colors (green/red) are fixed. With no theme file (other distros,
mock mode) the built-in cyan palette is used. All accent tints are derived from `--accent-rgb`; do not
reintroduce literal colors in `style.css`.

## Open work

Tracked as GitHub issues; the roadmap with the suggested order is issue #14
(https://github.com/hkairalla/ph18-72-linux-lighting/issues/14). In short: hardware QA of the remaining
controls (keep-alive, MagKey page, Cover Logo), Omarchy theme sync for the lights, the bar widget
plugin, matching the GUI keyboard layout to the real one, and research on Base Logo / Infinity Mirror
via the Acer WMI interface (findings so far in docs/PROTOCOL_NOTES.md).

## Hardware quick reference

| HID | Role | Notes |
| --- | --- | --- |
| `05af:866a` ff02 endpoint | Main keyboard mode transitions + MagKey LED map | Selected by report descriptor prefix `06 02 ff` |
| `05af:866a` vendor LED endpoint | `report82/84/85/86` per-key writes | Selected by report descriptor containing `85 82` and `85 83` |
| `0d62:ba51` | Darfon cover logo | Multiple transport variants attempted per write |
