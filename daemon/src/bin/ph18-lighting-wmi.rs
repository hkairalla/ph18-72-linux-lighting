//! Root helper for the two Acer gaming WMI lighting calls.
//!
//! The Infinity Mirror (rear light bar) and the Base Logo are not USB devices: they are driven
//! through the BIOS, `\_SB.PC00.WMID.WMBH`, reachable only through the `acpi_call` kernel
//! module's `/proc/acpi/call`, which is root-only.
//!
//! This binary is the only thing that needs root. It accepts exactly two calls and nothing else:
//!
//!   ph18-lighting-wmi 0x14 b<32 hex digits>    SetGamingKBBacklight: the mirror's 16-byte buffer
//!   ph18-lighting-wmi 0xc  0x<hex>             SetGamingLEDColor: the base logo (LED group 1 only)
//!
//! The installer copies it to /usr/local/libexec (root-owned) and adds a sudoers rule so the
//! user's daemon can run it without a password:
//!   `<user> ALL=(root) NOPASSWD: /usr/local/libexec/ph18-lighting-wmi`
//! Prints the ACPI result (e.g. `{0x00, 0x00, 0x00, 0x00}`), exit 0 on success.

use std::fs;
use std::io::{Read, Write};
use std::process::ExitCode;

const ACPI_CALL: &str = "/proc/acpi/call";
const WMBH: &str = "\\_SB.PC00.WMID.WMBH 0x0";

fn fail(msg: &str) -> ExitCode {
    eprintln!("ph18-lighting-wmi: {msg}");
    ExitCode::from(2)
}

/// Validate the argument for the method; returns the normalised argument.
fn check(method: &str, arg: &str) -> Result<(&'static str, String), &'static str> {
    let arg = arg.to_ascii_lowercase();
    match method {
        "0x14" => {
            let hex = arg.strip_prefix('b').ok_or("0x14 takes b<32 hex digits>")?;
            if hex.len() != 32 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("the backlight buffer must be exactly 16 bytes as 32 hex digits");
            }
            // Byte 9 is the zone: only the mirror (0x02) is driven this way.
            if &hex[18..20] != "02" {
                return Err("the backlight buffer may only address zone 0x02 (the mirror)");
            }
            Ok(("0x14", format!("b{hex}")))
        }
        "0xc" | "0x0c" => {
            let hex = arg.strip_prefix("0x").ok_or("0xc takes 0x<hex>")?;
            if hex.is_empty() || hex.len() > 16 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("the LED color value must be a hex integer of at most 8 bytes");
            }
            let v = u64::from_str_radix(hex, 16).map_err(|_| "bad hex")?;
            // Low byte is the LED group: only group 1 (the base logo) exists; the two high bytes
            // (brightness, 0x08) are the only other fields the firmware accepts.
            if v & 0xff != 0x01 || v >> 48 != 0 || (v >> 40) & 0xff != 0x08 {
                return Err("the LED color value must be group 1 with the 0x08 tag: 0x08<bri><B><G><R>01");
            }
            Ok(("0xc", format!("0x{v:x}")))
        }
        _ => Err("only methods 0x14 (mirror backlight) and 0xc (base logo color) are allowed"),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [method, arg] = args.as_slice() else {
        return fail("usage: ph18-lighting-wmi 0x14 b<32 hex> | ph18-lighting-wmi 0xc 0x<hex>");
    };
    let (method, arg) = match check(method, arg) {
        Ok(v) => v,
        Err(e) => return fail(e),
    };

    // Load acpi_call if the file is missing (the module is a DKMS package, not autoloaded).
    if fs::metadata(ACPI_CALL).is_err() {
        let _ = std::process::Command::new("modprobe").arg("acpi_call").status();
        if fs::metadata(ACPI_CALL).is_err() {
            return fail("/proc/acpi/call is missing: install the acpi_call-dkms package");
        }
    }

    let call = format!("{WMBH} {method} {arg}");
    let mut f = match fs::OpenOptions::new().read(true).write(true).open(ACPI_CALL) {
        Ok(f) => f,
        Err(e) => return fail(&format!("cannot open {ACPI_CALL} (need root): {e}")),
    };
    if let Err(e) = f.write_all(call.as_bytes()) {
        return fail(&format!("write failed: {e}"));
    }
    let mut out = String::new();
    if let Err(e) = f.read_to_string(&mut out) {
        return fail(&format!("read failed: {e}"));
    }
    let out = out.trim_end_matches('\0').trim().to_string();
    println!("{out}");
    if out.starts_with("Error") || out.starts_with("not called") {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn accepts_only_the_two_known_calls() {
        assert!(check("0x14", "bff05640001ff00000302000000000000").is_ok());
        assert!(check("0x14", "bff05640001ff00000301000000000000").is_err()); // base logo zone
        assert!(check("0x14", "bff0564").is_err());
        assert!(check("0xc", "0x08500000ff01").is_ok());
        assert!(check("0xc", "0x08500000ff02").is_err()); // group 2
        assert!(check("0xc", "0x00500000ff01").is_err()); // missing tag
        assert!(check("0x16", "0x1").is_err()); // SetGamingMiscSetting: never
        assert!(check("0xa", "0x0101").is_err());
    }
}
