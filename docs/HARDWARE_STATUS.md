# Hardware Status

Current public status for Acer Predator Helios 18 PH18-72 lighting support.

| Surface | Status | Backend |
| --- | --- | --- |
| MagKey 3.0 / WASD overlay | Working | HID `05af:866a` ff02 LED-map path |
| Cover Logo | Working | HID `0d62:ba51` Darfon short-command path |
| Main keyboard whole-color blue | Working | HID `05af:866a` ff02 commit33 path |
| Main keyboard per-key | Not working visibly | `report84` writes are accepted but do not visibly latch |
| `report 0x5A` group zones | Not useful so far | HID accepts writes but visible surfaces mostly do not change |
| Base Logo | Unknown | HID captures inconclusive; WMI/ACPI remains possible |
| Infinity Mirror | Unknown | HID captures inconclusive; WMI/ACPI remains possible |

The local reverse-engineering logs and packet captures live under ignored `testing/` in this workspace and should not be committed to the public app repo.
