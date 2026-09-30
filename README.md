# PH18-72 Linux Area Lighting Control

Linux lighting control for the **Acer Predator Helios 18 PH18-72**.

This is the clean public product repo. Reverse-engineering captures, scratch
scripts, and packet logs are kept out of Git (`testing/` is git-ignored).

## Which parts do you need?

The repo is layered, so you can take as little or as much as you want:

| You want... | Use | Read |
| --- | --- | --- |
| **Just control the lights on any Linux** | The **Rust daemon** (`daemon/`) plus the **udev rule** (`packaging/70-ph18-72-lighting.rules`). These two are the essential core; everything else calls them. | [docs/PORTABILITY.md](docs/PORTABILITY.md) |
| A GUI, login/resume repaint, and the keep-lights-on timer on another distro or desktop | The optional layers (`app/`, the systemd units, the keep-alive) and the porting checklist. | [docs/PORTABILITY.md](docs/PORTABILITY.md) |
| **The full setup on Omarchy** (Arch + Hyprland + Wayland) | `omarchy/install.sh`: builds everything, installs the udev rule, services, app-menu entry and Hyprland window rule. A worked example you can adapt to your own setup. | [omarchy/README.md](omarchy/README.md) |

```text
5  Desktop glue   Omarchy installer, Hyprland rule, menu entry   (omarchy/)   per desktop
4  GUI            PyWebView app                       (app/)                  optional
3  Services       systemd user units: restore, resume, keep-alive (packaging/) optional
2  Device access  udev rule                           (packaging/)            ESSENTIAL
1  Daemon CLI     Rust, talks HID, keeps saved state  (daemon/)               ESSENTIAL
```

Omarchy status: the components the installer sets up all run on the author's machine. The
installer itself has been dry-run and its file-editing steps tested in an isolated home
directory, but not yet run start to finish on a fresh Omarchy install. An Omarchy **bar
widget / panel plugin** for the lighting controls is planned but not built yet; the daemon
commands it would call already exist.

## Stack

```text
PyWebView UI (HTML/CSS/JS)
  -> Python shell (pywebview)
  -> Rust daemon CLI
  -> HID backends (WMI/ACPI investigated read-only, see below)
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
| Keyboard + WASD light sleep (30 s) | Worked around | MagKey frame write resets the firmware timer; see [Keep the lights on](#keep-the-lights-on-beat-the-30-s-sleep) |
| Base Logo | Unknown | HID inconclusive; WMI/ACPI may help |
| Infinity Mirror | Unknown | HID inconclusive; WMI/ACPI may help |

The firmware's Acer WMI gaming interface (keyboard backlight, misc settings, fans,
battery health) has been mapped read-only; the findings are in
[docs/PROTOCOL_NOTES.md](docs/PROTOCOL_NOTES.md).

See [docs/HARDWARE_STATUS.md](docs/HARDWARE_STATUS.md) for the full table.

### Per-key keyboard model

Per-key `report84` writes are inert against the firmware's default dynamic
animation; only the ff02 commit33 sweep flips the firmware into a static
frame. The daemon keeps a persistent state file
(`~/.cache/ph18-lighting/keyboard-state`) with a 24-bit RGB baseline and a
map of per-key overrides. Baseline / reset / repaint commands do a full
ff02 anchor (~6-20 s). Per-key `set-keyboard-key` / `clear-keyboard-key`
take a fast path (a single `report84`+`report86=0x01`, ~50 ms) and assume
the firmware is already anchored from an earlier baseline this session.
Setting Q red then E green leaves both as expected; clearing Q returns Q
to the baseline.

The daemon also saves the last MagKey (WASD) frame in the same state file and
re-sends it after every anchor, because the anchor wipes the MagKeys. The UI shows
each key in the color the keyboard is displaying; the firmware is write-only, so this
is the daemon's saved state, not a readback.

The ff02 word encoding is `[0xff, R, G, B]` — byte 0 = `0xff` is a
broadcast flag that reaches all 102 keyboard indices. See
[docs/PROTOCOL_NOTES.md](docs/PROTOCOL_NOTES.md) for the discovery.

## Development

On Omarchy you can skip the manual steps below and run [`omarchy/install.sh`](omarchy/README.md).
For another distro see [docs/PORTABILITY.md](docs/PORTABILITY.md).

### Prerequisites

Arch / Omarchy:

```bash
sudo pacman -S --needed rust python python-gobject webkit2gtk-4.1 gtk3
```

Debian / Ubuntu / Pop!_OS:

```bash
sudo apt install python3-pip python3.12-venv python3-gi gir1.2-webkit2-4.1 cargo
```

The UI uses pywebview's GTK backend, so the Python virtualenv must be able to see the
system GTK bindings (`--system-site-packages`). The keep-alive feature additionally needs
a Wayland compositor that supports `ext-idle-notify-v1` (tested on Hyprland; Sway and KDE
Plasma 6 also implement it) and systemd user services.

### Build and run

Build the daemon and run inventory:

```bash
cd daemon
cargo run -- inventory
```

Set up the UI once:

```bash
cd app
python3 -m venv --system-site-packages .venv
.venv/bin/pip install -e .
```

Then launch it (builds the daemon first; set `PH18_UI_BACKEND=mock` to run without
hardware):

```bash
scripts/ph18-lighting
```

`packaging/ph18-lighting.desktop` is a launcher entry for your app menu
(copy it to `~/.local/share/applications/` and replace `%h` with your home directory).
Other ways to start it:

```bash
# Mock mode: no cargo, no hardware required
PH18_UI_BACKEND=mock app/.venv/bin/ph18-72-lighting-ui

# Real mode: requires Rust/cargo and runs daemon commands against the device
PH18_UI_BACKEND=cargo app/.venv/bin/ph18-72-lighting-ui

make dev-ui      # rebuilds daemon then launches UI (recommended dev loop)
make ui-mock
make ui-real
make daemon-inventory
```

### Device access (udev)

The GUI does not run as root. Install the udev rule so your logged-in user can open the
keyboard's hidraw nodes:

```bash
sudo cp packaging/70-ph18-72-lighting.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw --action=change
```

If your session does not pick up the new ACLs immediately, log out and back
in once. The rule uses `uaccess`, so only the logged-in seat user gets access
to the hidraw nodes. (The `70-` prefix matters: `uaccess` has to be tagged before
`73-seat-late.rules` runs, so a `99-` file would get no ACL.)

### Wayland / Hyprland notes

`main.py` prepares the environment itself before GTK loads: it selects the GTK backend,
drops `GDK_SCALE` (an integer scale that makes clicks land in the wrong place under
fractional Wayland scaling) and disables WebKit's DMA-BUF renderer (it crashes on NVIDIA
with a Wayland protocol error). The window's Wayland app id is `ph18-lighting`, so you can
give it a Hyprland rule, e.g. in `~/.config/hypr/hyprland.lua` (this uses Omarchy's
`o.window` helper):

```lua
o.window("ph18-lighting", { float = true })
o.window("ph18-lighting", { center = true })
o.window("ph18-lighting", { size = { 1100, 720 } })
```

## Background services

All three are systemd **user** services. Install the ones you want:

| Service | What it does |
| --- | --- |
| `ph18-lighting-restore` | Repaints the saved keyboard + WASD colors on graphical login (the firmware reverts to its own animation on cold boot). |
| `ph18-lighting-resume` | Repaints them after the laptop wakes from suspend/hibernate. |
| `ph18-lighting-keepalive` | Keeps the keyboard and WASD lights from sleeping after 30 s; see the next section. |

```bash
cargo build --release --manifest-path daemon/Cargo.toml
mkdir -p ~/.config/systemd/user
cp packaging/ph18-lighting-{restore,resume,keepalive}.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now ph18-lighting-restore ph18-lighting-resume ph18-lighting-keepalive
```

The units expect the repo at `~/Projects/ph18-72-linux-lighting`; override `ExecStart=`
with `systemctl --user edit <unit>` if it lives elsewhere. The scripts they run
(`scripts/ph18-lighting-resume`, `scripts/ph18-lighting-keepalive`) use the most recently
built daemon (release, debug or `~/.local/bin`), so rebuild after updating.

### Keep the lights on (beat the 30 s sleep)

The firmware turns the keyboard and WASD lights off 30 s after the last key press
on the **laptop** keyboard (the external keyboard does not count). A MagKey frame
write wakes them and restarts that timer, so the keep-alive service rewrites the
saved MagKey frame (invisible: same colors) to hold them on. Choose the rule in the
GUI (Keyboard page, "Keep lights on") or the CLI:

```bash
ph18-lighting-daemon set-keepalive --mode active                 # on while you use the computer
ph18-lighting-daemon set-keepalive --mode timeout --minutes 10   # on until idle for 10 min
ph18-lighting-daemon set-keepalive --mode always                 # never sleep
ph18-lighting-daemon set-keepalive --mode off                    # firmware default (30 s)
ph18-lighting-daemon set-keepalive --ac-only true                # default: only on AC power
ph18-lighting-daemon get-keepalive
```

`active` uses your Omarchy screensaver delay (`idle.screensaver` in
`~/.config/omarchy/shell.json`, 150 s by default) as its idle threshold. Idle time comes
from the Wayland `ext-idle-notify` protocol (the input-idle variant, so an idle inhibitor
such as video playback does not hold it up), so input from any device counts, and the lights come back the moment
you touch anything. Settings live in `~/.config/ph18-lighting/keepalive.conf`. The
service does nothing while the mode is `off`.

`scripts/ph18-lighting-resume --now` repaints immediately (handy for testing); a repaint
takes 7-20 s.

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

# MagKeys (the last frame is saved and restored after every keyboard sweep)
ph18-lighting-daemon set-magkey-whole-key --key w --color blue
ph18-lighting-daemon set-magkey-zones --key a --left 255,0,0 --top 0,255,0 --right 0,0,255
ph18-lighting-daemon set-magkey-emitters --colors 0,180,255,0,180,255,...   # all 12 zones (36 values), what the UI uses
ph18-lighting-daemon set-animation --mode wheel --speed 1.5   # plays in the background service, survives closing the GUI
ph18-lighting-daemon set-animation --mode none                # stop; saved colors return

# Light sleep timer (see "Keep the lights on")
ph18-lighting-daemon set-keepalive --mode active
ph18-lighting-daemon get-keepalive
ph18-lighting-daemon keepalive-run        # the loop the service runs

# Diagnostics
ph18-lighting-daemon keyboard-key-indices q esc space   # key name -> firmware index
ph18-lighting-daemon idle-probe --idle-after 5 --run-for 30   # watch Wayland idle/active changes

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
