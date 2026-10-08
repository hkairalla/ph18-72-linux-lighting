//! Lights driven through the Acer gaming WMI interface (BIOS), not USB: the Infinity Mirror
//! (rear light bar) and the Base Logo. Both go through `\_SB.PC00.WMID.WMBH` (instance 0):
//!
//! * Mirror: method 0x14 `SetGamingKBBacklight`, 16-byte buffer
//!   `[mode, speed, brightness, 0x00, direction, R, G, B, 0x03, 0x02, 0 x6]`
//!   (byte 9 is a zone id: 0x02 = mirror; 0x01 only switches the base logo on). Mode 0xff is
//!   static; "off" is static with brightness 0 (the driver's mode 0 does nothing here).
//! * Base logo: method 0x0c `SetGamingLEDColor`, 64-bit input
//!   `group(1) | R<<8 | G<<16 | B<<24 | brightness<<32 | 0x08<<40`. Only group 1 exists.
//!   Black is off; brightness 0 also reads as off.
//!
//! Layouts from the community Venator driver (PH16-71) and this machine's own readback,
//! confirmed on the PH18-72 (see docs/PROTOCOL_NOTES.md). Neither reports its state back in a
//! useful way, so the last write per light is persisted in `~/.cache/ph18-lighting/<zone>-state`.
//!
//! The writes need root (`/proc/acpi/call`): they go through the `ph18-lighting-wmi` helper
//! via `sudo -n` unless this process can write the file itself.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const HELPER_INSTALLED: &str = "/usr/local/libexec/ph18-lighting-wmi";
const ACPI_CALL: &str = "/proc/acpi/call";
const WMBH: &str = "\\_SB.PC00.WMID.WMBH 0x0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Mirror,
    BaseLogo,
}

impl Zone {
    pub fn as_str(self) -> &'static str {
        match self {
            Zone::Mirror => "mirror",
            Zone::BaseLogo => "base-logo",
        }
    }

    /// Human name for log lines.
    pub fn label(self) -> &'static str {
        match self {
            Zone::Mirror => "Infinity Mirror",
            Zone::BaseLogo => "Base Logo",
        }
    }

    fn state_path(self) -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(format!(".cache/ph18-lighting/{}-state", self.as_str())))
    }

    /// Has this light ever been set (is there a saved state to repaint)?
    pub fn is_saved(self) -> bool {
        self.state_path().map(|p| p.exists()).unwrap_or(false)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Static,
    Breathing,
    Neon,
    Rainbow,
    Wave,
    Ripple,
    Scanner,
    Strobe,
}

impl Mode {
    pub const ALL: [Mode; 8] = [
        Mode::Static, Mode::Breathing, Mode::Neon, Mode::Rainbow,
        Mode::Wave, Mode::Ripple, Mode::Scanner, Mode::Strobe,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Static => "static",
            Mode::Breathing => "breathing",
            Mode::Neon => "neon",
            Mode::Rainbow => "rainbow",
            Mode::Wave => "wave",
            Mode::Ripple => "ripple",
            Mode::Scanner => "scanner",
            Mode::Strobe => "strobe",
        }
    }

    pub fn parse(s: &str) -> Option<Mode> {
        Mode::ALL.iter().copied().find(|m| m.as_str() == s.trim().to_ascii_lowercase())
    }

    pub fn byte(self) -> u8 {
        match self {
            Mode::Static => 0xff,
            Mode::Breathing => 0x01,
            Mode::Neon => 0x02,
            Mode::Rainbow => 0x03,
            Mode::Wave => 0x04,
            Mode::Ripple => 0x05,
            Mode::Scanner => 0x06,
            Mode::Strobe => 0x07,
        }
    }
}

/// What a light was last told to show. `on == false` means it was written dark; the other
/// fields are kept so "on" restores the previous look. The base logo ignores mode/speed/direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LightState {
    pub zone: Zone,
    pub on: bool,
    pub mode: Mode,
    pub color: (u8, u8, u8),
    pub brightness: u8, // 0-100
    pub speed: u8,      // 1-9
    pub direction: u8,  // 1 or 2
}

/// One ACPI call: the method id and its argument as acpi_call spells it (`b<hex>` or `0x<hex>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub method: u8,
    pub arg: String,
}

impl LightState {
    /// The firmware's own default look (readback on a fresh PH18-72: color 00,ae,c7, speed 5).
    pub fn default_for(zone: Zone) -> Self {
        LightState { zone, on: true, mode: Mode::Static, color: (0x00, 0xae, 0xc7), brightness: 100, speed: 5, direction: 1 }
    }

    /// The ACPI call that shows this state.
    pub fn call(&self) -> Call {
        let (r, g, b) = self.color;
        let brightness = if self.on { self.brightness.min(100) } else { 0 };
        match self.zone {
            Zone::Mirror => {
                let buf = [self.mode.byte(), self.speed.clamp(1, 9), brightness, 0x00, self.direction.clamp(1, 2), r, g, b, 0x03, 0x02, 0, 0, 0, 0, 0, 0];
                Call { method: 0x14, arg: format!("b{}", hex(&buf)) }
            }
            Zone::BaseLogo => {
                // Off = black at brightness 0: either alone was enough on the test machine, both is safest.
                let (r, g, b) = if self.on { (r, g, b) } else { (0, 0, 0) };
                let v: u64 = 0x01 | (r as u64) << 8 | (g as u64) << 16 | (b as u64) << 24 | (brightness as u64) << 32 | 0x08 << 40;
                Call { method: 0x0c, arg: format!("0x{v:x}") }
            }
        }
    }

    pub fn serialize(&self) -> String {
        let (r, g, b) = self.color;
        format!(
            "on={}\nmode={}\ncolor={r},{g},{b}\nbrightness={}\nspeed={}\ndirection={}\n",
            self.on, self.mode.as_str(), self.brightness, self.speed, self.direction
        )
    }

    pub fn parse(zone: Zone, text: &str) -> LightState {
        let mut s = LightState::default_for(zone);
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let v = v.trim();
            match k.trim() {
                "on" => s.on = v == "true",
                "mode" => s.mode = Mode::parse(v).unwrap_or(s.mode),
                "color" => {
                    let p: Vec<u8> = v.split(',').filter_map(|c| c.trim().parse().ok()).collect();
                    if p.len() == 3 { s.color = (p[0], p[1], p[2]); }
                }
                "brightness" => s.brightness = v.parse::<u8>().map(|b| b.min(100)).unwrap_or(s.brightness),
                "speed" => s.speed = v.parse::<u8>().map(|x| x.clamp(1, 9)).unwrap_or(s.speed),
                "direction" => s.direction = v.parse::<u8>().map(|x| x.clamp(1, 2)).unwrap_or(s.direction),
                _ => {}
            }
        }
        s
    }

    pub fn load(zone: Zone) -> LightState {
        zone.state_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|t| LightState::parse(zone, &t))
            .unwrap_or_else(|| LightState::default_for(zone))
    }

    pub fn save(&self) -> io::Result<()> {
        let path = self.zone.state_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, self.serialize())?;
        fs::rename(tmp, path)
    }
}

fn hex(buf: &[u8]) -> String {
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// Where the root helper is: the installed copy (the one sudoers trusts), else a build next to
/// this binary (only usable when already root).
fn helper_path() -> PathBuf {
    if Path::new(HELPER_INSTALLED).exists() {
        return PathBuf::from(HELPER_INSTALLED);
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("ph18-lighting-wmi")))
        .unwrap_or_else(|| PathBuf::from(HELPER_INSTALLED))
}

/// Can this process write the ACPI call file itself (i.e. is it root, or was the file opened up)?
fn can_write_directly() -> bool {
    fs::OpenOptions::new().read(true).write(true).open(ACPI_CALL).is_ok()
}

/// Perform the call. Returns the ACPI reply text and how it was sent.
pub fn write(call: &Call) -> io::Result<(String, &'static str)> {
    if can_write_directly() {
        let mut f = fs::OpenOptions::new().read(true).write(true).open(ACPI_CALL)?;
        f.write_all(format!("{WMBH} {:#x} {}", call.method, call.arg).as_bytes())?;
        let mut out = String::new();
        f.read_to_string(&mut out)?;
        let out = out.trim_end_matches('\0').trim().to_string();
        if out.starts_with("Error") {
            return Err(io::Error::new(io::ErrorKind::Other, out));
        }
        return Ok((out, "direct"));
    }
    let helper = helper_path();
    let output = Command::new("sudo")
        .args(["-n", "--"])
        .arg(&helper)
        .args([&format!("{:#x}", call.method), &call.arg])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        let why = if stderr.contains("password") || stderr.contains("sudo:") {
            format!("sudo refused ({stderr}). Run omarchy/install.sh (step 4b) to install the helper and its sudoers rule")
        } else if !stderr.is_empty() {
            stderr
        } else {
            stdout.clone()
        };
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, why));
    }
    Ok((stdout, "sudo-helper"))
}

/// Whether the no-password path is set up (helper installed and sudoers accepts it).
pub fn helper_ready() -> bool {
    if can_write_directly() {
        return true;
    }
    Path::new(HELPER_INSTALLED).exists()
        && Command::new("sudo")
            .args(["-n", "-l", "--", HELPER_INSTALLED])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_call_matches_the_confirmed_packets() {
        let red = LightState { zone: Zone::Mirror, on: true, mode: Mode::Static, color: (255, 0, 0), brightness: 100, speed: 5, direction: 1 };
        assert_eq!(red.call(), Call { method: 0x14, arg: "bff05640001ff00000302000000000000".into() });
        let off = LightState { on: false, ..red };
        assert_eq!(off.call().arg, "bff05000001ff00000302000000000000");
        let wave = LightState { mode: Mode::Wave, speed: 9, direction: 2, ..red };
        assert_eq!(wave.call().arg, "b0409640002ff00000302000000000000");
    }

    #[test]
    fn base_logo_call_matches_the_confirmed_values() {
        // Round 5 on the PH18-72: 0x08500000ff01 = red at brightness 0x50, 0x0850c7ae0001 = the default cyan.
        let red = LightState { zone: Zone::BaseLogo, on: true, mode: Mode::Static, color: (255, 0, 0), brightness: 80, speed: 5, direction: 1 };
        assert_eq!(red.call(), Call { method: 0x0c, arg: "0x8500000ff01".into() });
        let cyan = LightState { color: (0x00, 0xae, 0xc7), ..red };
        assert_eq!(cyan.call().arg, "0x850c7ae0001");
        let off = LightState { on: false, ..red };
        assert_eq!(off.call().arg, "0x80000000001");
    }

    #[test]
    fn state_round_trips() {
        let s = LightState { zone: Zone::BaseLogo, on: false, mode: Mode::Wave, color: (1, 2, 3), brightness: 42, speed: 9, direction: 2 };
        assert_eq!(LightState::parse(Zone::BaseLogo, &s.serialize()), s);
        assert_eq!(LightState::parse(Zone::Mirror, ""), LightState::default_for(Zone::Mirror));
        assert_eq!(
            LightState::parse(Zone::Mirror, "speed=99\nbrightness=200\nmode=bogus\n"),
            LightState { speed: 9, brightness: 100, ..LightState::default_for(Zone::Mirror) }
        );
    }

    #[test]
    fn mode_names_round_trip() {
        for m in Mode::ALL {
            assert_eq!(Mode::parse(m.as_str()), Some(m));
        }
        assert_eq!(Mode::parse("off"), None);
    }
}
