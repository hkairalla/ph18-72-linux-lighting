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

### Cover logo: firmware animation and extra findings (2026-10)

- Only segment ids 1-3 exist (ids 0 and 4-16 do nothing). They blend visually; the logo behind them has a
  firmware rainbow (top to bottom) that is active after boot until a brightness or color packet freezes it.
- Packet checksum: `tail = 0xff - (sum of the first 7 bytes) & 0xff`. The device does not seem to check it
  (the daemon's color/brightness tails are not exact and still work).
- Dynamic modes, from a Windows PredatorSense capture (`coverlogotest.pcapng`, frames 121-145), not yet in the
  daemon: mode 3 = `14 03 00 00 00 00 02 e6` then `08 00 03 05 64 08 04 7f`; mode 5 =
  `14 05 00 00 ae c7 02 6f` then `08 00 05 05 64 00 02 87`. Static = `14 01 <seg> BB GG RR 03 ..`
  (BGR order) and `08 01 01 05 <level> 01 00 ..` for brightness.
- Sending `08 01 01 <op> ...` for ops 1-10 only behaved as brightness; no other effect opcodes were found.

## Infinity Mirror and Base Logo: Acer WMI (2026-10)

Neither is USB (the Windows captures contain no writes for them). Both are driven through the Acer
gaming WMI interface, ACPI method `\_SB.PC00.WMID.WMBH(instance 0, method, input)` (class
`AcerGamingFunction`), reachable on Linux only through the `acpi_call` module's root-only
`/proc/acpi/call`. The daemon uses a tiny root helper (`ph18-lighting-wmi`, sudoers NOPASSWD) for it.

### Infinity Mirror (rear light bar, the HELIOS strip): method 0x14 `SetGamingKBBacklight`

16-byte buffer (layout from the community Venator driver for the PH16-71, confirmed here):

`[mode, speed, brightness, 0x00, direction, R, G, B, 0x03, zone, 0 x6]`

| Field | Values |
| --- | --- |
| mode | `0xff` static; `0x01` cycles colors with a blink between them; `0x02` fades between colors; `0x03` several colors changing along the bar; `0x04` seven static colors, one per zone (the Venator names breathing / neon / rainbow / wave fit); `0x05`-`0x07` (ripple / scanner / strobe) accepted but **not yet observed** |
| speed | 1-9 |
| brightness | 0-100; **off = static with brightness 0** (mode `0x00`, "off" in Venator, does nothing here) |
| direction | 1 or 2 (effect modes; not yet observed) |
| zone (byte 9) | `0x02` = the mirror. `0x01` only switches the Base Logo on (ignores everything else). `0x03` also hit the mirror. `0x00`, `0x04` are rejected (status 1) |

Whole-bar only: PredatorSense shows 7 zones but no per-zone write is known (byte 3 and byte 8
variants did nothing). `GetGamingKBBacklight` (0x15) does not reflect writes, so there is no readback.

### Base Logo (under the keyboard): method 0x0c `SetGamingLEDColor`

64-bit input `group | R<<8 | G<<16 | B<<24 | brightness<<32 | 0x08<<40`, group = 1 (the only LED
group: 2 and 3 are rejected). Confirmed: red, green, black (= off), the firmware default cyan
`00,ae,c7`; brightness `0x64` looked brighter than `0x50`; brightness 0 is also dark.
`GetGamingLEDColor(1)` (0x0d) reads back `{status, R, G, B, 0x50, 0x08, 0, 0}` with the color
just written (the brightness byte stays 0x50). `SetGamingLEDBehavior` (0x0a) with `group | value<<8`
changed nothing visible (and left `GetGamingLEDBehavior(1)` reading 0), so it is left alone; the
logo is static-only for now.

### Not the mirror / base logo

The LED getters (`GetGamingLED` 4, `GetGamingRgbKb` 7, `GetGamingLEDBehavior` 11, `GetGamingLEDColor`
13, `GetGamingSysInfo` 5, `GetGamingProfileSetting` 9) expose no per-zone state beyond the base
logo's color; the keyboard and MagKeys stay USB.

## Not Useful So Far

- `report 0x5A` group zones — HID accepts writes, no visible effect on the
  tested surfaces.

## Still Unknown

- Base Logo path.
- Infinity Mirror path.
- A general formula for ff02 commit33 words at arbitrary RGB (only the four
  baseline words above are confirmed).


## Backlight sleep timer and Acer WMI (findings, 2026-09)

**Timer.** Keyboard + WASD lights sleep 30 s after the last key press on the built-in
keyboard (measured +29.5 s). External-keyboard input does not count. Measured effects
on the timer (lights-off time after waking the keyboard with a key press):

| Action while lights are on | Lights went off at |
| --- | --- |
| nothing (control) | +29.5 s |
| per-key write (`report84`) at +20 s | ~+30 s (no reset) |
| MagKey frame write at +20 s | +55 s (reset) |

While the lights are off: a per-key write does not wake them; a MagKey frame write or a
full repaint does. The keep-alive feature is built on this.

**Not readable.** Nothing on the keyboard HID nodes reports the displayed colors:
feature reports `0x81`-`0x86` stall (`EPIPE`), `0x5A` returns a constant block, and the
ff02 node's 8-byte feature read returns a fixed echo that does not change with color.

**Acer WMI (gaming interface).** The firmware's own description (BMOF, decoded with
pali/bmfdec) lists class `AcerGamingFunction` (GUID `7A4DDFE7-5B5D-40B4-8595-4408E0CC7F56`)
with methods 1-25 (profile, LED, RGB keyboard `SetGamingRgbKb` 6/7, `SetGamingKBBacklight`
20/21, `SetGamingMiscSetting` 22/23, fan, overclock) and `BatteryControl`. It is object
`BH` on `\_SB.PC00.WMID`, i.e. ACPI method `\_SB.PC00.WMID.WMBH(instance, method, input)`.
Methods 20-23 end in `WSMI` (an SMI into BIOS), so the payload format is not visible in
ASL. No kernel driver binds it; `acpi_call` can reach it (root only).

`GetGamingMiscSetting` (method 23, input = setting index) returns `{status, value, ...}`;
`SetGamingMiscSetting` (22) takes `index | (value << 8)` (value in the second byte,
confirmed by readback). Valid settings on the PH18-72: `0x01`=0, `0x02`=3, `0x06`=1,
`0x07`=0xff, `0x08`=1, `0x09`=7, `0x0A`=0x73 (supported performance profiles bitmask),
`0x0B`=1 (performance profile); the rest report "unsupported". In Acer's published
layout `0x05`/`0x07` are overclocking and `0x0B` is the performance profile (setting it
also reprograms NVIDIA power limits): do not write those. Setting `0x06` or `0x08` to 0
did **not** change the 30 s sleep timer (each was restored and verified), so the timer
is not controlled there. The keyboard-backlight Get (method 21) returns a record with
brightness 0x64, not a timeout.
