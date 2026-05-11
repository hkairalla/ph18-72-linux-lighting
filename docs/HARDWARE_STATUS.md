# Hardware Status

Current public status for Acer Predator Helios 18 PH18-72 lighting support.

| Surface | Status | Backend |
| --- | --- | --- |
| MagKey 3.0 / WASD overlay | Confirmed | HID `05af:866a` ff02 LED-map path |
| Cover Logo (whole + segments) | Confirmed | HID `0d62:ba51` Darfon short-command path |
| Cover Logo brightness | Confirmed | HID `0d62:ba51` Darfon short-command path |
| Main keyboard whole-board color | Confirmed for any 24-bit RGB (broadcast-mode ff02 encoding) | HID `05af:866a` ff02 commit33 path |
| Main keyboard per-key colors | Confirmed (requires a baseline anchor) | HID `05af:866a` ff02 commit33 anchor + `report84` per-key writes |
| `report 0x5A` group zones | Not useful | HID accepts writes but no visible effect on tested surfaces |
| Base Logo | Unknown | HID captures inconclusive; WMI/ACPI remains possible |
| Infinity Mirror | Unknown | HID captures inconclusive; WMI/ACPI remains possible |

## How keyboard colors work

**Whole-board color (baseline)** is set via the ff02 commit33 sweep with a
4-byte word `[0xff, R, G, B]`. Byte 0 = `0xff` puts the controller in
broadcast mode, which reaches all 102 main-keyboard indices uniformly.
Without byte 0 = `0xff`, the write only lands on ~98 indices (the older
"red"/"green" words used by earlier research were missing this flag,
which is what created the "stubborn keys" symptom — those four indices
were just receiving an incomplete write).

**Per-key overrides** are set via `report84` + `report86=0x01` on the
vendor LED HID node. These only land if the keyboard is already in a
static frame — the ff02 anchor is the only known way to flip the
firmware out of its default dynamic animation.

**Persistent state** lives in `~/.cache/ph18-lighting/keyboard-state`:

```
baseline=0,0,255
override=39:255,0,0
```

The daemon writes per-key changes through a fast path (single
`report84`+`report86=0x01`, ~50 ms) and uses the full ff02 sweep only
for baseline / reset / repaint operations. Live-dragging a color slider
in the UI is therefore smooth.

See [PROTOCOL_NOTES.md](PROTOCOL_NOTES.md) for the byte-level encoding
discovery and the probing methodology.

## Known side effect

The ff02 channel is shared with the MagKey LED controller. Whole-board
baseline changes also write into MagKey territory as a side effect,
which can clobber MagKey colors. Per-key writes use the separate
`report84` path and do not touch MagKeys. A future change will persist
and restore MagKey state across ff02 anchors.
