//! Keep the keyboard and MagKey lights from going to sleep.
//!
//! The firmware turns the backlight off 30 s after the last key press on the
//! laptop keyboard. A MagKey frame write wakes it and restarts that 30 s
//! (measured: lights went off at +55 s when a write was sent at +20 s, versus
//! +29.5 s with no write). A plain per-key write does neither. So a periodic
//! rewrite of the saved MagKey frame is an invisible "activity" signal.
//!
//! Modes:
//!   off      do nothing (firmware default: lights sleep after 30 s)
//!   active   lights stay on while you are using the computer (any input device);
//!            they sleep once you have been idle as long as the Omarchy screensaver delay
//!   always   lights never sleep
//!   timeout  lights stay on until you have been idle for `minutes`
//! Optionally only while on AC power.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, SystemTime};

use crate::idle::IdleWatcher;

/// How often the saved frame is re-sent. Well inside the firmware's 30 s.
const TICK_SECS: u64 = 20;
/// If a MagKey frame was written this recently (e.g. the UI's live animation is
/// streaming frames), the lights are already being kept awake: skip our write.
const RECENT_WRITE_SECS: u64 = 15;
const DEFAULT_SCREENSAVER_SECS: u64 = 150;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Off,
    Active,
    Always,
    Timeout,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Active => "active",
            Mode::Always => "always",
            Mode::Timeout => "timeout",
        }
    }

    pub fn parse(s: &str) -> Option<Mode> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" => Some(Mode::Off),
            "active" => Some(Mode::Active),
            "always" => Some(Mode::Always),
            "timeout" => Some(Mode::Timeout),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub mode: Mode,
    pub minutes: u32,
    pub ac_only: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { mode: Mode::Off, minutes: 5, ac_only: true }
    }
}

impl Config {
    pub fn parse(text: &str) -> Config {
        let mut cfg = Config::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else { continue };
            match key.trim() {
                "mode" => {
                    if let Some(m) = Mode::parse(value) {
                        cfg.mode = m;
                    }
                }
                "minutes" => {
                    if let Ok(v) = value.trim().parse::<u32>() {
                        cfg.minutes = v.clamp(1, 1440);
                    }
                }
                "ac_only" => {
                    if let Some(v) = parse_bool(value) {
                        cfg.ac_only = v;
                    }
                }
                _ => {}
            }
        }
        cfg
    }

    pub fn serialize(&self) -> String {
        format!(
            "# PH18-72 lighting keep-alive (edited by `ph18-lighting-daemon set-keepalive`)\nmode={}\nminutes={}\nac_only={}\n",
            self.mode.as_str(),
            self.minutes,
            self.ac_only
        )
    }

    pub fn load() -> Config {
        config_path()
            .ok()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|t| Config::parse(&t))
            .unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_path()?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, self.serialize())?;
        fs::rename(&tmp, &path)
    }

    /// Seconds of no input after which the session counts as idle, for modes that
    /// depend on idle time.
    pub fn idle_threshold_secs(&self) -> Option<u64> {
        match self.mode {
            Mode::Active => Some(omarchy_screensaver_secs()),
            Mode::Timeout => Some(u64::from(self.minutes) * 60),
            Mode::Off | Mode::Always => None,
        }
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn home() -> io::Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))
}

fn config_path() -> io::Result<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => home()?.join(".config"),
    };
    Ok(base.join("ph18-lighting").join("keepalive.conf"))
}

fn last_write_path() -> io::Result<PathBuf> {
    Ok(home()?.join(".cache").join("ph18-lighting").join("last-magkey-write"))
}

/// Record that a MagKey frame was just written (by anyone). Best effort.
pub fn touch_magkey_write() {
    let Ok(path) = last_write_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(f) = fs::OpenOptions::new().create(true).write(true).truncate(false).open(&path) {
        let _ = f.set_modified(SystemTime::now());
    }
}

fn secs_since_magkey_write() -> u64 {
    last_write_path()
        .ok()
        .and_then(|p| fs::metadata(p).ok())
        .and_then(|m| m.modified().ok())
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .map(|d| d.as_secs())
        .unwrap_or(u64::MAX)
}

/// Omarchy's screensaver delay (`idle.screensaver` in shell.json), so "active"
/// means the same as it does for the screensaver.
pub fn omarchy_screensaver_secs() -> u64 {
    let path = match home() {
        Ok(h) => h.join(".config/omarchy/shell.json"),
        Err(_) => return DEFAULT_SCREENSAVER_SECS,
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|t| parse_screensaver_secs(&t))
        .unwrap_or(DEFAULT_SCREENSAVER_SECS)
}

fn parse_screensaver_secs(json: &str) -> Option<u64> {
    let after = &json[json.find("\"screensaver\"")? + "\"screensaver\"".len()..];
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<u64>().ok().filter(|v| *v > 0)
}

/// True when on AC power (or when there is no battery/mains info at all).
fn on_ac_power() -> bool {
    let Ok(entries) = fs::read_dir("/sys/class/power_supply") else { return true };
    let mut saw_mains = false;
    for entry in entries.flatten() {
        let dir = entry.path();
        let is_mains = fs::read_to_string(dir.join("type"))
            .map(|t| t.trim() == "Mains")
            .unwrap_or(false);
        if !is_mains {
            continue;
        }
        saw_mains = true;
        if fs::read_to_string(dir.join("online")).map(|v| v.trim() == "1").unwrap_or(false) {
            return true;
        }
    }
    !saw_mains
}

pub struct Inputs {
    pub idle: bool,
    pub on_ac: bool,
    pub secs_since_magkey_write: u64,
}

/// Should the saved MagKey frame be re-sent right now?
pub fn should_write(cfg: &Config, now: &Inputs) -> bool {
    if cfg.mode == Mode::Off {
        return false;
    }
    if cfg.ac_only && !now.on_ac {
        return false;
    }
    if now.secs_since_magkey_write < RECENT_WRITE_SECS {
        return false;
    }
    match cfg.mode {
        Mode::Always => true,
        Mode::Active | Mode::Timeout => !now.idle,
        Mode::Off => false,
    }
}

fn config_mtime() -> Option<SystemTime> {
    config_path().ok().and_then(|p| fs::metadata(p).ok()).and_then(|m| m.modified().ok())
}

/// Send the saved MagKey frame (all-off if none was ever saved).
fn send_saved_frame() -> io::Result<()> {
    let frame = crate::load_keyboard_state().magkeys.unwrap_or([(0, 0, 0); 12]);
    crate::apply_magkey_frame_raw(&crate::build_magkey_frame(&frame)).map(|_| ())
}

/// The long-running loop behind `keepalive-run`.
pub fn run() -> io::Result<()> {
    let mut watcher: Option<(IdleWatcher, std::sync::mpsc::Receiver<()>)> = None;
    let mut watcher_error_logged = false;
    let mut last_state = String::new();

    loop {
        let cfg = Config::load();
        let cfg_stamp = config_mtime();

        // Idle tracking is only needed for the idle-based modes.
        match cfg.idle_threshold_secs() {
            Some(secs) => {
                if watcher.is_none() && !watcher_error_logged {
                    match IdleWatcher::start() {
                        Ok(w) => watcher = Some(w),
                        Err(e) => {
                            eprintln!("keepalive: idle detection unavailable ({e}); treating the session as always active");
                            watcher_error_logged = true;
                        }
                    }
                }
                if let Some((w, _)) = watcher.as_mut() {
                    if let Err(e) = w.watch(Duration::from_secs(secs)) {
                        eprintln!("keepalive: could not set idle threshold: {e}");
                    }
                }
            }
            None => {}
        }

        let inputs = Inputs {
            idle: watcher.as_ref().map(|(w, _)| w.is_idle()).unwrap_or(false),
            on_ac: on_ac_power(),
            secs_since_magkey_write: secs_since_magkey_write(),
        };
        let write = should_write(&cfg, &inputs);

        let state = format!(
            "mode={} idle={} ac={} write={}",
            cfg.mode.as_str(),
            inputs.idle,
            inputs.on_ac,
            write
        );
        if state != last_state {
            println!("keepalive: {state}");
            last_state = state;
        }

        if write {
            match send_saved_frame() {
                Ok(()) => touch_magkey_write(),
                Err(e) => eprintln!("keepalive: write failed (device busy or absent?): {e}"),
            }
        }

        // Sleep until the next tick, but wake early if the user becomes active
        // again (so sleeping lights come back right away) or the settings change.
        let mut waited = 0;
        while waited < TICK_SECS {
            if let Some((_, rx)) = watcher.as_ref() {
                match rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(()) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => std::thread::sleep(Duration::from_secs(1)),
                }
            } else {
                std::thread::sleep(Duration::from_secs(1));
            }
            waited += 1;
            if config_mtime() != cfg_stamp {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(idle: bool, on_ac: bool, since: u64) -> Inputs {
        Inputs { idle, on_ac, secs_since_magkey_write: since }
    }

    fn cfg(mode: Mode, ac_only: bool) -> Config {
        Config { mode, minutes: 5, ac_only }
    }

    #[test]
    fn off_never_writes() {
        assert!(!should_write(&cfg(Mode::Off, false), &inputs(false, true, 999)));
    }

    #[test]
    fn always_writes_even_when_idle() {
        assert!(should_write(&cfg(Mode::Always, false), &inputs(true, true, 999)));
    }

    #[test]
    fn active_and_timeout_stop_when_idle() {
        for mode in [Mode::Active, Mode::Timeout] {
            assert!(should_write(&cfg(mode, false), &inputs(false, true, 999)));
            assert!(!should_write(&cfg(mode, false), &inputs(true, true, 999)));
        }
    }

    #[test]
    fn ac_only_blocks_on_battery_but_not_when_disabled() {
        assert!(!should_write(&cfg(Mode::Always, true), &inputs(false, false, 999)));
        assert!(should_write(&cfg(Mode::Always, false), &inputs(false, false, 999)));
        assert!(should_write(&cfg(Mode::Always, true), &inputs(false, true, 999)));
    }

    #[test]
    fn skips_when_a_frame_was_just_written() {
        assert!(!should_write(&cfg(Mode::Always, false), &inputs(false, true, 3)));
        assert!(should_write(&cfg(Mode::Always, false), &inputs(false, true, RECENT_WRITE_SECS)));
    }

    #[test]
    fn config_round_trips() {
        let c = Config { mode: Mode::Timeout, minutes: 12, ac_only: false };
        assert_eq!(Config::parse(&c.serialize()), c);
        assert_eq!(Config::parse(""), Config::default());
    }

    #[test]
    fn config_parse_is_forgiving() {
        let c = Config::parse("mode = ACTIVE\nminutes=99999\nac_only=no\nbogus\nmode2=x\n");
        assert_eq!(c.mode, Mode::Active);
        assert_eq!(c.minutes, 1440);
        assert!(!c.ac_only);
        // A bad mode keeps the default instead of failing.
        assert_eq!(Config::parse("mode=banana").mode, Mode::Off);
    }

    #[test]
    fn idle_thresholds() {
        assert_eq!(cfg(Mode::Off, true).idle_threshold_secs(), None);
        assert_eq!(cfg(Mode::Always, true).idle_threshold_secs(), None);
        assert_eq!(Config { mode: Mode::Timeout, minutes: 7, ac_only: true }.idle_threshold_secs(), Some(420));
        assert!(cfg(Mode::Active, true).idle_threshold_secs().is_some());
    }

    #[test]
    fn screensaver_seconds_parse_from_shell_json() {
        let json = r#"{ "version": 1, "idle": { "screensaver": 150, "lock": 300 } }"#;
        assert_eq!(parse_screensaver_secs(json), Some(150));
        assert_eq!(parse_screensaver_secs(r#"{"idle":{"lock":300}}"#), None);
        assert_eq!(parse_screensaver_secs(r#"{"idle":{"screensaver": 0}}"#), None);
    }
}
