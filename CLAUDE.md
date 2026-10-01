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
path; animations are not saved as colors. The WASD keys on the Keyboard page
are read-only but show the current MagKey colors (a gradient across each key's left/top/right zones,
class `mk-lit`), painted from `updateKeyLabel()` so the two pages always agree.

## MagKey animations (background)

`set-animation --mode wheel|knight|hue|chase|breathe|zone|cascade|none [--speed 0.1-4]` writes
`~/.config/ph18-lighting/animation.conf` (mode, speed, epoch_ms, phase0). The keep-alive service
(`keepalive-run`) plays it at ~25 fps (`daemon/src/animation.rs` is the Rust port of the JS `MODES`;
clock `t = phase0 + (now - epoch_ms)/1000 * speed`), so it survives closing the GUI. It obeys the
keep-alive rules (idle / AC-only) and, when stopped, re-sends the saved static colors. Starting one
while keep-alive is `off` switches it to `active`. The GUI only previews (same clock) and sends
`set-animation`; `get-animation` reads state. Frame bursts take a shared flock on
`$XDG_RUNTIME_DIR/ph18-lighting-hid.lock`; keyboard sweeps take it exclusively. `animation-frame --mode M --t T`
prints the 36 values (used for JS/Rust parity checks).

## Lid closed

The keep-alive service also watches `/proc/acpi/button/lid/*/state`. Config keys in `keepalive.conf`:
`lid_keys_off` (default true: no keep-alive/animation writes with the lid closed, so the keyboard + WASD
sleep within the firmware's 30 s), `lid_logo` (`keep|off|timer`, default timer) and `lid_logo_minutes`
(default 10): the cover logo is set to brightness 0 and restored (last brightness saved in
`~/.cache/ph18-lighting/cover-brightness`) when the lid opens. CLI: `set-keepalive --lid-keys-off --lid-logo
--lid-logo-minutes`. Only matters when the machine stays awake with the lid shut (external monitor);
otherwise logind suspends it. The Infinity Mirror and Base Logo cannot be controlled yet (issue #1/#14
research), so they are not covered. Batch key commands: `set-keyboard-keys --keys a,b --color r,g,b`,
`clear-keyboard-keys --keys a,b`.

## Keyboard layout (GUI)

`app/src/ph18_72_lighting_ui/ui/layout.js` is the single source for the Keyboard page: every key's
daemon `name`, sidebar `label`, printed legend (`main`, shifted `top`, small `sub`, `icon`, `fn`, `corner`)
and its `x, y, w, h` in key units (1 = one standard key; the board is 19 x 6: a 15-wide main block plus a
4-wide numpad). Positions were measured from a photo of the real keyboard. `name` must match the daemon's
`keyboard_key_index`; never rename one without changing the daemon. Icons are an SVG sprite in
`index.html` (`#ic-...`, drawn with `currentColor` so they follow the theme and a lit key's color).
`app.js` (`renderLegend`) builds each keycap and `style.css` positions keys absolutely from `--x --y --w --h`.

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
colors and never change, and status colors (green/red) are fixed. The font follows the fontconfig
`monospace` alias (what `omarchy font set` changes and what Omarchy's shell draws in), resolved once via
`fc-match` and cached until `~/.config/fontconfig/fonts.conf` changes; each key legend is sized to fit its
key (`--fit`), so a wide font never clips labels. With no theme file (other distros,
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
