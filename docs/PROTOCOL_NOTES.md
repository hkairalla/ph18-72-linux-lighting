# Protocol Notes

Sanitized public record of what the daemon currently knows. Raw packet
captures, exploratory scripts, screenshots, and command logs are kept out of
this repo (`testing/` is git-ignored).

## Known Controllers

| Device | Role |
| --- | --- |
| HID `05af:866a` | Acer/Jing-Mold keyboard-class controller. Drives both the main keyboard (ff02 commit33 + `report82/84/85/86`) and MagKeys (ff02 LED-map). |
| HID `0d62:ba51` | Darfon cover-logo controller. Short-packet color and brightness commands. |
| Acer WMI/ACPI | Present on the machine; still plausible for unresolved Base Logo / Infinity Mirror work. Treat as read-only until methods are understood. |

## Main Keyboard

Two distinct hardware paths on the `05af:866a` controller, with different
roles:

| Path | What it does |
| --- | --- |
| ff02 commit33 (whole-board word writes) | **Flips firmware out of dynamic into a static frame.** Used by `set-main-keyboard-{blue,red,green}` and `set-keyboard-baseline`. The only known mode transition. |
| `report84` + `report86=0x01` (per-index writes) | Modifies individual cells *within an existing static frame*. Inert on a keyboard that is still in dynamic mode. |

### Confirmed ff02 commit33 words

The 4-byte word inside the 64-byte ff02 frame is not a conventional RGB
encoding. Known-working words on this unit:

| Visible color | Word |
| --- | --- |
| Blue | `ff 00 00 ff` |
| Red-ish | `00 00 ff 00` |
| Green | `00 00 00 ff` |
| Off | `00 00 00 00` |

`set-keyboard-baseline --color {off,blue,red,green}` exposes these. Arbitrary
RGB baselines via the ff02 path are not understood.

### `report86` semantics (from research probes)

- `[0x86, 0x00]` standalone → full keyboard blackout. Also clears any pending
  `report84` buffer, so it must not be sent between `report84` and the commit.
- `[0x86, 0x01]` standalone → "return to default dynamic pattern."
- `[0x86, 0x01]` immediately after a `report84` → commits the pending per-key
  change against the current static frame.

The daemon never sends `[0x86, 0x00]` in the per-key write loop; doing so was
the original bug that made per-key writes appear to revert to dynamic.

### Stubborn indices

After an ff02 commit33 sweep, four indices on this unit do not pick up the
baseline color cleanly and need an explicit `report84` follow-up at the
baseline RGB:

- 25 (digit 5)
- 66 (semicolon)
- 71 (keypad 6)
- 98 (arrow down)

The daemon repaints these every time it runs a full repaint, unless the user
has explicitly overridden them.

## MagKey 3.0

The visible MagKey path is `05af:866a` ff02 with an init prelude, a 64-byte
LED map, and a commit packet (`08 02 4f 05 32 08 01 66`).

Confirmed:

- All red / green / blue / off.
- Per-key whole-key presets (`set-magkey-whole-key`).
- Per-zone (`left` / `top` / `right`) RGB within a key (`set-magkey-zones`).

Known caveat:

- A/S/D share slots in ways that can cause color bleed, especially when blue
  is involved. `--safe-magkeys` on the older pattern commands intentionally
  sacrifices some blue behavior to reduce bleed; the verified frame model
  used by the current commands generally avoids it.

## Cover Logo

HID `0d62:ba51`, short feature/output packets.

Confirmed:

- Whole-logo red / green / blue.
- Left / middle / right segments (note: segments visually blend on this
  hardware).
- Brightness 0-100.

The daemon attempts four transport variants per write
(`feature_prefixed`, `feature_raw`, `output_prefixed`, `output_raw`); at least
one succeeds.

## Not Useful So Far

- `report 0x5A` group zones — HID accepts writes, no visible effect on the
  tested surfaces.

## Still Unknown

- Base Logo path.
- Infinity Mirror path.
- A general formula for ff02 commit33 words at arbitrary RGB (only the four
  baseline words above are confirmed).
