# What you need, and porting to another Linux setup

The repo is layered. The bottom layer is all you need to control the lights on **any**
Linux system; everything above it is optional convenience. The Omarchy / Hyprland / Arch
setup is one worked example of the upper layers (see [`omarchy/`](../omarchy/README.md)).

```text
  ┌──────────────────────────────────────────────────────────────────────┐
  │ 5  Desktop glue      Omarchy installer, Hyprland window rule,        │  per desktop
  │                      app-menu entry                  (omarchy/)      │
  ├──────────────────────────────────────────────────────────────────────┤
  │ 4  GUI               PyWebView app          (app/, scripts/ph18-lighting)│  optional
  ├──────────────────────────────────────────────────────────────────────┤
  │ 3  Background        systemd user units: restore, resume, keep-alive │  optional
  │    services          (packaging/*.service, scripts/)                 │
  ├──────────────────────────────────────────────────────────────────────┤
  │ 2  Device access     udev rule                (packaging/70-*.rules) │  ESSENTIAL
  ├──────────────────────────────────────────────────────────────────────┤
  │ 1  Daemon CLI        Rust, talks HID, keeps the saved state          │  ESSENTIAL
  │                      (daemon/)                                       │
  └──────────────────────────────────────────────────────────────────────┘
```

## Essential: layers 1 and 2

| Piece | Why | What it needs from the OS |
| --- | --- | --- |
| `daemon/` (Rust CLI) | Every lighting operation, the saved color state, the keep-alive loop. The UI and services only ever call this. | A Rust toolchain (`cargo`). It links only against libc, so no Wayland/GTK libraries are needed to build or run it. |
| `packaging/70-ph18-72-lighting.rules` | Lets your user open the keyboard's `/dev/hidraw*` nodes without root. | udev, plus systemd-logind for `uaccess`. See "Device access" below if you don't have logind. |

With just those two:

```bash
cargo build --release --manifest-path daemon/Cargo.toml
sudo cp packaging/70-ph18-72-lighting.rules /etc/udev/rules.d/ && sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=hidraw --action=change
daemon/target/release/ph18-lighting-daemon set-keyboard-baseline --color 255,128,0
```

Device IDs are `05af:866a` (keyboard + MagKeys) and `0d62:ba51` (cover logo). This is written
for the Acer Predator Helios 18 PH18-72; other Predator models with the same controllers are
untested.

## Optional: layers 3 and 4

| Piece | What it does | Portability |
| --- | --- | --- |
| Services: restore, resume, keep-alive | Repaint on login and after suspend; keep the lights from sleeping after 30 s. | Plain systemd **user** units. Not on systemd? Run `ph18-lighting-daemon repaint-keyboard` from your session startup, call it from your suspend/resume hook (elogind, acpid), and run `ph18-lighting-daemon keepalive-run` under any supervisor. |
| Keep-alive idle detection | Lets the `active` and `timeout` modes know you are idle. | Needs a Wayland compositor with `ext-idle-notify-v1` (tested: Hyprland; Sway and KDE Plasma 6 implement it). **Without it** (for example on X11) the daemon logs a warning and treats you as always active, so `active`/`timeout` behave like `always`; `always` and `off` work anywhere. |
| GUI | Key/zone colors, baseline, keep-alive settings. | PyWebView's GTK backend: needs GTK 3, WebKitGTK 4.1 and the Python GObject bindings, and a virtualenv created with `--system-site-packages`. `main.py` only applies its Wayland workarounds when it detects Wayland. |

## Porting checklist

Work through this for a different distro or desktop. Items 1-2 are usually all you need to
change; the rest are opt-in.

1. **Packages.** Names differ per distro (table below).
2. **Device access.** Keep the udev rule; it tags the nodes `uaccess`, which gives the
   logged-in seat user an ACL. The rule file name must sort **before** `73-seat-late.rules`
   (hence `70-`), or no ACL is applied. No logind/seat support? Replace `TAG+="uaccess"` with
   `GROUP="<a group you belong to>", MODE="0660"`. Avoid `MODE="0666"`: it lets every local user read your
   keyboard's raw HID stream.
3. **Repo location.** The packaged units and the `.desktop` file assume
   `~/Projects/ph18-72-linux-lighting`. `omarchy/install.sh` substitutes the real path; if you install by hand,
   do the same with `sed` or use `systemctl --user edit <unit>` to override `ExecStart=`.
4. **Window rules.** The GUI's Wayland app id is `ph18-lighting`. Match on that in your
   compositor if you want it floating and sized (the Hyprland/Omarchy example is in
   [`omarchy/hypr/ph18-lighting.lua`](../omarchy/hypr/ph18-lighting.lua)).
5. **Menu entry.** Copy `packaging/ph18-lighting.desktop` and fix the path.
6. **Theme.** The GUI follows the Omarchy theme if `~/.local/state/omarchy/current/theme/colors.toml` exists; otherwise
   it uses its built-in palette, so nothing needs changing elsewhere.
7. **Idle threshold for `active` mode.** It follows Omarchy's `idle.screensaver`
   (`~/.config/omarchy/shell.json`, default 150 s). On other desktops that file won't exist, so the daemon falls back to 150 s.
   Use `timeout` with `--minutes N` to choose your own value.

### Package names

| | Arch / Omarchy | Debian / Ubuntu | Fedora |
| --- | --- | --- | --- |
| Rust | `rust` | `cargo` | `cargo` |
| Python | `python` | `python3-venv python3-pip` | `python3` |
| GObject bindings | `python-gobject` | `python3-gi` | `python3-gobject` |
| WebKitGTK 4.1 | `webkit2gtk-4.1` | `gir1.2-webkit2-4.1` | `webkit2gtk4.1` |
| GTK 3 | `gtk3` | `gir1.2-gtk-3.0` | `gtk3` |

Only the Arch column has been verified on real hardware; the others are from memory
and unverified, so check them before relying on them.

## What is specific to Omarchy

Everything under [`omarchy/`](../omarchy/README.md), and the `idle.screensaver` lookup above.
Nothing in `daemon/`, `app/` or `packaging/` depends on Omarchy; they only assume Linux, and
(for the keep-alive's `active`/`timeout` modes) a Wayland compositor.
