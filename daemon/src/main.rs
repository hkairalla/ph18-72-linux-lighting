use std::collections::{BTreeMap, HashMap};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

const TARGET_HID_ID: &str = "0003:000005AF:0000866A";
const DARFON_HID_ID: &str = "0003:00000D62:0000BA51";
const REPORT_DESCRIPTOR_PREFIX: [u8; 3] = [0x06, 0x02, 0xff];
const PKT_PRELUDE: [[u8; 8]; 6] = [
    [0xb1, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4e],
    [0x08, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf5],
    [0x08, 0x02, 0x4f, 0x0a, 0x32, 0x00, 0x00, 0x6a],
    [0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xea],
    [0x13, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0xe4],
    [0x08, 0x02, 0x4f, 0x05, 0x32, 0x08, 0x01, 0x66],
];
const PKT_PRE_A: [u8; 8] = [0x88, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x77];
const PKT_PRE_B: [u8; 8] = [0x12, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0xe5];
const PKT_COMMIT33: [u8; 8] = [0x08, 0x02, 0x33, 0x05, 0x32, 0x08, 0x01, 0x82];
// Confirmed ff02 commit33 word encoding (2026-05-11 probe):
//   word = [0xff, R, G, B]
// Byte 0 = 0xff puts the controller in "broadcast" mode where the word
// reaches all 102 main-keyboard indices. Without it the write only lands on
// ~98 indices, leaving stragglers tinted by the previous baseline. Bytes
// 1/2/3 are conventional 8-bit R/G/B channels.
const MAGKEY_COMMIT_PACKET: [u8; 8] = [0x08, 0x02, 0x4f, 0x05, 0x32, 0x08, 0x01, 0x66];
const MAGKEY_KEYS: [&str; 4] = ["w", "a", "s", "d"];
const REPORT84_REPEAT: usize = 2;
const DARFON_OUTPUT_PAYLOAD_LEN: usize = 64;
const KEYBOARD_STATE_RELATIVE: &str = ".cache/ph18-lighting/keyboard-state";

#[derive(Debug, Parser)]
#[command(version, about = "Acer PH18-72 lighting control daemon")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print detected backend capabilities without changing hardware.
    Inventory,
    /// Emit the known-good full blue flow label.
    RestoreKnownGood,
    /// Set the main keyboard to the confirmed blue word/path.
    SetMainKeyboardBlue,
    /// Set the main keyboard to the observed red-ish test word/path.
    SetMainKeyboardRed,
    /// Set the main keyboard to the experimental green test word/path.
    SetMainKeyboardGreen,
    /// Set the keyboard baseline (whole-board anchor) and clear per-key overrides.
    SetKeyboardBaseline {
        /// One of: off, blue, red, green.
        #[arg(long)]
        color: String,
    },
    /// Add or update a per-key override and repaint the board.
    SetKeyboardKey {
        #[arg(long)]
        key: String,
        #[arg(long)]
        red: u8,
        #[arg(long)]
        green: u8,
        #[arg(long)]
        blue: u8,
    },
    /// Remove a per-key override and repaint the board.
    ClearKeyboardKey {
        #[arg(long)]
        key: String,
    },
    /// Clear all per-key overrides; keep the current baseline.
    ResetKeyboard,
    /// Re-emit the persisted state to hardware without changing it.
    /// Use this at login (via systemd user service) to restore the last
    /// applied colors after the firmware has reverted to dynamic mode.
    RepaintKeyboard,
    /// Run an ff02 commit33 sweep with an arbitrary 4-byte word.
    /// Used for research: only `off`/`blue`/`red`/`green` words are confirmed
    /// today. Does NOT touch the persisted keyboard state, so you can probe
    /// freely. Word format: hex pairs separated by `:` or just 8 hex chars.
    /// Example: --word ff:00:00:ff  (the confirmed blue word)
    ProbeKeyboardWord {
        #[arg(long)]
        word: String,
    },
    /// Print the current persisted keyboard state.
    GetKeyboardState,
    /// Set MagKeys using a confirmed safe command shape.
    SetMagkeys {
        #[arg(long)]
        all: Option<String>,
    },
    /// Set all four MagKeys with explicit per-key colors.
    SetMagkeysPattern {
        #[arg(long)]
        w: String,
        #[arg(long)]
        a: String,
        #[arg(long)]
        s: String,
        #[arg(long)]
        d: String,
        #[arg(long, default_value_t = false)]
        safe_magkeys: bool,
    },
    /// Set one MagKey through the ff02 LED-map path.
    SetMagkeyKey {
        #[arg(long)]
        key: String,
        #[arg(long)]
        red: u8,
        #[arg(long)]
        green: u8,
        #[arg(long)]
        blue: u8,
        #[arg(long, default_value_t = false)]
        safe_magkeys: bool,
    },
    /// Set one MagKey using a named whole-key preset across all three words.
    SetMagkeyWholeKey {
        #[arg(long)]
        key: String,
        #[arg(long)]
        color: String,
    },
    /// Set all four MagKeys using a named whole-key preset across all three words.
    SetMagkeysWhole {
        #[arg(long)]
        color: String,
    },
    /// Set one MagKey with independent per-zone RGB values (left/top/right).
    SetMagkeyZones {
        #[arg(long)]
        key: String,
        /// R,G,B for the left zone
        #[arg(long)]
        left: String,
        /// R,G,B for the top zone
        #[arg(long)]
        top: String,
        /// R,G,B for the right zone
        #[arg(long)]
        right: String,
    },
    /// Set the Darfon cover logo as a whole or one segment at a time.
    SetCoverLogo {
        #[arg(long)]
        segment: Option<String>,
        #[arg(long)]
        red: u8,
        #[arg(long)]
        green: u8,
        #[arg(long)]
        blue: u8,
        #[arg(long, default_value_t = false)]
        no_force_brightness: bool,
    },
    /// Set Darfon cover logo brightness 0-100.
    SetCoverLogoBrightness {
        #[arg(long)]
        level: u8,
    },
}

fn main() {
    let args = Args::parse();
    let result = match args.command {
        Command::Inventory => inventory(),
        Command::RestoreKnownGood => restore_known_good(),
        Command::SetMainKeyboardBlue => set_main_keyboard_blue(),
        Command::SetMainKeyboardRed => set_main_keyboard_red(),
        Command::SetMainKeyboardGreen => set_main_keyboard_green(),
        Command::SetKeyboardBaseline { color } => set_keyboard_baseline(&color),
        Command::SetKeyboardKey {
            key,
            red,
            green,
            blue,
        } => set_keyboard_key(&key, (red, green, blue)),
        Command::ClearKeyboardKey { key } => clear_keyboard_key(&key),
        Command::ResetKeyboard => reset_keyboard(),
        Command::RepaintKeyboard => repaint_keyboard_cmd(),
        Command::ProbeKeyboardWord { word } => probe_keyboard_word(&word),
        Command::GetKeyboardState => get_keyboard_state(),
        Command::SetMagkeys { all } => set_magkeys(all),
        Command::SetMagkeysPattern {
            w,
            a,
            s,
            d,
            safe_magkeys,
        } => set_magkeys_pattern(&w, &a, &s, &d, safe_magkeys),
        Command::SetMagkeyKey {
            key,
            red,
            green,
            blue,
            safe_magkeys,
        } => set_magkey_key(&key, (red, green, blue), safe_magkeys),
        Command::SetMagkeyWholeKey { key, color } => set_magkey_whole_key(&key, &color),
        Command::SetMagkeysWhole { color } => set_magkeys_whole(&color),
        Command::SetMagkeyZones { key, left, top, right } => (|| {
            let l = parse_rgb_csv(&left)?;
            let t = parse_rgb_csv(&top)?;
            let r = parse_rgb_csv(&right)?;
            set_magkey_zones(&key, l, t, r)
        })(),
        Command::SetCoverLogo {
            segment,
            red,
            green,
            blue,
            no_force_brightness,
        } => set_cover_logo(segment.as_deref(), (red, green, blue), !no_force_brightness),
        Command::SetCoverLogoBrightness { level } => set_cover_logo_brightness(level),
    };

    if let Err(err) = result {
        eprintln!("error={err}");
        std::process::exit(1);
    }
}

fn inventory() -> io::Result<()> {
    println!("ph18-lighting-daemon inventory");
    println!("hid.jingmold=05af:866a");
    println!("hid.darfon=0d62:ba51");
    println!("wmi=todo-read-only-triage");
    println!("surface.main_keyboard=functional");
    println!("surface.magkeys=functional");
    println!("surface.cover_logo=functional");
    println!("surface.base_logo=in-development");
    println!("surface.infinity_mirror=in-development");
    Ok(())
}

fn restore_known_good() -> io::Result<()> {
    println!("action=restore-known-good");
    println!("controller=05af:866a");
    println!("controller=0d62:ba51");
    println!("note=confirmed hybrid full-blue restore flow placeholder");
    Ok(())
}

// Keyboard baseline (whole-board ff02 anchor) handling.
//
// On this firmware, report84/report86 per-key writes only land if the board
// is already in a static frame. The ff02 commit33 sweep is the only known
// mode-transition out of dynamic.
//
// The ff02 word encoding is `[0xff, R, G, B]` (see the constant block at the
// top of this file). Any 24-bit RGB can be a baseline; named colors are just
// convenient aliases.

fn baseline_word(rgb: (u8, u8, u8)) -> [u8; 4] {
    [0xff, rgb.0, rgb.1, rgb.2]
}

fn parse_baseline_color(input: &str) -> io::Result<(u8, u8, u8)> {
    match normalize_name(input).as_str() {
        "off" => Ok((0, 0, 0)),
        "blue" => Ok((0, 0, 255)),
        "red" => Ok((255, 0, 0)),
        "green" => Ok((0, 255, 0)),
        _ => parse_rgb_csv(input).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "unknown baseline color '{input}'; expected off/blue/red/green or R,G,B"
                ),
            )
        }),
    }
}

fn baseline_name(rgb: (u8, u8, u8)) -> String {
    // Friendly display: render presets by name, custom colors as CSV.
    match rgb {
        (0, 0, 0) => "off".to_string(),
        (0, 0, 255) => "blue".to_string(),
        (255, 0, 0) => "red".to_string(),
        (0, 255, 0) => "green".to_string(),
        (r, g, b) => format!("{r},{g},{b}"),
    }
}

#[derive(Debug, Clone)]
struct KeyboardState {
    baseline_rgb: (u8, u8, u8),
    overrides: BTreeMap<u16, (u8, u8, u8)>,
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self {
            baseline_rgb: (0, 0, 255),
            overrides: BTreeMap::new(),
        }
    }
}

fn keyboard_state_path() -> io::Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "HOME env var not set; cannot locate state file")
    })?;
    Ok(PathBuf::from(home).join(KEYBOARD_STATE_RELATIVE))
}

fn load_keyboard_state() -> KeyboardState {
    // Best-effort load: any IO or parse error falls back to defaults so the
    // daemon never refuses to operate because of a malformed cache file.
    let Ok(path) = keyboard_state_path() else {
        return KeyboardState::default();
    };
    let Ok(contents) = fs::read_to_string(&path) else {
        return KeyboardState::default();
    };

    let mut state = KeyboardState::default();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(value) = line.strip_prefix("baseline=") {
            // Accepts either a named preset (legacy state files) or R,G,B.
            if let Ok(rgb) = parse_baseline_color(value) {
                state.baseline_rgb = rgb;
            }
        } else if let Some(rest) = line.strip_prefix("override=") {
            // Format: override=<index>:<r>,<g>,<b>
            if let Some((index_str, rgb_str)) = rest.split_once(':') {
                if let (Ok(index), Ok(rgb)) = (index_str.parse::<u16>(), parse_rgb_csv(rgb_str)) {
                    state.overrides.insert(index, rgb);
                }
            }
        }
    }
    state
}

fn save_keyboard_state(state: &KeyboardState) -> io::Result<()> {
    let path = keyboard_state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut buf = String::new();
    let (br, bg, bb) = state.baseline_rgb;
    buf.push_str(&format!("baseline={br},{bg},{bb}\n"));
    for (index, &(r, g, b)) in &state.overrides {
        buf.push_str(&format!("override={index}:{r},{g},{b}\n"));
    }
    // Write to a sibling tmp file then rename so a partial write can't corrupt state.
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, buf)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

fn run_ff02_anchor(word: [u8; 4]) -> io::Result<PathBuf> {
    let node = find_ff02_node()?;
    let frame = keyboard_frame(word);

    for packet in PKT_PRELUDE {
        send_feature_ff02(&node, &packet)?;
    }
    for _ in 0..20 {
        send_feature_ff02(&node, &PKT_PRE_A)?;
        send_feature_ff02(&node, &PKT_PRE_B)?;
        for _ in 0..8 {
            send_out64(&node, &frame)?;
        }
        send_feature_ff02(&node, &PKT_COMMIT33)?;
    }
    Ok(node)
}

fn paint_index_via_report84(
    node: &Path,
    index: u16,
    color: (u8, u8, u8),
) -> io::Result<()> {
    let report84 = build_report84_single_index(index, color.0, color.1, color.2, 1, 8);
    // Never send [0x86, 0x00] between report84 and [0x86, 0x01]: the blackout
    // discards the pending per-key buffer and the next [0x86, 0x01] is read
    // as "return to default dynamic pattern" instead of "commit".
    for _ in 0..REPORT84_REPEAT {
        send_feature_report(node, &report84)?;
        send_feature_report(node, &[0x86, 0x01])?;
    }
    Ok(())
}

fn repaint_keyboard(state: &KeyboardState) -> io::Result<(PathBuf, PathBuf)> {
    // The ff02 anchor now uses byte 0 = 0xff broadcast mode (see
    // `baseline_word`), which reaches all 102 main-keyboard indices
    // uniformly. Previous code patched a hardcoded list of "stubborn"
    // indices afterwards; that was a workaround for the incomplete legacy
    // words and is no longer needed.
    let ff02 = run_ff02_anchor(baseline_word(state.baseline_rgb))?;
    let vendor = find_vendor_keyboard_node()?;

    for (&index, &rgb) in &state.overrides {
        paint_index_via_report84(&vendor, index, rgb)?;
    }
    Ok((ff02, vendor))
}

fn set_main_keyboard_blue() -> io::Result<()> {
    apply_baseline((0, 0, 255), "set-main-keyboard-blue")
}

fn set_main_keyboard_red() -> io::Result<()> {
    apply_baseline((255, 0, 0), "set-main-keyboard-red")
}

fn set_main_keyboard_green() -> io::Result<()> {
    apply_baseline((0, 255, 0), "set-main-keyboard-green")
}

fn set_keyboard_baseline(color: &str) -> io::Result<()> {
    let rgb = parse_baseline_color(color)?;
    apply_baseline(rgb, "set-keyboard-baseline")
}

fn apply_baseline(rgb: (u8, u8, u8), action: &str) -> io::Result<()> {
    let mut state = load_keyboard_state();
    state.baseline_rgb = rgb;
    state.overrides.clear();
    let (ff02, vendor) = repaint_keyboard(&state)?;
    save_keyboard_state(&state)?;

    println!("action={action}");
    println!("controller=05af:866a");
    println!("path=ff02_commit33");
    println!("baseline={}", baseline_name(rgb));
    println!("baseline_rgb={},{},{}", rgb.0, rgb.1, rgb.2);
    println!("baseline_word={}", hex_string(&baseline_word(rgb)));
    println!("ff02_hidraw={}", ff02.display());
    println!("vendor_hidraw={}", vendor.display());
    println!("overrides=0");
    println!("result=sent");
    Ok(())
}

fn set_keyboard_key(key: &str, color: (u8, u8, u8)) -> io::Result<()> {
    let index = keyboard_key_index(key).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown keyboard key {key}"),
        )
    })?;

    let mut state = load_keyboard_state();
    state.overrides.insert(index, color);

    // Fast path: assume firmware is already in static mode (a baseline anchor
    // has been applied at some point this session). Write the single key via
    // report84/report86=1 — no whole-board ff02 anchor, no MagKey-channel
    // side effects, no visible flicker on other keys. If the firmware has
    // drifted back to dynamic, this write is silently absorbed; recovery is
    // `set-keyboard-baseline` or `repaint-keyboard`.
    let vendor = find_vendor_keyboard_node()?;
    paint_index_via_report84(&vendor, index, color)?;
    save_keyboard_state(&state)?;

    println!("action=set-keyboard-key");
    println!("controller=05af:866a");
    println!("path=report84_report86");
    println!("key={}", normalize_name(key));
    println!("index={index}");
    println!("rgb={},{},{}", color.0, color.1, color.2);
    println!("baseline={}", baseline_name(state.baseline_rgb));
    println!("overrides={}", state.overrides.len());
    println!("vendor_hidraw={}", vendor.display());
    println!("result=sent");
    Ok(())
}

fn clear_keyboard_key(key: &str) -> io::Result<()> {
    let index = keyboard_key_index(key).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown keyboard key {key}"),
        )
    })?;
    let mut state = load_keyboard_state();
    let removed = state.overrides.remove(&index).is_some();
    let baseline_rgb = state.baseline_rgb;

    // Fast path: paint the cleared key with the baseline color via report84,
    // matching the visual result of a full repaint without the cost.
    let vendor = find_vendor_keyboard_node()?;
    paint_index_via_report84(&vendor, index, baseline_rgb)?;
    save_keyboard_state(&state)?;

    println!("action=clear-keyboard-key");
    println!("controller=05af:866a");
    println!("path=report84_report86");
    println!("key={}", normalize_name(key));
    println!("index={index}");
    println!("removed={removed}");
    println!("baseline={}", baseline_name(state.baseline_rgb));
    println!("baseline_rgb={},{},{}", baseline_rgb.0, baseline_rgb.1, baseline_rgb.2);
    println!("overrides={}", state.overrides.len());
    println!("vendor_hidraw={}", vendor.display());
    println!("result=sent");
    Ok(())
}

fn reset_keyboard() -> io::Result<()> {
    let mut state = load_keyboard_state();
    let cleared = state.overrides.len();
    state.overrides.clear();
    let (ff02, vendor) = repaint_keyboard(&state)?;
    save_keyboard_state(&state)?;

    println!("action=reset-keyboard");
    println!("controller=05af:866a");
    println!("baseline={}", baseline_name(state.baseline_rgb));
    println!("cleared_overrides={cleared}");
    println!("ff02_hidraw={}", ff02.display());
    println!("vendor_hidraw={}", vendor.display());
    println!("result=sent");
    Ok(())
}

fn probe_keyboard_word(word_arg: &str) -> io::Result<()> {
    let word = parse_word_hex(word_arg)?;
    let ff02 = run_ff02_anchor(word)?;

    println!("action=probe-keyboard-word");
    println!("controller=05af:866a");
    println!("path=ff02_commit33");
    println!("word={}", hex_string(&word));
    println!("ff02_hidraw={}", ff02.display());
    if word[0] == 0xff {
        println!(
            "decoded=broadcast R={} G={} B={} (all 102 keys)",
            word[1], word[2], word[3]
        );
    } else {
        println!("decoded=legacy mode (byte 0 != 0xff); partial coverage on this firmware");
    }
    println!("note=probe does not touch persistent keyboard state");
    println!("result=sent");
    Ok(())
}

fn parse_word_hex(value: &str) -> io::Result<[u8; 4]> {
    let cleaned: String = value
        .chars()
        .filter(|c| !matches!(c, ':' | '-' | ' '))
        .collect();
    if cleaned.len() != 8 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("expected 8 hex chars (optionally separated by ':'), got '{value}'"),
        ));
    }
    let mut out = [0_u8; 4];
    for i in 0..4 {
        let byte = u8::from_str_radix(&cleaned[i * 2..i * 2 + 2], 16).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid hex byte in word '{value}'"),
            )
        })?;
        out[i] = byte;
    }
    Ok(out)
}

fn repaint_keyboard_cmd() -> io::Result<()> {
    let state = load_keyboard_state();
    let (ff02, vendor) = repaint_keyboard(&state)?;

    println!("action=repaint-keyboard");
    println!("controller=05af:866a");
    println!("baseline={}", baseline_name(state.baseline_rgb));
    println!("overrides={}", state.overrides.len());
    println!("ff02_hidraw={}", ff02.display());
    println!("vendor_hidraw={}", vendor.display());
    println!("result=sent");
    Ok(())
}

fn get_keyboard_state() -> io::Result<()> {
    let state = load_keyboard_state();
    let path = keyboard_state_path()?;
    println!("action=get-keyboard-state");
    println!("state_path={}", path.display());
    println!("baseline={}", baseline_name(state.baseline_rgb));
    println!("overrides={}", state.overrides.len());
    for (index, &(r, g, b)) in &state.overrides {
        println!("override={index}:{r},{g},{b}");
    }
    Ok(())
}

fn set_magkeys(all: Option<String>) -> io::Result<()> {
    let color = parse_rgb_csv(all.as_deref().unwrap_or("0,0,255"))?;
    let entries = MAGKEY_KEYS.map(|key| (key, color));
    let (node, payload) = apply_magkey_entries(&entries)?;

    println!("action=set-magkeys");
    println!("controller=05af:866a");
    println!("path=ff02_ledmap_commit");
    println!("hidraw={}", node.display());
    println!("rgb={},{},{}", color.0, color.1, color.2);
    println!("commit={}", hex_string(&MAGKEY_COMMIT_PACKET));
    println!("payload={}", hex_string(&payload));
    println!("result=sent");
    Ok(())
}

fn set_magkeys_pattern(w: &str, a: &str, s: &str, d: &str, safe_magkeys: bool) -> io::Result<()> {
    if safe_magkeys {
        eprintln!("note: --safe-magkeys is no longer needed; the corrected frame model handles blue routing correctly");
    }
    let entries = [
        ("w", parse_rgb_csv(w)?),
        ("a", parse_rgb_csv(a)?),
        ("s", parse_rgb_csv(s)?),
        ("d", parse_rgb_csv(d)?),
    ];
    let (node, payload) = apply_magkey_entries(&entries)?;

    println!("action=set-magkeys-pattern");
    println!("controller=05af:866a");
    println!("path=ff02_ledmap_commit");
    println!("hidraw={}", node.display());
    println!(
        "w={},{},{}",
        entries[0].1 .0, entries[0].1 .1, entries[0].1 .2
    );
    println!(
        "a={},{},{}",
        entries[1].1 .0, entries[1].1 .1, entries[1].1 .2
    );
    println!(
        "s={},{},{}",
        entries[2].1 .0, entries[2].1 .1, entries[2].1 .2
    );
    println!(
        "d={},{},{}",
        entries[3].1 .0, entries[3].1 .1, entries[3].1 .2
    );
    println!("commit={}", hex_string(&MAGKEY_COMMIT_PACKET));
    println!("payload={}", hex_string(&payload));
    if safe_magkeys {
        println!("mode=safe-magkeys");
    }
    println!("result=sent");
    Ok(())
}

fn set_magkey_key(key: &str, color: (u8, u8, u8), safe_magkeys: bool) -> io::Result<()> {
    if safe_magkeys {
        eprintln!("note: --safe-magkeys is no longer needed; the corrected frame model handles blue routing correctly");
    }
    let normalized = normalize_name(key);
    if !MAGKEY_KEYS.contains(&normalized.as_str()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown magkey {key}"),
        ));
    }

    let entries = [(normalized.as_str(), color)];
    let (node, payload) = apply_magkey_entries(&entries)?;

    println!("action=set-magkey-key");
    println!("controller=05af:866a");
    println!("path=ff02_ledmap_commit");
    println!("hidraw={}", node.display());
    println!("key={normalized}");
    println!("rgb={},{},{}", color.0, color.1, color.2);
    println!("commit={}", hex_string(&MAGKEY_COMMIT_PACKET));
    println!("payload={}", hex_string(&payload));
    if safe_magkeys {
        println!("mode=safe-magkeys");
    }
    println!("result=sent");
    Ok(())
}

fn set_magkey_whole_key(key: &str, color: &str) -> io::Result<()> {
    let normalized = normalize_name(key);
    if !MAGKEY_KEYS.contains(&normalized.as_str()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown magkey {key}"),
        ));
    }
    let rgb = magkey_named_color(color)?;
    let base = magkey_slot(&normalized);
    let mut emitters = [(0u8, 0u8, 0u8); 12];
    emitters[base]     = rgb;
    emitters[base + 1] = rgb;
    emitters[base + 2] = rgb;
    let frame = build_magkey_frame(&emitters);
    let (node, payload) = apply_magkey_frame(&frame)?;

    println!("action=set-magkey-whole-key");
    println!("controller=05af:866a");
    println!("path=ff02_ledmap_commit");
    println!("hidraw={}", node.display());
    println!("key={normalized}");
    println!("color={}", normalize_name(color));
    println!("rgb={},{},{}", rgb.0, rgb.1, rgb.2);
    println!("commit={}", hex_string(&MAGKEY_COMMIT_PACKET));
    println!("payload={}", hex_string(&payload));
    println!("result=sent");
    Ok(())
}

fn set_magkeys_whole(color: &str) -> io::Result<()> {
    let rgb = magkey_named_color(color)?;
    let emitters = [rgb; 12];
    let frame = build_magkey_frame(&emitters);
    let (node, payload) = apply_magkey_frame(&frame)?;

    println!("action=set-magkeys-whole");
    println!("controller=05af:866a");
    println!("path=ff02_ledmap_commit");
    println!("hidraw={}", node.display());
    println!("color={}", normalize_name(color));
    println!("rgb={},{},{}", rgb.0, rgb.1, rgb.2);
    println!("commit={}", hex_string(&MAGKEY_COMMIT_PACKET));
    println!("payload={}", hex_string(&payload));
    println!("result=sent");
    Ok(())
}

fn set_magkey_zones(
    key: &str,
    left: (u8, u8, u8),
    top: (u8, u8, u8),
    right: (u8, u8, u8),
) -> io::Result<()> {
    let normalized = normalize_name(key);
    if !MAGKEY_KEYS.contains(&normalized.as_str()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown magkey {key}"),
        ));
    }
    let base = magkey_slot(&normalized);
    let mut emitters = [(0u8, 0u8, 0u8); 12];
    emitters[base]     = left;
    emitters[base + 1] = top;
    emitters[base + 2] = right;
    let frame = build_magkey_frame(&emitters);
    let (node, payload) = apply_magkey_frame(&frame)?;

    println!("action=set-magkey-zones");
    println!("controller=05af:866a");
    println!("path=ff02_ledmap_commit");
    println!("hidraw={}", node.display());
    println!("key={normalized}");
    println!("left={},{},{}", left.0, left.1, left.2);
    println!("top={},{},{}", top.0, top.1, top.2);
    println!("right={},{},{}", right.0, right.1, right.2);
    println!("commit={}", hex_string(&MAGKEY_COMMIT_PACKET));
    println!("payload={}", hex_string(&payload));
    println!("result=sent");
    Ok(())
}

fn apply_magkey_entries(entries: &[(&str, (u8, u8, u8))]) -> io::Result<(PathBuf, [u8; 64])> {
    apply_magkey_frame(&build_magkey_payload(entries))
}

fn apply_magkey_frame(payload: &[u8; 64]) -> io::Result<(PathBuf, [u8; 64])> {
    let node = find_ff02_node()?;
    for packet in PKT_PRELUDE {
        send_feature_ff02(&node, &packet)?;
    }
    send_out64(&node, payload)?;
    send_feature_ff02(&node, &MAGKEY_COMMIT_PACKET)?;
    Ok((node, *payload))
}

fn set_cover_logo(segment: Option<&str>, color: (u8, u8, u8), force_brightness: bool) -> io::Result<()> {
    let node = find_darfon_node()?;
    let mut results: Vec<(String, String)> = Vec::new();

    println!("action=set-cover-logo");
    println!("controller=0d62:ba51");
    println!("path=darfon_short_packets");
    println!("hidraw={}", node.display());
    println!("rgb={},{},{}", color.0, color.1, color.2);
    println!("force_brightness={}", if force_brightness { "true" } else { "false" });

    if force_brightness {
        let payload = darfon_brightness_packet(100);
        println!("brightness_packet={}", hex_string(&payload));
        results.extend(attempt_darfon_transports(&node, &payload));
    }

    match segment {
        Some(name) => {
            let segment_id = darfon_segment_id(name)?;
            let payload = darfon_color_packet(segment_id, color.0, color.1, color.2);
            println!("segment={}", normalize_name(name));
            println!("color_packet={}", hex_string(&payload));
            results.extend(attempt_darfon_transports(&node, &payload));
        }
        None => {
            println!("segment=all");
            for segment_id in 1..=3 {
                let payload = darfon_color_packet(segment_id, color.0, color.1, color.2);
                println!("color_packet={}", hex_string(&payload));
                results.extend(attempt_darfon_transports(&node, &payload));
            }
        }
    }

    for (method, outcome) in results {
        println!("{method}={outcome}");
    }
    println!("result=sent");
    Ok(())
}

fn set_cover_logo_brightness(level: u8) -> io::Result<()> {
    let node = find_darfon_node()?;
    let payload = darfon_brightness_packet(level);

    println!("action=set-cover-logo-brightness");
    println!("controller=0d62:ba51");
    println!("path=darfon_short_packets");
    println!("hidraw={}", node.display());
    println!("level={}", level.min(100));
    println!("brightness_packet={}", hex_string(&payload));

    for (method, outcome) in attempt_darfon_transports(&node, &payload) {
        println!("{method}={outcome}");
    }
    println!("result=sent");
    Ok(())
}

fn find_ff02_node() -> io::Result<PathBuf> {
    for entry in fs::read_dir("/sys/class/hidraw")? {
        let entry = entry?;
        let hidraw_name = entry.file_name();
        let Some(name) = hidraw_name.to_str() else {
            continue;
        };
        if !name.starts_with("hidraw") {
            continue;
        }

        let device_dir = entry.path().join("device");
        let uevent_path = device_dir.join("uevent");
        if !uevent_path.exists() {
            continue;
        }

        let fields = parse_key_value_file(&uevent_path)?;
        if fields.get("HID_ID").map(String::as_str) != Some(TARGET_HID_ID) {
            continue;
        }

        let descriptor = fs::read(device_dir.join("report_descriptor"))?;
        if descriptor.starts_with(&REPORT_DESCRIPTOR_PREFIX) {
            return Ok(Path::new("/dev").join(name));
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "ff02 hidraw node not found for 05af:866a",
    ))
}

fn find_vendor_keyboard_node() -> io::Result<PathBuf> {
    for entry in fs::read_dir("/sys/class/hidraw")? {
        let entry = entry?;
        let hidraw_name = entry.file_name();
        let Some(name) = hidraw_name.to_str() else {
            continue;
        };
        if !name.starts_with("hidraw") {
            continue;
        }

        let device_dir = entry.path().join("device");
        let uevent_path = device_dir.join("uevent");
        if !uevent_path.exists() {
            continue;
        }

        let fields = parse_key_value_file(&uevent_path)?;
        if fields.get("HID_ID").map(String::as_str) != Some(TARGET_HID_ID) {
            continue;
        }

        let descriptor = fs::read(device_dir.join("report_descriptor"))?;
        if descriptor.windows(2).any(|window| window == [0x85, 0x82])
            && descriptor.windows(2).any(|window| window == [0x85, 0x83])
        {
            return Ok(Path::new("/dev").join(name));
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "vendor keyboard HID node not found for report84/report86",
    ))
}

fn find_darfon_node() -> io::Result<PathBuf> {
    for entry in fs::read_dir("/sys/class/hidraw")? {
        let entry = entry?;
        let hidraw_name = entry.file_name();
        let Some(name) = hidraw_name.to_str() else {
            continue;
        };
        if !name.starts_with("hidraw") {
            continue;
        }

        let device_dir = entry.path().join("device");
        let uevent_path = device_dir.join("uevent");
        if !uevent_path.exists() {
            continue;
        }

        let fields = parse_key_value_file(&uevent_path)?;
        if fields.get("HID_ID").map(String::as_str) != Some(DARFON_HID_ID) {
            continue;
        }

        let descriptor = fs::read(device_dir.join("report_descriptor"))?;
        let has_feature_8 = descriptor.windows(4).any(|window| window == [0x95, 0x08, 0xb1, 0x02]);
        let has_output_64 = descriptor.windows(4).any(|window| window == [0x75, 0x08, 0x95, 0x40])
            && descriptor.windows(4).any(|window| window == [0x09, 0x21, 0x91, 0x02]);
        if has_feature_8 && has_output_64 {
            return Ok(Path::new("/dev").join(name));
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Darfon cover-logo HID node not found for 0d62:ba51",
    ))
}

fn parse_key_value_file(path: &Path) -> io::Result<HashMap<String, String>> {
    let mut fields = HashMap::new();
    let contents = fs::read_to_string(path)?;
    for line in contents.lines() {
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(key.to_owned(), value.to_owned());
        }
    }
    Ok(fields)
}

fn keyboard_frame(word: [u8; 4]) -> [u8; 64] {
    let mut frame = [0_u8; 64];
    for chunk in frame.chunks_exact_mut(4) {
        chunk.copy_from_slice(&word);
    }
    frame
}

/// Build a 64-byte MagKey payload from per-emitter RGB values.
///
/// Emitter numbering:
///    0=W-left   1=W-top   2=W-right
///    3=A-left   4=A-top   5=A-right
///    6=S-left   7=S-top   8=S-right
///    9=D-left  10=D-top  11=D-right
///
/// Verified word model (2026-04-26):
///   frame[N*4+2] = red   for emitter N
///   frame[N*4+3] = green for emitter N
///   frame[(N+1)*4] = blue for emitter N  (routed via the next word's byte0)
fn build_magkey_frame(emitters: &[(u8, u8, u8); 12]) -> [u8; 64] {
    let mut frame = [0u8; 64];
    for (i, &(r, g, b)) in emitters.iter().enumerate() {
        frame[i * 4 + 2] = r;
        frame[i * 4 + 3] = g;
        frame[(i + 1) * 4] = b;
    }
    frame
}

fn build_magkey_payload(entries: &[(&str, (u8, u8, u8))]) -> [u8; 64] {
    let mut emitters = [(0u8, 0u8, 0u8); 12];
    for &(key, color) in entries {
        let base = magkey_slot(key);
        emitters[base]     = color;
        emitters[base + 1] = color;
        emitters[base + 2] = color;
    }
    build_magkey_frame(&emitters)
}

fn magkey_named_color(color: &str) -> io::Result<(u8, u8, u8)> {
    match normalize_name(color).as_str() {
        "red"   => Ok((255, 0, 0)),
        "green" => Ok((0, 255, 0)),
        "blue"  => Ok((0, 0, 255)),
        "off"   => Ok((0, 0, 0)),
        other   => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown color preset '{other}'; expected red, green, blue, or off"),
        )),
    }
}

fn magkey_slot(key: &str) -> usize {
    match key {
        "w" => 0,
        "a" => 3,
        "s" => 6,
        "d" => 9,
        _ => unreachable!("unexpected magkey"),
    }
}

fn build_report84_single_index(
    index: u16,
    red: u8,
    green: u8,
    blue: u8,
    mode_selector: u8,
    brightness_level: u8,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(51);
    payload.extend_from_slice(&[0x84, mode_selector, brightness_level.min(8)]);
    for _ in 0..8 {
        payload.extend_from_slice(&index.to_le_bytes());
    }
    for _ in 0..8 {
        payload.extend_from_slice(&[red, green, blue, 0x00]);
    }
    payload
}

// Retained as protocol documentation + test coverage even though no
// production code path currently uses it (see repaint_keyboard).
#[allow(dead_code)]
fn build_report82_color(red: u8, green: u8, blue: u8, alpha: u8, mode_index: u8) -> Vec<u8> {
    let counter = u16::from(mode_index) * 16710 + 9415;
    let mut body = vec![0_u8; 28];
    body[0] = mode_index;
    body[2] = (counter & 0xff) as u8;
    body[3] = (counter >> 8) as u8;
    body[6] = 0x1e;
    body[7] = 0x14;
    body[14] = 0x88;
    body[15] = 0x13;
    body[18] = 0x01;
    body[22] = red;
    body[23] = green;
    body[24] = blue;
    body[25] = alpha;
    body[26] = 0x01;
    body[27] = 0x39u8.saturating_add(mode_index);

    let mut payload = Vec::with_capacity(29);
    payload.push(0x82);
    payload.extend_from_slice(&body);
    payload
}

fn darfon_segment_id(name: &str) -> io::Result<u8> {
    match normalize_name(name).as_str() {
        "left" => Ok(1),
        "middle" => Ok(2),
        "right" => Ok(3),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown Darfon segment {other}"),
        )),
    }
}

fn darfon_color_packet(segment: u8, red: u8, green: u8, blue: u8) -> [u8; 8] {
    let tail = 0xe8u8.saturating_sub(segment);
    [0x14, 0x01, segment, red, green, blue, 0x03, tail]
}

fn darfon_brightness_packet(level: u8) -> [u8; 8] {
    let level = level.min(100);
    let tail = (0xefi32 - (((level as i32) * 25) / 4)) as u8;
    [0x08, 0x01, 0x01, 0x05, level, 0x01, 0x00, tail]
}

fn attempt_darfon_transports(node: &Path, payload: &[u8]) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let methods: [(&str, fn(&Path, &[u8]) -> io::Result<()>); 4] = [
        ("feature_prefixed", darfon_feature_prefixed),
        ("feature_raw", darfon_feature_raw),
        ("output_prefixed", darfon_output_prefixed),
        ("output_raw", darfon_output_raw),
    ];

    for (name, func) in methods {
        let outcome = match func(node, payload) {
            Ok(()) => "ok".to_string(),
            Err(err) => err.to_string(),
        };
        results.push((name.to_string(), outcome));
    }

    results
}

fn send_feature_ff02(node: &Path, payload: &[u8]) -> io::Result<()> {
    let mut buffer = Vec::with_capacity(payload.len() + 1);
    buffer.push(0x00);
    buffer.extend_from_slice(payload);
    ioctl_feature(node, &mut buffer)
}

fn darfon_feature_prefixed(node: &Path, payload: &[u8]) -> io::Result<()> {
    let mut buffer = Vec::with_capacity(payload.len() + 1);
    buffer.push(0x00);
    buffer.extend_from_slice(payload);
    ioctl_feature(node, &mut buffer)
}

fn darfon_feature_raw(node: &Path, payload: &[u8]) -> io::Result<()> {
    let mut buffer = payload.to_vec();
    ioctl_feature(node, &mut buffer)
}

fn darfon_output_prefixed(node: &Path, payload: &[u8]) -> io::Result<()> {
    let mut packet = Vec::with_capacity(DARFON_OUTPUT_PAYLOAD_LEN + 1);
    packet.push(0x00);
    packet.extend_from_slice(payload);
    packet.resize(DARFON_OUTPUT_PAYLOAD_LEN + 1, 0x00);
    send_raw_output(node, &packet)
}

fn darfon_output_raw(node: &Path, payload: &[u8]) -> io::Result<()> {
    let mut packet = payload.to_vec();
    packet.resize(DARFON_OUTPUT_PAYLOAD_LEN, 0x00);
    send_raw_output(node, &packet)
}

fn send_feature_report(node: &Path, payload: &[u8]) -> io::Result<()> {
    let mut buffer = payload.to_vec();
    ioctl_feature(node, &mut buffer)
}

fn ioctl_feature(node: &Path, buffer: &mut [u8]) -> io::Result<()> {
    let file = OpenOptions::new().read(true).write(true).open(node)?;
    let result = unsafe { libc::ioctl(file.as_raw_fd(), hidiocsfeature(buffer.len()), buffer.as_mut_ptr()) };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn send_out64(node: &Path, payload: &[u8]) -> io::Result<()> {
    send_raw_output(node, payload)
}

fn send_raw_output(node: &Path, payload: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).open(node)?;
    file.write_all(payload)?;
    file.flush()?;
    Ok(())
}

fn parse_rgb_csv(value: &str) -> io::Result<(u8, u8, u8)> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("expected R,G,B, got {value}"),
        ));
    }

    Ok((
        parse_rgb_component(parts[0], value)?,
        parse_rgb_component(parts[1], value)?,
        parse_rgb_component(parts[2], value)?,
    ))
}

fn parse_rgb_component(component: &str, original: &str) -> io::Result<u8> {
    component.trim().parse::<u8>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid RGB component in {original}"),
        )
    })
}

fn keyboard_key_index(name: &str) -> Option<u16> {
    match normalize_name(name).as_str() {
        "esc" => Some(0),
        "f1" => Some(1),
        "f2" => Some(2),
        "f3" => Some(3),
        "f4" => Some(4),
        "f5" => Some(5),
        "f6" => Some(6),
        "f7" => Some(7),
        "f8" => Some(8),
        "f9" => Some(9),
        "f10" => Some(10),
        "f11" => Some(11),
        "f12" => Some(12),
        "print_screen" | "prtsc" | "prt_sc" => Some(13),
        "insert" | "ins" => Some(14),
        "delete" | "del" => Some(15),
        "media_prev" | "prev_track" => Some(16),
        "media_play_pause" | "play_pause" | "pause_play" => Some(17),
        "media_next" | "next_track" => Some(18),
        "power" => Some(19),
        "grave" | "backtick" | "tilde" => Some(20),
        "1" | "digit_1" => Some(21),
        "2" | "digit_2" => Some(22),
        "3" | "digit_3" => Some(23),
        "4" | "digit_4" => Some(24),
        "5" | "digit_5" => Some(25),
        "6" | "digit_6" => Some(26),
        "7" | "digit_7" => Some(27),
        "8" | "digit_8" => Some(28),
        "9" | "digit_9" => Some(29),
        "0" | "digit_0" => Some(30),
        "minus" => Some(31),
        "equal" | "equals" => Some(32),
        "backspace" => Some(33),
        "predator_sense" => Some(34),
        "keypad_num_lock" | "num_lock" => Some(35),
        "keypad_divide" | "kp_divide" => Some(36),
        "keypad_multiply" | "kp_multiply" => Some(37),
        "tab" => Some(38),
        "q" => Some(39),
        "e" => Some(41),
        "r" => Some(42),
        "t" => Some(43),
        "y" => Some(44),
        "u" => Some(45),
        "i" => Some(46),
        "o" => Some(47),
        "p" => Some(48),
        "left_bracket" | "lbracket" => Some(49),
        "right_bracket" | "rbracket" => Some(50),
        "backslash" => Some(51),
        "keypad_7" => Some(52),
        "keypad_8" => Some(53),
        "keypad_9" => Some(54),
        "keypad_minus" | "kp_minus" => Some(55),
        "caps_lock" => Some(56),
        "f" => Some(60),
        "g" => Some(61),
        "h" => Some(62),
        "j" => Some(63),
        "k" => Some(64),
        "l" => Some(65),
        "semicolon" => Some(66),
        "apostrophe" | "quote" => Some(67),
        "enter" => Some(68),
        "keypad_4" => Some(69),
        "keypad_5" => Some(70),
        "keypad_6" => Some(71),
        "keypad_plus" | "kp_plus" => Some(72),
        "left_shift" | "lshift" => Some(73),
        "z" => Some(74),
        "x" => Some(75),
        "c" => Some(76),
        "v" => Some(77),
        "b" => Some(78),
        "n" => Some(79),
        "m" => Some(80),
        "comma" => Some(81),
        "period" => Some(82),
        "slash" => Some(83),
        "right_shift" | "rshift" => Some(84),
        "arrow_up" | "up" => Some(85),
        "keypad_1" => Some(86),
        "keypad_2" => Some(87),
        "keypad_3" => Some(88),
        "left_ctrl" | "lctrl" => Some(89),
        "fn" => Some(90),
        "left_windows" | "lwin" | "win" | "windows" => Some(91),
        "left_alt" => Some(92),
        "space" => Some(93),
        "right_alt" | "altgr" => Some(94),
        "menu" => Some(95),
        "copilot" => Some(96),
        "arrow_left" | "left" => Some(97),
        "arrow_down" | "down" => Some(98),
        "arrow_right" | "right" => Some(99),
        "keypad_0" => Some(100),
        "keypad_decimal" | "kp_decimal" => Some(101),
        "keypad_enter" | "kp_enter" => Some(102),
        _ => None,
    }
}

fn normalize_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .replace('-', "_")
        .replace(' ', "_")
}

fn hidiocsfeature(length: usize) -> libc::c_ulong {
    const IOC_NRBITS: u32 = 8;
    const IOC_TYPEBITS: u32 = 8;
    const IOC_SIZEBITS: u32 = 14;
    const IOC_NRSHIFT: u32 = 0;
    const IOC_TYPESHIFT: u32 = IOC_NRSHIFT + IOC_NRBITS;
    const IOC_SIZESHIFT: u32 = IOC_TYPESHIFT + IOC_TYPEBITS;
    const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;
    const IOC_READ: u32 = 2;
    const IOC_WRITE: u32 = 1;

    (((IOC_READ | IOC_WRITE) << IOC_DIRSHIFT)
        | ((b'H' as u32) << IOC_TYPESHIFT)
        | (0x06_u32 << IOC_NRSHIFT)
        | ((length as u32) << IOC_SIZESHIFT)) as libc::c_ulong
}

fn hex_string(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report84_layout_blue_q() {
        // Q = index 39 (0x27), blue, mode=1, brightness=8.
        // Layout: [0x84, mode, brightness, index*8 (LE u16), (R,G,B,0)*8]
        let bytes = build_report84_single_index(39, 0, 0, 255, 1, 8);
        let expected = concat!(
            "840108",
            "2700270027002700270027002700270000", // index slots + first byte of pad
            "00ff00",
            "000000ff00",
            "000000ff00",
            "000000ff00",
            "000000ff00",
            "000000ff00",
            "000000ff00",
            "000000ff00",
        ).replace('\n', "");
        // Easier to just rebuild the expected by hand:
        let mut want = vec![0x84_u8, 0x01, 0x08];
        for _ in 0..8 { want.extend_from_slice(&[0x27, 0x00]); }
        for _ in 0..8 { want.extend_from_slice(&[0x00, 0x00, 0xff, 0x00]); }
        assert_eq!(bytes, want, "got {}, expected_concat_attempt {}", hex_string(&bytes), expected);
        assert_eq!(bytes.len(), 51);
    }

    #[test]
    fn report84_brightness_is_clamped_to_8() {
        let bytes = build_report84_single_index(0, 0, 0, 0, 1, 200);
        assert_eq!(bytes[2], 8, "brightness must clamp to 8 (the firmware max)");
    }

    #[test]
    fn report82_layout_blue_mode1() {
        // counter = 1*16710 + 9415 = 26125 = 0x660d (LE -> 0x0d, 0x66)
        let bytes = build_report82_color(0, 0, 255, 0xff, 1);
        assert_eq!(bytes.len(), 29);
        assert_eq!(bytes[0], 0x82);
        assert_eq!(&bytes[1..5], &[0x01, 0x00, 0x0d, 0x66]); // mode_index, _, counter LE
        assert_eq!(&bytes[7..9], &[0x1e, 0x14]); // body[6..8]
        assert_eq!(&bytes[15..17], &[0x88, 0x13]); // body[14..16]
        assert_eq!(bytes[19], 0x01); // body[18]
        assert_eq!(&bytes[23..27], &[0x00, 0x00, 0xff, 0xff]); // R, G, B, A
        assert_eq!(bytes[27], 0x01); // body[26]
        assert_eq!(bytes[28], 0x3a); // body[27] = 0x39 + mode_index
    }

    #[test]
    fn report82_counter_advances_per_mode_index() {
        // The body[2..4] counter is a function of mode_index; the daemon repaint
        // currently only emits mode_index=1, but lock this behavior in to catch
        // accidental changes to the counter formula.
        let m1 = build_report82_color(0, 0, 0, 0, 1);
        let m2 = build_report82_color(0, 0, 0, 0, 2);
        let c1 = u16::from_le_bytes([m1[3], m1[4]]);
        let c2 = u16::from_le_bytes([m2[3], m2[4]]);
        assert_eq!(c1, 1 * 16710 + 9415);
        assert_eq!(c2, 2 * 16710 + 9415);
    }

    #[test]
    fn magkey_frame_blue_routes_to_next_word_byte0() {
        // W = emitters 0..3. Per the verified word model, blue lands in
        // frame[(N+1)*4] for emitter N. So all-blue W => bytes 4, 8, 12 = 0xff.
        let mut emitters = [(0_u8, 0_u8, 0_u8); 12];
        emitters[0] = (0, 0, 255);
        emitters[1] = (0, 0, 255);
        emitters[2] = (0, 0, 255);
        let frame = build_magkey_frame(&emitters);
        assert_eq!(frame[4], 0xff, "W-left blue must route to frame[4]");
        assert_eq!(frame[8], 0xff, "W-top blue must route to frame[8]");
        assert_eq!(frame[12], 0xff, "W-right blue must route to frame[12]");
        // No other byte should be set.
        for (i, &b) in frame.iter().enumerate() {
            if i == 4 || i == 8 || i == 12 {
                continue;
            }
            assert_eq!(b, 0, "frame[{i}] expected 0, got 0x{b:02x}");
        }
    }

    #[test]
    fn magkey_frame_red_lands_in_byte2() {
        let mut emitters = [(0_u8, 0_u8, 0_u8); 12];
        emitters[3] = (255, 0, 0); // A-left
        let frame = build_magkey_frame(&emitters);
        assert_eq!(frame[3 * 4 + 2], 0xff, "A-left red must land in frame[14]");
    }

    #[test]
    fn keyboard_key_index_round_trip() {
        // Sanity-check a few canonical names against their fixed indices.
        // If anyone reorders the match arms, this catches it.
        assert_eq!(keyboard_key_index("esc"), Some(0));
        assert_eq!(keyboard_key_index("q"), Some(39));
        assert_eq!(keyboard_key_index("space"), Some(93));
        assert_eq!(keyboard_key_index("keypad_enter"), Some(102));
        // W / A / S / D are intentionally absent: they are MagKeys, not main keys.
        assert_eq!(keyboard_key_index("w"), None);
        assert_eq!(keyboard_key_index("a"), None);
        assert_eq!(keyboard_key_index("s"), None);
        assert_eq!(keyboard_key_index("d"), None);
        // Aliases resolve.
        assert_eq!(keyboard_key_index("ins"), Some(14));
        assert_eq!(keyboard_key_index("up"), Some(85));
        // Normalization: case + spaces + dashes.
        assert_eq!(keyboard_key_index("Caps-Lock"), Some(56));
        assert_eq!(keyboard_key_index("Print Screen"), Some(13));
    }

    #[test]
    fn parse_word_hex_accepts_known_formats() {
        assert_eq!(parse_word_hex("ff0000ff").unwrap(), [0xff, 0x00, 0x00, 0xff]);
        assert_eq!(parse_word_hex("ff:00:00:ff").unwrap(), [0xff, 0x00, 0x00, 0xff]);
        assert_eq!(parse_word_hex("FF-00-00-FF").unwrap(), [0xff, 0x00, 0x00, 0xff]);
        assert!(parse_word_hex("toolong00").is_err());
        assert!(parse_word_hex("ff0000zz").is_err());
    }

    #[test]
    fn parse_baseline_color_named_and_rgb() {
        assert_eq!(parse_baseline_color("off").unwrap(), (0, 0, 0));
        assert_eq!(parse_baseline_color("blue").unwrap(), (0, 0, 255));
        assert_eq!(parse_baseline_color("red").unwrap(), (255, 0, 0));
        assert_eq!(parse_baseline_color("green").unwrap(), (0, 255, 0));
        assert_eq!(parse_baseline_color("128,64,255").unwrap(), (128, 64, 255));
        assert_eq!(parse_baseline_color("0,0,0").unwrap(), (0, 0, 0));
        // Bad names / bad CSV fail.
        assert!(parse_baseline_color("magenta").is_err());
        assert!(parse_baseline_color("not,a,number").is_err());
    }

    #[test]
    fn baseline_word_uses_broadcast_byte0() {
        // The 4-byte ff02 word is [0xff, R, G, B]. Byte 0 = 0xff is the
        // "broadcast" flag that makes the write reach all 102 keys.
        assert_eq!(baseline_word((0, 0, 255)), [0xff, 0x00, 0x00, 0xff]);
        assert_eq!(baseline_word((255, 0, 0)), [0xff, 0xff, 0x00, 0x00]);
        assert_eq!(baseline_word((0, 255, 0)), [0xff, 0x00, 0xff, 0x00]);
        assert_eq!(baseline_word((0, 0, 0)), [0xff, 0x00, 0x00, 0x00]);
        assert_eq!(baseline_word((128, 64, 32)), [0xff, 0x80, 0x40, 0x20]);
    }

    #[test]
    fn baseline_name_renders_presets_and_rgb() {
        assert_eq!(baseline_name((0, 0, 0)), "off");
        assert_eq!(baseline_name((0, 0, 255)), "blue");
        assert_eq!(baseline_name((255, 0, 0)), "red");
        assert_eq!(baseline_name((0, 255, 0)), "green");
        assert_eq!(baseline_name((128, 64, 32)), "128,64,32");
    }
}
