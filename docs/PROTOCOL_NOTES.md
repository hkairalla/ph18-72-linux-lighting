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

The 4-byte word inside the 64-byte ff02 frame is a conventional 24-bit RGB
encoding with a routing flag:

```
word = [ 0xff, R, G, B ]
       └── broadcast mode: reaches all 102 main-keyboard indices
```

`set-keyboard-baseline --color X` accepts either a named preset
(`off` / `blue` / `red` / `green`) or any `R,G,B` triple, e.g.
`--color 255,128,0` for orange. The daemon builds the word as
`[0xff, R, G, B]`.

#### Discovery (2026-05-11)

The earlier protocol notes here listed four "known-working" words derived
empirically from packet captures:

| Visible | Word | Status |
| --- | --- | --- |
| Blue | `ff 00 00 ff` | ✓ all 102 keys (coincidentally broadcast — byte 0 = 0xff) |
| "Red-ish" | `00 00 ff 00` | ✗ legacy mode, only ~98 keys respond |
| "Green" | `00 00 00 ff` | ✗ legacy mode, only ~98 keys respond |
| Off | `00 00 00 00` | ✗ legacy mode, only ~98 keys respond |

This is why the previous code had a "stubborn-keys" patch (indices 25,
66, 71, 98) — those four keys retained the previous baseline's color
through any non-broadcast write, producing visible artifacts (e.g.
purple cells when changing blue → "red"). The patch worked around the
symptom, not the cause.

A four-probe `probe-keyboard-word` session against this unit unlocked
the real encoding:

| Probe | Word | Result |
| --- | --- | --- |
| 1 | `ff 00 ff 00` | All 102 keys → green |
| 2 | `ff 00 ff ff` | All 102 keys → cyan (green + blue) |
| 3 | `ff ff 00 00` | All 102 keys → red |

That established: byte 0 = 0xff is a broadcast flag; bytes 1/2/3 are
plain 8-bit R/G/B channels. The patch is now removed and the daemon
supports arbitrary 24-bit RGB baselines.

### Probing new ff02 words

The daemon exposes a `probe-keyboard-word` command that runs an ff02 commit33
sweep with an arbitrary 4-byte word and **does not touch persistent state**,
so you can sweep freely without disturbing the saved baseline / overrides:

```bash
ph18-lighting-daemon probe-keyboard-word --word ff:80:40:20
# decoded=broadcast R=128 G=64 B=32 (all 102 keys)
```

Restore your normal state afterwards with `repaint-keyboard`.

### `report86` semantics (from research probes)

- `[0x86, 0x00]` standalone → full keyboard blackout. Also clears any pending
  `report84` buffer, so it must not be sent between `report84` and the commit.
- `[0x86, 0x01]` standalone → "return to default dynamic pattern."
- `[0x86, 0x01]` immediately after a `report84` → commits the pending per-key
  change against the current static frame.

The daemon never sends `[0x86, 0x00]` in the per-key write loop; doing so was
the original bug that made per-key writes appear to revert to dynamic.

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
