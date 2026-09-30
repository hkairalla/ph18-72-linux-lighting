# PH18-72 Linux Area Lighting Control

Linux lighting control for the **Acer Predator Helios 18 PH18-72**.

This is the clean public product repo. Reverse-engineering captures, scratch
scripts, and packet logs are kept out of Git (`testing/` is git-ignored).

## Stack

```text
PyWebView UI (HTML/CSS/JS)
  -> Python shell (pywebview)
  -> Rust daemon CLI
  -> HID (and future WMI/ACPI) backends
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the intended structure
and [docs/PROTOCOL_NOTES.md](docs/PROTOCOL_NOTES.md) for the sanitized
protocol summary.

## Hardware Status

| Surface | Status | Backend |
| --- | --- | --- |
| MagKey 3.0 / WASD overlay | Confirmed | HID `05af:866a` ff02 LED-map |
| Cover Logo (whole + segments + brightness) | Confirmed | HID `0d62:ba51` |
| Main keyboard whole-board color | Confirmed for any 24-bit RGB | HID `05af:866a` ff02 commit33 (broadcast mode) |
| Main keyboard per-key colors | Confirmed (anchored to a baseline) | HID `05af:866a` ff02 anchor + `report84` per-key |
| Base Logo | Unknown | HID inconclusive; WMI/ACPI may help |
| Infinity Mirror | Unknown | HID inconclusive; WMI/ACPI may help |

See [docs/HARDWARE_STATUS.md](docs/HARDWARE_STATUS.md) for the full table.

### Per-key keyboard model

Per-key `report84` writes are inert against the firmware's default dynamic
animation; only the ff02 commit33 sweep flips the firmware into a static
frame. The daemon keeps a persistent state file
(`~/.cache/ph18-lighting/keyboard-state`) with a 24-bit RGB baseline and a
map of per-key overrides. Baseline / reset / repaint commands do a full
ff02 anchor (~6 sec). Per-key `set-keyboard-key` / `clear-keyboard-key`
take a fast path (a single `report84`+`report86=0x01`, ~50 ms) and assume
the firmware is already anchored from an earlier baseline this session.
Setting Q red then E green leaves both as expected; clearing Q returns Q
to the baseline.

The ff02 word encoding is `[0xff, R, G, B]` — byte 0 = `0xff` is a
broadcast flag that reaches all 102 keyboard indices. See
[docs/PROTOCOL_NOTES.md](docs/PROTOCOL_NOTES.md) for the discovery.

## Development

Prerequisites:

```bash
sudo apt install python3-pip python3.12-venv cargo
```

Build the daemon and run inventory:

```bash
cd daemon
cargo run -- inventory
```

Install and run the testing UI:

```bash
cd app
python3 -m venv .venv
source .venv/bin/activate
pip install -e .
ph18-72-lighting-ui
```

Backend modes:

```bash
# Mock mode: no cargo, no hardware required
PH18_UI_BACKEND=mock ph18-72-lighting-ui

# Real mode: requires Rust/cargo and runs daemon commands against the device
PH18_UI_BACKEND=cargo ph18-72-lighting-ui
```

For local desktop testing without `sudo`, install the udev rule:

```bash
sudo cp packaging/70-ph18-72-lighting.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw --action=change
```

If your session does not pick up the new ACLs immediately, log out and back
in once. The rule uses `uaccess`, so only the logged-in seat user gets access
to the hidraw nodes.

### Keep the lights on (beat the 30 s sleep)

The firmware turns the keyboard and WASD lights off 30 s after the last key press
on the **laptop** keyboard (the external keyboard does not count). A MagKey frame
write wakes them and restarts that timer, so a background service rewrites the
saved MagKey frame (invisible: same colors) to hold them on. Choose the rule in the
GUI (Keyboard page, "Keep lights on") or the CLI:

```bash
ph18-lighting-daemon set-keepalive --mode active            # on while you use the computer
ph18-lighting-daemon set-keepalive --mode timeout --minutes 10   # on until idle for 10 min
ph18-lighting-daemon set-keepalive --mode always            # never sleep
ph18-lighting-daemon set-keepalive --mode off               # firmware default (30 s)
ph18-lighting-daemon set-keepalive --ac-only true           # default: only on AC power
ph18-lighting-daemon get-keepalive
```

`active` uses your Omarchy screensaver delay (`idle.screensaver` in
`~/.config/omarchy/shell.json`, 150 s by default) as its idle threshold. Idle time comes
from the Wayland `ext-idle-notify` protocol, so input from any device counts, and the
lights come back the moment you touch anything. Settings live in
`~/.config/ph18-lighting/keepalive.conf`. Install the service once:

```bash
cargo build --release --manifest-path daemon/Cargo.toml
cp packaging/ph18-lighting-keepalive.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now ph18-lighting-keepalive.service
```

The service does nothing while the mode is `off`.

### Repaint after suspend/resume

The keyboard controller loses its colors when the machine sleeps and the
firmware drops back to its own animation. A small user service watches
systemd-logind for the wake-up signal and re-sends the saved colors:

```bash
cargo build --release --manifest-path daemon/Cargo.toml   # or the debug build
cp packaging/ph18-lighting-resume.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now ph18-lighting-resume.service
```

It waits a few seconds for the USB device to come back and retries, so the
repaint takes 7-20 s after wake. `scripts/ph18-lighting-resume --now` repaints
immediately (handy for testing). The service expects the repo at
`~/Projects/ph18-72-linux-lighting`; override `ExecStart=` with
`systemctl --user edit ph18-lighting-resume.service` if it lives elsewhere.

### Restore keyboard state on login (optional)

The firmware reverts to its dynamic animation on cold boot. To replay your
last keyboard colors and MagKey (WASD) colors on graphical login, install the
included systemd **user** service:

```bash
mkdir -p ~/.config/systemd/user
cp packaging/ph18-lighting-restore.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now ph18-lighting-restore.service
```

The service runs `scripts/ph18-lighting-resume --now`, which finds the newest
daemon build, retries while the USB device settles, and calls
`repaint-keyboard`. That re-emits the state file at
`~/.cache/ph18-lighting/keyboard-state` (baseline, per-key overrides and the
saved MagKey colors) without modifying it.

Make targets:

```bash
make daemon-inventory
make ui-mock
make ui-real
make dev-ui      # rebuilds daemon then launches UI (recommended dev loop)
```

## Daemon CLI

The daemon is a CLI today. The UI shells out to it per command.

```bash
ph18-lighting-daemon inventory

# Whole-board baseline (also clears per-key overrides)
ph18-lighting-daemon set-keyboard-baseline --color blue
ph18-lighting-daemon set-keyboard-baseline --color 255,128,0   # arbitrary RGB
ph18-lighting-daemon set-main-keyboard-blue   # alias

# Per-key overrides (stacks across calls)
ph18-lighting-daemon set-keyboard-key --key q --red 255 --green 0 --blue 0
ph18-lighting-daemon set-keyboard-key --key e --red 0   --green 255 --blue 0

ph18-lighting-daemon clear-keyboard-key --key q
ph18-lighting-daemon reset-keyboard
ph18-lighting-daemon get-keyboard-state

# MagKeys
ph18-lighting-daemon set-magkey-whole-key --key w --color blue
ph18-lighting-daemon set-magkey-zones --key a --left 255,0,0 --top 0,255,0 --right 0,0,255

# Cover Logo
ph18-lighting-daemon set-cover-logo --red 0 --green 128 --blue 255
ph18-lighting-daemon set-cover-logo-brightness --level 75
```

## Design Notes

- The GUI does not run as root.
- The Rust daemon is the only layer that writes to hardware.
- The UI calls semantic CLI commands; raw HID packet construction lives in
  the daemon.
- Unknown WMI/ACPI paths stay read-only until methods are understood.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for what to include in device-report
issues and the keep-captures-out-of-git rule.

## License

MIT — see [LICENSE](LICENSE).

## Related Work

- Acer PredatorSense: https://www.acer.com/us-en/predator/predatorsense
- Acer Predator WMI/kernel module lineage: https://github.com/JafarAkhondali/acer-predator-turbo-and-rgb-keyboard-linux-module
- Linuwu-Sense: https://github.com/0x7375646F/Linuwu-Sense
