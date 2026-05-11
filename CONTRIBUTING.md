# Contributing

This project targets the Acer Predator Helios 18 PH18-72 lighting controllers.

## Before Sending Device Reports

Please include:

- Exact model name from `/sys/class/dmi/id/product_name`
- Product family from `/sys/class/dmi/id/product_family`
- BIOS version from `/sys/class/dmi/id/bios_version`
- Linux distribution and kernel version
- Which lighting surfaces changed, and which did not

Do not attach raw packet captures to issues unless asked. Captures can contain local USB device details and should be shared deliberately.

## Development Notes

- Keep hardware writes in the Rust daemon.
- Keep the GUI unprivileged.
- Keep reverse-engineering captures and scratch logs out of Git.
- Prefer small, repeatable hardware tests with clear observations.

## Reverse-engineering New Surfaces

If you're extending support to another Acer model or to currently-unknown
surfaces on the PH18-72 (Base Logo, Infinity Mirror, etc.), the methodology
that produced the current protocol notes was:

1. **Capture in Windows with USBPcap + Wireshark.** PredatorSense is the only
   reliable way to trigger the firmware's mode transitions; capturing on
   Linux misses the source-select handshakes.
   - Install USBPcap (https://desowin.org/usbpcap/) and Wireshark.
   - Identify the USB device by VID:PID — `05af:866a` for keyboard /
     MagKey, `0d62:ba51` for the Darfon cover logo.
   - Take **two short captures of similar length**: a "before" capture in
     PredatorSense's default state, and an "after" capture where you've
     performed exactly **one** action (e.g. switched to static / per-key
     custom mode, set a single color, toggled brightness). Single-action
     diffs are what make the next step tractable.

2. **Diff captures on Linux with `tshark`.** Extract feature reports and
   output frames going *to* the device, then look for payloads that appear
   in "after" but not "before" (or with materially higher counts). A
   reference diff/replay helper exists under
   `testing/imports/ph18_72_research/standalone/tests/keyboard/keyboard_windows_capture_gate_hunt.py`
   (in the git-ignored private research workspace, not in this repo). The
   relevant filter is roughly:

   ```
   usbhid.data && usb.endpoint_address.direction==0
   ```

3. **Replay candidates one at a time on Linux** through the existing
   daemon's vendor HID node. Confirm visible behavior before promoting a
   packet from "experimental candidate" to a daemon command.

4. **Sanitize before publishing.** Raw `.pcapng` files, USBPcap logs,
   PredatorSense screenshots, and large command-output dumps stay out of
   this repo — they often contain unrelated USB device details. Only the
   distilled byte-level protocol summary lands in
   [docs/PROTOCOL_NOTES.md](docs/PROTOCOL_NOTES.md).

If you find new working packets and want to upstream them, opening an issue
with the *observed behavior* (which surface changed, what it looked like)
plus the *minimum* byte sequence that reproduces it is the most useful form.
We will not ask for the raw capture file.
