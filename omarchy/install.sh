#!/usr/bin/env bash
# Install the PH18-72 lighting stack on Omarchy (Arch Linux + Hyprland + Wayland).
#
#   omarchy/install.sh                 do everything below
#   omarchy/install.sh --dry-run       print what would happen, change nothing
#   omarchy/install.sh --uninstall     remove the services, menu entry and Hyprland rule
#   --no-deps --no-ui --no-udev --no-wmi --no-services --no-desktop --no-hypr    skip a step
#
# Steps:
#   1 deps      system packages (pacman)
#   2 build     Rust daemon, release build
#   3 ui        Python virtualenv + the GUI
#   4 udev      hidraw access for your user         (needs sudo)
#   4b wmi      root helper + sudoers rule for the Infinity Mirror (BIOS/WMI path)  (needs sudo)
#   5 services  systemd user units: restore (login), resume (wake), keepalive (light timer)
#   6 desktop   app-menu entry
#   7 hypr      Hyprland window rule for the GUI
#
# Everything is idempotent: running it again just brings things up to date. The repo
# location is filled into the unit files, so it can live anywhere.
# Porting to another distro/compositor: see docs/PORTABILITY.md - steps 1, 4 (usually
# unchanged), 6 and 7 are the parts that differ.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.." && pwd)"
DEFAULT_REPO_IN_UNITS='%h/Projects/ph18-72-linux-lighting'   # what the packaged units assume

DRY=0 UNINSTALL=0
DO_DEPS=1 DO_UI=1 DO_UDEV=1 DO_WMI=1 DO_SERVICES=1 DO_DESKTOP=1 DO_HYPR=1
for arg in "$@"; do
  case "$arg" in
    --dry-run)     DRY=1 ;;
    --uninstall)   UNINSTALL=1 ;;
    --no-deps)     DO_DEPS=0 ;;
    --no-ui)       DO_UI=0 ;;
    --no-udev)     DO_UDEV=0 ;;
    --no-wmi)      DO_WMI=0 ;;
    --no-services) DO_SERVICES=0 ;;
    --no-desktop)  DO_DESKTOP=0 ;;
    --no-hypr)     DO_HYPR=0 ;;
    -h|--help)     sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown option: $arg (try --help)" >&2; exit 2 ;;
  esac
done

SERVICES=(restore resume keepalive)
UNIT_DIR="$HOME/.config/systemd/user"
DESKTOP_DIR="$HOME/.local/share/applications"
HYPR_FILE="$HOME/.config/hypr/hyprland.lua"
WMI_HELPER_DST=/usr/local/libexec/ph18-lighting-wmi
SUDOERS_DST=/etc/sudoers.d/ph18-lighting
MARK_BEGIN='-- >>> ph18-lighting (managed by omarchy/install.sh) >>>'
MARK_END='-- <<< ph18-lighting <<<'

say()  { printf '\n\033[1;36m==> %s\033[0m\n' "$*"; }
note() { printf '    %s\n' "$*"; }
run()  { if (( DRY )); then printf '    + %s\n' "$*"; else "$@"; fi; }

# Write $2 (a file) to $1 with the repo path filled in.
install_with_repo_path() {
  local dest="$1" src="$2"
  if (( DRY )); then note "+ write $dest (from ${src#"$REPO_ROOT"/})"; return; fi
  mkdir -p "$(dirname "$dest")"
  sed "s|$DEFAULT_REPO_IN_UNITS|$REPO_ROOT|g" "$src" > "$dest"
}

hypr_has_unmanaged_rule() {
  [[ -f "$HYPR_FILE" ]] && grep -q 'o.window("ph18-lighting"' "$HYPR_FILE" && ! grep -qF -- "$MARK_BEGIN" "$HYPR_FILE"
}

if (( UNINSTALL )); then
  say "Removing services, menu entry and Hyprland rule"
  for s in "${SERVICES[@]}"; do
    run systemctl --user disable --now "ph18-lighting-$s.service" 2>/dev/null || true
    run rm -f "$UNIT_DIR/ph18-lighting-$s.service"
  done
  run systemctl --user daemon-reload
  run rm -f "$DESKTOP_DIR/ph18-lighting.desktop"
  if [[ -f "$HYPR_FILE" ]] && grep -qF -- "$MARK_BEGIN" "$HYPR_FILE"; then
    if (( DRY )); then note "+ remove the managed block from $HYPR_FILE"
    else
      awk -v b="$MARK_BEGIN" -v e="$MARK_END" '$0==b{skip=1} !skip{print} $0==e{skip=0}' "$HYPR_FILE" > "$HYPR_FILE.tmp" && mv "$HYPR_FILE.tmp" "$HYPR_FILE"
    fi
  fi
  note "Left in place: the udev rule, system packages, ~/.config/ph18-lighting, ~/.cache/ph18-lighting."
  note "Remove the udev rule with: sudo rm /etc/udev/rules.d/70-ph18-72-lighting.rules"
  note "Remove the Infinity Mirror root helper with: sudo rm $WMI_HELPER_DST $SUDOERS_DST"
  exit 0
fi

command -v pacman >/dev/null || { echo "This installer targets Arch/Omarchy (pacman not found). See docs/PORTABILITY.md for other distros." >&2; exit 1; }

# 1 ---------------------------------------------------------------------------------
if (( DO_DEPS )); then
  say "1/7 System packages"
  # acpi_call-dkms (AUR) gives /proc/acpi/call, which the Infinity Mirror needs; it is optional.
  PKGS=(rust python python-gobject webkit2gtk-4.1 gtk3)
  missing=$(pacman -T "${PKGS[@]}" || true)
  if [[ -z "$missing" ]]; then note "all present: ${PKGS[*]}"
  else note "missing: $(echo $missing)"; run sudo pacman -S --needed $missing; fi
fi

# 2 ---------------------------------------------------------------------------------
say "2/7 Build the daemon (release)"
run cargo build --release --manifest-path "$REPO_ROOT/daemon/Cargo.toml"

# 3 ---------------------------------------------------------------------------------
if (( DO_UI )); then
  say "3/7 GUI (Python virtualenv with system GTK bindings)"
  if [[ ! -x "$REPO_ROOT/app/.venv/bin/python" ]]; then run python3 -m venv --system-site-packages "$REPO_ROOT/app/.venv"; fi
  run "$REPO_ROOT/app/.venv/bin/pip" install --quiet -e "$REPO_ROOT/app"
fi

# 4 ---------------------------------------------------------------------------------
if (( DO_UDEV )); then
  say "4/7 Device access (udev rule)"
  RULE_SRC="$REPO_ROOT/packaging/70-ph18-72-lighting.rules"
  RULE_DST=/etc/udev/rules.d/70-ph18-72-lighting.rules
  [[ -e /etc/udev/rules.d/99-ph18-72-lighting.rules ]] && note "note: an old 99- rule exists and can be deleted (uaccess needs the 70- prefix)"
  if cmp -s "$RULE_SRC" "$RULE_DST" 2>/dev/null; then note "already installed and up to date"
  else
    run sudo install -Dm644 "$RULE_SRC" "$RULE_DST"
    run sudo udevadm control --reload-rules
    run sudo udevadm trigger --subsystem-match=hidraw --action=change
  fi
fi

# 4b --------------------------------------------------------------------------------
if (( DO_WMI )); then
  say "4b/7 Infinity Mirror root helper (Acer WMI through acpi_call)"
  HELPER_SRC="$REPO_ROOT/daemon/target/release/ph18-lighting-wmi"
  if ! lsmod | grep -q '^acpi_call' && ! modinfo acpi_call >/dev/null 2>&1; then
    note "acpi_call is not installed: the Infinity Mirror will stay unavailable until it is."
    note "  yay -S acpi_call-dkms   (then run this installer again)"
  fi
  # The helper only accepts one 16-byte lighting buffer (see daemon/src/bin/ph18-lighting-wmi.rs), so
  # letting your user run it as root without a password exposes nothing else. It must live in a
  # root-owned place: sudoers trusting a file in your home would be root for anything that can edit it.
  if cmp -s "$HELPER_SRC" "$WMI_HELPER_DST" 2>/dev/null; then note "helper already installed and up to date"
  else run sudo install -Dm755 -o root -g root "$HELPER_SRC" "$WMI_HELPER_DST"; fi
  SUDOERS_LINE="$USER ALL=(root) NOPASSWD: $WMI_HELPER_DST"
  if [[ -f "$SUDOERS_DST" ]] && sudo grep -qxF -- "$SUDOERS_LINE" "$SUDOERS_DST" 2>/dev/null; then note "sudoers rule already present"
  elif (( DRY )); then note "+ write $SUDOERS_DST: $SUDOERS_LINE"
  else
    printf '%s\n' "$SUDOERS_LINE" | sudo tee "$SUDOERS_DST" >/dev/null
    sudo chmod 0440 "$SUDOERS_DST"
    if ! sudo visudo -cf "$SUDOERS_DST" >/dev/null; then sudo rm -f "$SUDOERS_DST"; echo "sudoers rule failed validation and was removed" >&2; exit 1; fi
    note "sudoers rule written"
  fi
  if command -v modprobe >/dev/null && modinfo acpi_call >/dev/null 2>&1; then
    MODLOAD=/etc/modules-load.d/ph18-lighting.conf
    [[ -f "$MODLOAD" ]] || { run sudo sh -c "echo acpi_call > $MODLOAD"; run sudo modprobe acpi_call; }
  fi
fi

# 5 ---------------------------------------------------------------------------------
if (( DO_SERVICES )); then
  say "5/7 Background services"
  for s in "${SERVICES[@]}"; do install_with_repo_path "$UNIT_DIR/ph18-lighting-$s.service" "$REPO_ROOT/packaging/ph18-lighting-$s.service"; done
  run systemctl --user daemon-reload
  run systemctl --user enable --now "${SERVICES[@]/#/ph18-lighting-}"
  note "keep-alive mode comes from ~/.config/ph18-lighting/keepalive.conf (default: off = the firmware's 30 s). Choose one in the GUI or:"
  note "  $REPO_ROOT/daemon/target/release/ph18-lighting-daemon set-keepalive --mode active"
fi

# 6 ---------------------------------------------------------------------------------
if (( DO_DESKTOP )); then
  say "6/7 App-menu entry"
  install_with_repo_path "$DESKTOP_DIR/ph18-lighting.desktop" "$REPO_ROOT/packaging/ph18-lighting.desktop"
fi

# 7 ---------------------------------------------------------------------------------
if (( DO_HYPR )); then
  say "7/7 Hyprland window rule"
  if [[ ! -f "$HYPR_FILE" ]]; then note "no $HYPR_FILE: skipping (not an Omarchy Lua config?)"
  elif hypr_has_unmanaged_rule; then note "a ph18-lighting rule is already in $HYPR_FILE: leaving it alone"
  elif grep -qF -- "$MARK_BEGIN" "$HYPR_FILE"; then note "managed block already present"
  else
    if (( DRY )); then note "+ append the rules from omarchy/hypr/ph18-lighting.lua to $HYPR_FILE"
    else
      cp "$HYPR_FILE" "$HYPR_FILE.bak.$(date +%s)"
      { echo; echo "$MARK_BEGIN"; cat "$REPO_ROOT/omarchy/hypr/ph18-lighting.lua"; echo "$MARK_END"; } >> "$HYPR_FILE"
      command -v hyprctl >/dev/null && hyprctl reload >/dev/null 2>&1 || true
      note "added (backup saved next to it); Hyprland reloaded"
    fi
  fi
fi

say "Done"
note "Start the GUI from your app menu (\"PH18-72 Lighting\") or: $REPO_ROOT/scripts/ph18-lighting"
note "If the GUI cannot open the keyboard, log out and back in once so the udev ACL applies."
