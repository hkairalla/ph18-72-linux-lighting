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

use crate::animation;
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

/// Do the current settings want the lights on right now? (The keep-alive rewrite and the
/// animation both follow this.)
pub fn lights_should_be_on(cfg: &Config, idle: bool, on_ac: bool) -> bool {
    if cfg.mode == Mode::Off {
        return false;
    }
    if cfg.ac_only && !on_ac {
        return false;
    }
    match cfg.mode {
        Mode::Always => true,
        Mode::Active | Mode::Timeout => !idle,
        Mode::Off => false,
    }
}

/// Should the saved MagKey frame be re-sent right now?
pub fn should_write(cfg: &Config, now: &Inputs) -> bool {
    // A frame written moments ago (e.g. by the animation) already keeps the lights awake.
    if now.secs_since_magkey_write < RECENT_WRITE_SECS {
        return false;
    }
    lights_should_be_on(cfg, now.idle, now.on_ac)
}

// ── A lock so HID writers never interleave ────────────────────────────────────────────
// A whole-board sweep is a long run of packets and the animation streams a frame every 40 ms;
// if both wrote at once the packets would mix. Sweeps take this lock exclusively (see
// `repaint_keyboard`), and the animation takes it shared and non-blocking for each burst.

/// Holds an flock; the lock is released when this is dropped (the file is closed).
pub struct HidLock(#[allow(dead_code)] fs::File);

fn hid_lock_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"));
    dir.join("ph18-lighting-hid.lock")
}

fn open_hid_lock() -> io::Result<fs::File> {
    fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(hid_lock_path())
}

fn flock(file: &fs::File, op: i32) -> bool {
    use std::os::fd::AsRawFd;
    // SAFETY: plain syscall on a file descriptor we own for the duration of the call.
    unsafe { libc::flock(file.as_raw_fd(), op) == 0 }
}

/// Wait for, then hold, the exclusive HID lock (released when dropped).
pub fn hid_lock_exclusive() -> io::Result<HidLock> {
    let f = open_hid_lock()?;
    if flock(&f, libc::LOCK_EX) {
        Ok(HidLock(f))
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Take the shared lock if no sweep is running; `None` means "a sweep is in progress, skip".
fn hid_lock_shared_try() -> Option<HidLock> {
    let f = open_hid_lock().ok()?;
    flock(&f, libc::LOCK_SH | libc::LOCK_NB).then(|| HidLock(f))
}

fn config_mtime() -> Option<SystemTime> {
    config_path().ok().and_then(|p| fs::metadata(p).ok()).and_then(|m| m.modified().ok())
}

/// Send the saved MagKey frame (all-off if none was ever saved).
fn send_saved_frame() -> io::Result<()> {
    let frame = crate::load_keyboard_state().magkeys.unwrap_or([(0, 0, 0); 12]);
    crate::apply_magkey_frame_raw(&crate::build_magkey_frame(&frame)).map(|_| ())
}

// ── Animation output ──────────────────────────────────────────────────────────────────

const FRAME_PERIOD: Duration = Duration::from_millis(40); // 25 frames per second
const BURST_FRAMES: u32 = 5; // re-read the settings every 200 ms

/// The open connection to the MagKey controller for the animation: the init packets are sent
/// once, then each frame is one 64-byte write plus the commit packet. Dropped on any error so the
/// next burst finds the device again (it re-enumerates after suspend).
struct AnimLink {
    node: Option<PathBuf>,
}

impl AnimLink {
    fn send(&mut self, frame: &animation::Frame) -> io::Result<()> {
        let node = match &self.node {
            Some(n) => n.clone(),
            None => {
                let n = crate::find_ff02_node()?;
                for packet in crate::PKT_PRELUDE {
                    crate::send_feature_ff02(&n, &packet)?;
                }
                self.node = Some(n.clone());
                n
            }
        };
        let result = crate::send_out64(&node, &crate::build_magkey_frame(frame))
            .and_then(|_| crate::send_feature_ff02(&node, &crate::MAGKEY_COMMIT_PACKET));
        if result.is_err() {
            self.node = None;
        }
        result
    }
}

/// Play `BURST_FRAMES` frames at 25 fps. Returns false if the device could not be written.
fn animate_burst(link: &mut AnimLink, anim: &animation::Config, mode: animation::Mode) -> bool {
    let Some(_lock) = hid_lock_shared_try() else {
        std::thread::sleep(FRAME_PERIOD * BURST_FRAMES); // a sweep is running: wait it out
        return true;
    };
    let start = std::time::Instant::now();
    for n in 0..BURST_FRAMES {
        let frame = animation::frame(mode, anim.t_at(animation::now_ms()));
        if link.send(&frame).is_err() {
            return false;
        }
        let target = start + FRAME_PERIOD * (n + 1);
        if let Some(wait) = target.checked_duration_since(std::time::Instant::now()) {
            std::thread::sleep(wait);
        }
    }
    true
}

/// The long-running loop behind `keepalive-run`.
pub fn run() -> io::Result<()> {
    let mut watcher: Option<(IdleWatcher, std::sync::mpsc::Receiver<()>)> = None;
    let mut watcher_error_logged = false;
    let mut last_state = String::new();
    let mut link = AnimLink { node: None };
    let mut was_animating = false;
    let mut last_touch = std::time::Instant::now();
    let mut link_error_logged = false;

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
        // ── An animation is selected: play it while the keep-alive rules want the lights on ──
        let anim = animation::Config::load();
        if let Some(mode) = anim.mode {
            let on = lights_should_be_on(&cfg, inputs.idle, inputs.on_ac);
            let state = format!(
                "animation={} on={} keepalive={} idle={} ac={}",
                mode.as_str(), on, cfg.mode.as_str(), inputs.idle, inputs.on_ac
            );
            if state != last_state {
                println!("keepalive: {state}");
                last_state = state;
            }
            was_animating = true;
            if on {
                if animate_burst(&mut link, &anim, mode) {
                    link_error_logged = false;
                    if last_touch.elapsed() >= Duration::from_secs(1) {
                        touch_magkey_write();
                        last_touch = std::time::Instant::now();
                    }
                } else {
                    if !link_error_logged {
                        eprintln!("keepalive: animation write failed (device busy or absent?); retrying");
                        link_error_logged = true;
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
            } else if let Some((_, rx)) = watcher.as_ref() {
                // Lights are meant to sleep: idle, or on battery. Wake as soon as the user is back.
                let _ = rx.recv_timeout(Duration::from_secs(1));
            } else {
                std::thread::sleep(Duration::from_secs(1));
            }
            continue;
        }
        if was_animating {
            // The animation was just turned off: bring back the saved static colors.
            was_animating = false;
            link.node = None;
            last_state.clear();
            match send_saved_frame() {
                Ok(()) => touch_magkey_write(),
                Err(e) => eprintln!("keepalive: could not restore the static colors: {e}"),
            }
        }

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
        let anim_stamp = animation_mtime();
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
            if config_mtime() != cfg_stamp || animation_mtime() != anim_stamp {
                break;
            }
        }
    }
}

/// Modification time of the animation config, so starting or stopping one wakes the loop at once.
fn animation_mtime() -> Option<std::time::SystemTime> {
    animation::path().ok().and_then(|p| fs::metadata(p).ok()).and_then(|m| m.modified().ok())
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
    fn lights_policy_ignores_how_recently_we_wrote() {
        // Used by the animation: it must not be gated by the "recent write" rule.
        assert!(lights_should_be_on(&cfg(Mode::Always, false), false, true));
        assert!(!lights_should_be_on(&cfg(Mode::Off, false), false, true));
        assert!(!lights_should_be_on(&cfg(Mode::Active, false), true, true));
        assert!(!lights_should_be_on(&cfg(Mode::Always, true), false, false));
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
