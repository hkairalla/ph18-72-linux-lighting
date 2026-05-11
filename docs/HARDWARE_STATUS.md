# Hardware Status

Current public status for Acer Predator Helios 18 PH18-72 lighting support.

| Surface | Status | Backend |
| --- | --- | --- |
| MagKey 3.0 / WASD overlay | Confirmed | HID `05af:866a` ff02 LED-map path |
| Cover Logo (whole + segments) | Confirmed | HID `0d62:ba51` Darfon short-command path |
| Cover Logo brightness | Confirmed | HID `0d62:ba51` Darfon short-command path |
| Main keyboard whole-board color | Confirmed for `off` / `blue` / `red` / `green` baselines | HID `05af:866a` ff02 commit33 path |
| Main keyboard per-key colors | Confirmed (requires a baseline anchor) | HID `05af:866a` ff02 commit33 anchor + `report84` per-key writes |
| `report 0x5A` group zones | Not useful | HID accepts writes but no visible effect on tested surfaces |
| Base Logo | Unknown | HID captures inconclusive; WMI/ACPI remains possible |
| Infinity Mirror | Unknown | HID captures inconclusive; WMI/ACPI remains possible |

## Per-key colors: how it actually works

The per-key `report84`/`report86=1` path only lands when the keyboard is
already in a static frame. The firmware's default dynamic-pattern mode silently
absorbs per-key writes. The ff02 commit33 sweep (the one used by
`set-main-keyboard-blue`) is the only known transition out of dynamic mode.

The daemon now keeps a small persistent state file
(`~/.cache/ph18-lighting/keyboard-state`) with a baseline color and a map of
per-key overrides. Every keyboard command repaints the full board: ff02 anchor
with the current baseline, then per-key `report84`/`report86=1` for each
override. This is what makes accumulated per-key changes survive.

See [PROTOCOL_NOTES.md](PROTOCOL_NOTES.md) for the sanitized protocol summary.
