//! MagKey (WASD) animations, computed in the background service so they keep running after the
//! GUI is closed.
//!
//! The seven animations are pure functions of time, ported 1:1 from the GUI's `MODES` table
//! (app.js), which still draws a live preview of the same thing. Both sides derive the animation
//! clock from the same three numbers in `animation.conf`:
//!
//!     t = phase0 + (now - epoch_ms) / 1000 * speed
//!
//! so the preview in the GUI and the real keys stay in step, and changing the speed continues the
//! animation smoothly (phase0 is re-based at the moment of the change).

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub type Rgb = (u8, u8, u8);
pub type Frame = [Rgb; 12];

pub const MIN_SPEED: f64 = 0.1;
pub const MAX_SPEED: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Wheel,
    Knight,
    Hue,
    Chase,
    Breathe,
    Zone,
    Cascade,
}

impl Mode {
    pub const ALL: [Mode; 7] = [
        Mode::Wheel,
        Mode::Knight,
        Mode::Hue,
        Mode::Chase,
        Mode::Breathe,
        Mode::Zone,
        Mode::Cascade,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Wheel => "wheel",
            Mode::Knight => "knight",
            Mode::Hue => "hue",
            Mode::Chase => "chase",
            Mode::Breathe => "breathe",
            Mode::Zone => "zone",
            Mode::Cascade => "cascade",
        }
    }

    pub fn parse(s: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.as_str() == s.trim().to_ascii_lowercase())
    }
}

// ── Geometry of the 12 emitters (W: left/top/right, A, S, D), from a photo of the keys ──
const EMITTER_POS: [[f64; 2]; 12] = [
    [0.297, 0.825], [0.600, 1.350], [0.903, 0.825],
    [-0.303, -0.175], [0.000, 0.350], [0.303, -0.175],
    [0.697, -0.175], [1.000, 0.350], [1.303, -0.175],
    [1.697, -0.175], [2.000, 0.350], [2.303, -0.175],
];
const CX: f64 = 0.90;
const CY: f64 = 0.25;

fn x_min() -> f64 {
    EMITTER_POS.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min)
}
fn x_max() -> f64 {
    EMITTER_POS.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max)
}

fn emitter_angle(i: usize) -> f64 {
    let [x, y] = EMITTER_POS[i];
    (((y - CY).atan2(x - CX) / (2.0 * std::f64::consts::PI)) + 1.0) % 1.0
}
fn norm_x(i: usize) -> f64 {
    (EMITTER_POS[i][0] - x_min()) / (x_max() - x_min())
}

// JS `Math.round` on a non-negative number equals Rust's `round` (half away from zero).
fn to_u8(c: f64) -> u8 {
    (c * 255.0).round().clamp(0.0, 255.0) as u8
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> Rgb {
    let h = h % 1.0;
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match (i as i64).rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    (to_u8(r), to_u8(g), to_u8(b))
}

fn pulse(t: f64, spd: f64) -> f64 {
    ((t * spd * std::f64::consts::PI * 2.0).sin() + 1.0) / 2.0
}

/// The 12 emitter colors of `mode` at animation time `t` (seconds, >= 0).
pub fn frame(mode: Mode, t: f64) -> Frame {
    let mut out = [(0u8, 0u8, 0u8); 12];
    match mode {
        Mode::Wheel => {
            for (i, px) in out.iter_mut().enumerate() {
                *px = hsv_to_rgb((emitter_angle(i) + t * 0.12) % 1.0, 1.0, 1.0);
            }
        }
        Mode::Knight => {
            let sweep_x = x_min() + (x_max() - x_min()) * ((t * 0.6).sin() + 1.0) / 2.0;
            for (i, px) in out.iter_mut().enumerate() {
                let dist = (EMITTER_POS[i][0] - sweep_x).abs();
                let v = (1.0 - dist * 1.6).max(0.0);
                *px = hsv_to_rgb(0.04 + v * 0.06, 1.0, v);
            }
        }
        Mode::Hue => {
            for (i, px) in out.iter_mut().enumerate() {
                *px = hsv_to_rgb((t * 0.35 + emitter_angle(i)) % 1.0, 1.0, 1.0);
            }
        }
        Mode::Chase => {
            let step = (t * 1.2).floor();
            let key = (step as i64).rem_euclid(4) as usize;
            let hue = (step * 0.25) % 1.0;
            let fl = pulse(t, 2.0);
            for (i, px) in out.iter_mut().enumerate() {
                if i / 3 == key {
                    *px = hsv_to_rgb(hue, 1.0, fl);
                }
            }
        }
        Mode::Breathe => {
            let v = pulse(t, 0.4);
            let hue = (t * 0.08) % 1.0;
            for px in out.iter_mut() {
                *px = hsv_to_rgb(hue, 1.0, v);
            }
        }
        Mode::Zone => {
            let hold = 1.0;
            let zi = ((t / hold).floor() as i64).rem_euclid(12) as usize;
            let ph = (t % hold) / hold;
            let v = (ph * std::f64::consts::PI).sin();
            let hue = (zi / 3) as f64 / 4.0 + (zi % 3) as f64 / 12.0;
            out[zi] = hsv_to_rgb(hue, 1.0, v);
        }
        Mode::Cascade => {
            for (i, px) in out.iter_mut().enumerate() {
                let nx = norm_x(i);
                let ph = (t / 5.0 - nx * 0.7) % 1.0;
                let v = ((ph * std::f64::consts::PI * 2.0).sin() + 1.0) / 2.0;
                *px = hsv_to_rgb((t * 0.07 + nx * 0.4) % 1.0, 1.0, v);
            }
        }
    }
    out
}

// ── Settings shared with the GUI ──────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// `None` = no animation (the static MagKey colors show).
    pub mode: Option<Mode>,
    pub speed: f64,
    /// Unix milliseconds at which `phase0` applied.
    pub epoch_ms: u64,
    pub phase0: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config { mode: None, speed: 1.0, epoch_ms: 0, phase0: 0.0 }
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl Config {
    /// Animation clock at `now_ms`.
    pub fn t_at(&self, now_ms: u64) -> f64 {
        let elapsed = now_ms.saturating_sub(self.epoch_ms) as f64 / 1000.0;
        (self.phase0 + elapsed * self.speed).max(0.0)
    }

    pub fn parse(text: &str) -> Config {
        let mut c = Config::default();
        for line in text.lines() {
            let Some((k, v)) = line.trim().split_once('=') else { continue };
            let v = v.trim();
            match k.trim() {
                "mode" => c.mode = Mode::parse(v),
                "speed" => {
                    if let Ok(s) = v.parse::<f64>() {
                        c.speed = s.clamp(MIN_SPEED, MAX_SPEED);
                    }
                }
                "epoch_ms" => c.epoch_ms = v.parse().unwrap_or(0),
                "phase0" => c.phase0 = v.parse().unwrap_or(0.0),
                _ => {}
            }
        }
        c
    }

    pub fn serialize(&self) -> String {
        format!(
            "# MagKey animation (edited by `ph18-lighting-daemon set-animation`)\nmode={}\nspeed={}\nepoch_ms={}\nphase0={}\n",
            self.mode.map(Mode::as_str).unwrap_or("none"),
            self.speed,
            self.epoch_ms,
            self.phase0
        )
    }

    pub fn load() -> Config {
        path().ok().and_then(|p| fs::read_to_string(p).ok()).map(|t| Config::parse(&t)).unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let p = path()?;
        if let Some(dir) = p.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = p.with_extension("tmp");
        fs::write(&tmp, self.serialize())?;
        fs::rename(&tmp, &p)
    }

    /// Apply a change from the CLI at `now_ms`: a new mode restarts the clock, a new speed keeps
    /// the animation continuous (the phase is re-based so nothing jumps).
    pub fn updated(&self, mode: Option<Option<Mode>>, speed: Option<f64>, now_ms: u64) -> Config {
        let mut next = *self;
        let current_t = self.t_at(now_ms);
        if let Some(new_mode) = mode {
            if new_mode != self.mode {
                next.mode = new_mode;
                next.phase0 = 0.0;
                next.epoch_ms = now_ms;
            }
        }
        if let Some(s) = speed {
            next.phase0 = if next.mode == self.mode { current_t } else { next.phase0 };
            next.epoch_ms = now_ms;
            next.speed = s.clamp(MIN_SPEED, MAX_SPEED);
        }
        next
    }
}

pub fn path() -> io::Result<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => PathBuf::from(
            std::env::var_os("HOME").ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?,
        )
        .join(".config"),
    };
    Ok(base.join("ph18-lighting").join("animation.conf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_name_round_trips() {
        for m in Mode::ALL {
            assert_eq!(Mode::parse(m.as_str()), Some(m));
        }
        assert_eq!(Mode::parse("none"), None);
        assert_eq!(Mode::parse("bogus"), None);
    }

    #[test]
    fn frames_are_defined_for_every_mode_and_time() {
        for m in Mode::ALL {
            for step in 0..400 {
                let f = frame(m, step as f64 * 0.137);
                assert_eq!(f.len(), 12);
            }
        }
    }

    #[test]
    fn wheel_at_zero_is_the_angle_rainbow() {
        let f = frame(Mode::Wheel, 0.0);
        // Emitters at different angles get different hues, all fully saturated and bright.
        assert_ne!(f[0], f[1]);
        assert!(f.iter().all(|&(r, g, b)| r.max(g).max(b) == 255));
    }

    #[test]
    fn chase_lights_one_key_at_a_time() {
        // At t = 0 the first key's three zones are lit (pulse(0) = 0.5), the others are off.
        let f = frame(Mode::Chase, 0.0);
        assert!(f[..3].iter().all(|&px| px != (0, 0, 0)));
        assert!(f[3..].iter().all(|&px| px == (0, 0, 0)));
    }

    #[test]
    fn zone_lights_exactly_one_emitter() {
        for step in 0..48 {
            let lit = frame(Mode::Zone, step as f64 * 0.25 + 0.1).iter().filter(|&&px| px != (0, 0, 0)).count();
            assert!(lit <= 1, "zone mode lit {lit} emitters");
        }
    }

    #[test]
    fn config_round_trips() {
        let c = Config { mode: Some(Mode::Cascade), speed: 2.5, epoch_ms: 1_700_000_000_123, phase0: 4.25 };
        assert_eq!(Config::parse(&c.serialize()), c);
        assert_eq!(Config::parse(""), Config::default());
        assert_eq!(Config::parse("mode=none").mode, None);
    }

    #[test]
    fn speed_is_clamped() {
        assert_eq!(Config::parse("speed=99").speed, MAX_SPEED);
        assert_eq!(Config::parse("speed=0").speed, MIN_SPEED);
    }

    #[test]
    fn clock_advances_with_speed() {
        let c = Config { mode: Some(Mode::Hue), speed: 2.0, epoch_ms: 10_000, phase0: 1.0 };
        assert!((c.t_at(10_000) - 1.0).abs() < 1e-9);
        assert!((c.t_at(13_000) - 7.0).abs() < 1e-9); // 1 + 3 s * 2
        assert!((c.t_at(5_000) - 1.0).abs() < 1e-9);  // before the epoch: does not go backwards
    }

    #[test]
    fn changing_speed_does_not_make_the_animation_jump() {
        let c = Config { mode: Some(Mode::Wheel), speed: 1.0, epoch_ms: 0, phase0: 0.0 };
        let now = 5_000;
        let before = c.t_at(now);
        let after = c.updated(None, Some(3.0), now);
        assert!((after.t_at(now) - before).abs() < 1e-9, "the clock jumped when the speed changed");
        assert!((after.t_at(now + 1000) - (before + 3.0)).abs() < 1e-9);
    }

    #[test]
    fn changing_mode_restarts_the_clock_and_same_mode_keeps_it() {
        let c = Config { mode: Some(Mode::Wheel), speed: 1.0, epoch_ms: 0, phase0: 0.0 };
        let restarted = c.updated(Some(Some(Mode::Knight)), None, 9_000);
        assert_eq!(restarted.mode, Some(Mode::Knight));
        assert!(restarted.t_at(9_000).abs() < 1e-9);
        let same = c.updated(Some(Some(Mode::Wheel)), None, 9_000);
        assert_eq!(same, c);
        assert_eq!(c.updated(Some(None), None, 9_000).mode, None);
    }
}
