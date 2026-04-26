# Protocol Notes

This file is the sanitized public record of what the private research workspace has confirmed. Keep raw packet captures, exploratory scripts, screenshots, and command logs out of this repo.

## Known Controllers

| Device | Role |
| --- | --- |
| HID `05af:866a` | Acer/Jing-Mold keyboard-class controller. Used for main keyboard and MagKey paths. |
| HID `0d62:ba51` | Darfon cover-logo controller. Used for short cover-logo color and brightness commands. |
| Acer WMI/ACPI | Present on the machine and still plausible for unresolved Base Logo / Infinity Mirror work. Treat as read-only until methods are understood. |

## Main Keyboard

The visible whole-keyboard path is the `05af:866a` ff02 / commit33 flow.

Confirmed:

- `ff0000ff` makes the main keyboard blue.
- The hybrid restore flow can set the main keyboard plus MagKeys blue.

Observed caveats:

- The old `report84` per-key path accepts writes but does not visibly latch.
- `report86=1` returns the keyboard to firmware dynamic/default behavior.
- `report86=0` behaved statefully in testing; in the latest run it restored blue rather than acting as a reliable blackout.
- Early red/green labels were misleading:
  - `00ff0000` produced an off/blackout-like result.
  - `0000ff00` produced red-ish main keyboard output while MagKeys stayed off.

## MagKey 3.0

The visible MagKey path is `05af:866a` ff02 with an init, 64-byte LED map, and commit.

Confirmed:

- All red works.
- All green works.
- All blue works.
- All off works.
- Split colors work with caveats.

Known caveat:

- A/S/D share slots in ways that can cause color bleed, especially when blue is involved. The safe mode intentionally sacrifices some blue behavior to reduce bleed.

## Cover Logo

The visible Cover Logo path is HID `0d62:ba51`, but it is currently being re-verified in the public app workflow rather than treated as settled.

Confirmed:

- Brightness control works.
- Whole-logo red, green, and blue work.
- Left segment works.
- Middle segment works, but segments visually blend.
- Right segment is likely correct, but one test was not isolated because it may already have been blue.

## Not Useful So Far

`report 0x5A` group writes are accepted by the HID device but have not produced useful visible RGB changes on tested surfaces.

Treat `report 0x5A` as low priority unless new evidence appears.

## Still Unknown

- Base Logo
- Infinity Mirror
- Reliable main-keyboard red/green/white words for the ff02 commit33 path
- Per-key visible latch/source-select behavior

## What To Bring From Research Into This Repo

Bring:

- Confirmed controller IDs.
- Confirmed semantic behavior.
- Clean packet-builder code once implemented in Rust.
- Small non-hardware tests for packet builders.

Do not bring:

- `.pcapng` captures.
- USBPcap logs.
- Screenshots from PredatorSense.
- Imported experimental repos.
- Large command-output logs.
- Scripts that blindly sweep hardware packets.
