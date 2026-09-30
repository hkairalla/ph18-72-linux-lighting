# Omarchy / Hyprland / Arch setup

This is the integration used on the author's machine: **Omarchy** (Arch Linux) with
**Hyprland** on **Wayland**. It is a thin layer over the generic pieces in the rest of the
repo, so you can also use it as a template for another setup: see
[docs/PORTABILITY.md](../docs/PORTABILITY.md) for which parts are essential and what to change.

## Quick start

```bash
git clone https://github.com/hkairalla/ph18-72-linux-lighting ~/Projects/ph18-72-linux-lighting
cd ~/Projects/ph18-72-linux-lighting
omarchy/install.sh --dry-run     # see exactly what it will do
omarchy/install.sh
```

It is safe to run again, and `omarchy/install.sh --uninstall` removes what it added (the udev rule,
packages and your saved colors/settings are left alone, and it prints how to remove the rule).

| Step | What it does |
| --- | --- |
| 1 deps | `sudo pacman -S --needed` for `rust python python-gobject webkit2gtk-4.1 gtk3` (only the missing ones). |
| 2 build | `cargo build --release` for the daemon. |
| 3 ui | `app/.venv` with `--system-site-packages`, then `pip install -e app`. |
| 4 udev | Installs `packaging/70-ph18-72-lighting.rules` (sudo). Log out and in once if the GUI can't open the keyboard. |
| 5 services | Writes the three systemd user units with your repo path and enables them: **restore** (login), **resume** (wake from sleep), **keepalive** (the light timer). |
| 6 desktop | App-menu entry "PH18-72 Lighting". |
| 7 hypr | Appends [`hypr/ph18-lighting.lua`](hypr/ph18-lighting.lua) to `~/.config/hypr/hyprland.lua` between marker comments (with a backup), so the GUI opens floating at 1100x720. Skipped if you already have a rule. |

Every step except the build has a `--no-...` switch (`--no-deps --no-ui --no-udev --no-services --no-desktop --no-hypr`).

## After installing

- Start the GUI from the menu, or `scripts/ph18-lighting`.
- Keep-alive starts in whatever `~/.config/ph18-lighting/keepalive.conf` says (default `off`, the firmware's 30 s). Pick a mode on the Keyboard page ("Keep lights on"), or:

  ```bash
  daemon/target/release/ph18-lighting-daemon set-keepalive --mode active
  ```

  In `active` mode the lights stay on while you use the computer and sleep when Omarchy's screensaver
  delay (`idle.screensaver` in `~/.config/omarchy/shell.json`) passes.

## What is Omarchy-specific here

- `install.sh` (uses `pacman`).
- `hypr/ph18-lighting.lua` (uses Omarchy's `o.window()` helper in its Lua Hyprland config).
- `active` mode reads Omarchy's `idle.screensaver` delay.
- The GUI mirrors the active Omarchy theme (reads `~/.local/state/omarchy/current/theme/colors.toml`; light and dark themes both supported). Without that file it keeps its own palette.

Everything else is generic Linux. Porting notes are in [docs/PORTABILITY.md](../docs/PORTABILITY.md).

## Status

- Working on the author's machine: everything the installer sets up (daemon, GUI, services, window rule).
- Installer testing: dry run, plus the file-editing, idempotency and uninstall steps in an isolated home directory with `systemctl`/`hyprctl` stubbed. **Not yet run start to finish on a fresh Omarchy install**, so treat the first real run with care (use `--dry-run` first).
- Not built yet: an Omarchy **bar widget / panel plugin** (a QML shell plugin that would put the
  lighting controls in the bar). The pieces it would call (`set-keepalive`, `set-keyboard-baseline`,
  `set-magkey-emitters`, `get-keyboard-state`) already exist as daemon commands.
