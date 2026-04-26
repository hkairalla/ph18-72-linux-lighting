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
