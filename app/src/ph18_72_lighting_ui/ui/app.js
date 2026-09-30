/* ── API bridge ──────────────────────────────────────────────────────
   In production: window.pywebview.api.<method>() returns a Promise.
   In browser dev mode: mock shim so the UI is inspectable without Python.
──────────────────────────────────────────────────────────────────────── */
// pywebview injects window.pywebview.api asynchronously after page load.
// Proxy reads the real API at call time so we never get stuck on the mock.
const api = (() => {
  const mock = {
    get_backend_mode:  () => Promise.resolve('mock'),
    get_history:       () => Promise.resolve([]),
    run_daemon:        (args) => Promise.resolve({ ok: true, title: args[0], output: `mock: ${args.join(' ')}` }),
    get_animation: () => Promise.resolve({ mode: 'none', speed: 1, epoch_ms: 0, phase0: 0, keepalive_mode: 'active', service_active: true }),
    get_theme: () => Promise.resolve({}),
    get_keepalive: () => Promise.resolve({ mode: 'active', minutes: 5, ac_only: true, screensaver_seconds: 150, service_active: true }),
    get_keyboard_view: (names) => Promise.resolve({
      baseline: [0, 0, 255],
      keys: Object.fromEntries(names.map(n => [n, n === 'q' ? [255, 60, 60] : n === 'e' ? [60, 255, 90] : [0, 0, 255]])),
    }),
  };
  return new Proxy({}, {
    get(_, prop) {
      const real = window.pywebview && window.pywebview.api;
      const target = real || mock;
      return typeof target[prop] === 'function' ? target[prop].bind(target) : target[prop];
    },
  });
})();


/* ── MagKey emitter spatial data ─────────────────────────────────────
   Real 2D coordinates derived from hardware photo (2026-04-26).
   Each key's 3 emitters form an equilateral triangle:
     top = up, left = down-left (210°), right = down-right (330°)
──────────────────────────────────────────────────────────────────────── */
const EMITTER_POS = [
  [ 0.297, 0.825], [ 0.600, 1.350], [ 0.903, 0.825],  // W: left, top, right
  [-0.303,-0.175], [ 0.000, 0.350], [ 0.303,-0.175],  // A
  [ 0.697,-0.175], [ 1.000, 0.350], [ 1.303,-0.175],  // S
  [ 1.697,-0.175], [ 2.000, 0.350], [ 2.303,-0.175],  // D
];
const CX = 0.90, CY = 0.25;
const X_VALS = EMITTER_POS.map(p => p[0]);
const X_MIN = Math.min(...X_VALS), X_MAX = Math.max(...X_VALS);

function emitterAngle(i) {
  const [x, y] = EMITTER_POS[i];
  return ((Math.atan2(y - CY, x - CX) / (2 * Math.PI)) + 1) % 1;
}
function normX(i) { return (EMITTER_POS[i][0] - X_MIN) / (X_MAX - X_MIN); }

/* ── Color math ──────────────────────────────────────────────────────*/
function hsvToRgb(h, s = 1, v = 1) {
  h = h % 1;
  const i = Math.floor(h * 6), f = h * 6 - i;
  const p = v*(1-s), q = v*(1-f*s), t = v*(1-(1-f)*s);
  const cases = [[v,t,p],[q,v,p],[p,v,t],[p,q,v],[t,p,v],[v,p,q]];
  const [r,g,b] = cases[i % 6];
  return [r,g,b].map(c => Math.round(c * 255));
}
function pulse(t, spd = 1) { return (Math.sin(t * spd * Math.PI * 2) + 1) / 2; }

/* ── Animation modes ─────────────────────────────────────────────────*/
const MODES = {
  wheel(t) {
    return Array.from({length:12}, (_,i) => hsvToRgb((emitterAngle(i) + t*0.12) % 1));
  },
  knight(t) {
    const sweepX = X_MIN + (X_MAX-X_MIN) * (Math.sin(t*0.6)+1)/2;
    return Array.from({length:12}, (_,i) => {
      const dist = Math.abs(EMITTER_POS[i][0] - sweepX);
      const v = Math.max(0, 1 - dist*1.6);
      return hsvToRgb(0.04 + v*0.06, 1, v);
    });
  },
  hue(t) {
    return Array.from({length:12}, (_,i) => hsvToRgb((t*0.35 + emitterAngle(i)) % 1));
  },
  chase(t) {
    const key = Math.floor(t*1.2) % 4, hue = (Math.floor(t*1.2)*0.25) % 1;
    const fl  = pulse(t, 2);
    return Array.from({length:12}, (_,i) =>
      Math.floor(i/3) === key ? hsvToRgb(hue, 1, fl) : [0,0,0]);
  },
  breathe(t) {
    const v = pulse(t, 0.4), hue = (t*0.08) % 1;
    return Array.from({length:12}, () => hsvToRgb(hue, 1, v));
  },
  zone(t) {
    const hold = 1.0, zi = Math.floor(t/hold) % 12;
    const ph = (t % hold) / hold, v = Math.sin(ph*Math.PI);
    const hue = Math.floor(zi/3)/4 + (zi%3)/12;
    return Array.from({length:12}, (_,i) => i === zi ? hsvToRgb(hue,1,v) : [0,0,0]);
  },
  cascade(t) {
    return Array.from({length:12}, (_,i) => {
      const nx = normX(i), ph = (t/5 - nx*0.7) % 1;
      const v = (Math.sin(ph*Math.PI*2)+1)/2;
      return hsvToRgb((t*0.07 + nx*0.4) % 1, 1, v);
    });
  },
};

/* ── App state ───────────────────────────────────────────────────────*/
const state = {
  panel: 'keyboard',
  kbKey: null,         // last single-selected key; null when 0 or >1 selected
  kbKeys: [],          // full selection set (length 0/1/N)
  mkEmitter: null,
  coverSeg: 'all',
  emitterColors: Array.from({length:12}, () => [0,0,0]),
  animRunning: false,
  animMode: 'wheel',
  animEpoch: 0,
  animPhase0: 0,
  animSpeed: 1.0,
};

/* ── Status bar ──────────────────────────────────────────────────────*/
function setStatus(text, type = '') {
  document.getElementById('status-text').textContent = text;
  const dot = document.getElementById('status-dot');
  dot.className = 'status-dot' + (type ? ` ${type}` : '');
}

/* ── History ─────────────────────────────────────────────────────────*/
function pushHistory(record) {
  const el = document.createElement('div');
  el.className = 'history-entry';
  const ok = record.ok;
  el.innerHTML =
    `<span class="he-title">${record.title}</span> ` +
    `<span class="${ok ? 'he-ok' : 'he-fail'}">${ok ? '✓' : '✗'}</span>\n` +
    `<span class="he-out">${record.output || ''}</span>`;
  const scroll = document.getElementById('history-scroll');
  const ph = scroll.querySelector('.history-placeholder');
  if (ph) ph.remove();
  scroll.prepend(el);
}

/* ── Daemon commands ─────────────────────────────────────────────────*/
// Commands that change what the main keyboard is showing.
const KEYBOARD_CMDS = new Set([
  'set-keyboard-key', 'clear-keyboard-key', 'reset-keyboard', 'set-keyboard-baseline',
  'set-main-keyboard-blue', 'set-main-keyboard-red', 'set-main-keyboard-green',
  'repaint-keyboard', 'restore-known-good',
]);

async function runDaemon(args) {
  setStatus(args[0], 'busy');
  const result = await api.run_daemon(args);
  pushHistory(result);
  setStatus(result.ok ? result.title + ' — ok' : result.title + ' — failed', result.ok ? 'ok' : 'err');
  if (KEYBOARD_CMDS.has(args[0])) scheduleKeyColorRefresh();
  return result;
}

/* ── Omarchy theme: mirror the desktop palette ───────────────────────
   Python reads ~/.local/state/omarchy/current/theme/colors.toml. We map it onto the CSS
   variables by ROLE (page / sidebar / card / hover), because the names do not mean the same
   in light themes ("darker_background" is a darker light grey there). Only the chrome is
   themed: the colors of the lit keys are the real keyboard colors and never change.
   With no theme ({}), every override is removed and the built-in palette shows. */
const THEME_VARS = ['--accent-rgb', '--bg', '--bg-rgb', '--bg-panel', '--bg-panel-rgb', '--bg-card',
  '--bg-hover', '--bg-selected', '--text', '--text-rgb', '--text-dim', '--text-muted', '--on-accent', '--lift',
  '--font', '--font-mono'];

function hexToRgb(hex) {
  let h = hex.replace('#', '');
  if (h.length === 3) h = [...h].map(c => c + c).join('');
  const n = parseInt(h, 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
function luminance([r, g, b]) {   // WCAG relative luminance, 0..1
  const f = v => { v /= 255; return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}

function applyTheme(theme) {
  const root = document.documentElement;
  if (!theme || !theme.colors || !theme.colors.accent) {
    THEME_VARS.forEach(v => root.style.removeProperty(v));
    root.classList.remove('theme-light');
    state.themeLight = false;
    return;
  }
  const c = theme.colors, light = theme.mode === 'light';
  const pick = (...names) => names.map(n => c[n]).find(Boolean) || c.background;
  // Roles, deepest to most lifted. Light themes are ordered by contrast instead of depth.
  const roles = light
    ? { bg: pick('background'), panel: pick('dark_background'), card: pick('lighter_background'),
        hover: pick('darker_background'), selected: pick('selection', 'muted') }
    : { bg: pick('darker_background'), panel: pick('dark_background'), card: pick('background'),
        hover: pick('lighter_background'), selected: pick('selection', 'lighter_background') };
  const accent = hexToRgb(c.accent);
  const text = hexToRgb(pick('bright_foreground', 'foreground'));
  const set = (k, v) => root.style.setProperty(k, v);
  set('--accent-rgb', accent.join(' '));
  set('--bg', roles.bg);             set('--bg-rgb', hexToRgb(roles.bg).join(' '));
  set('--bg-panel', roles.panel);    set('--bg-panel-rgb', hexToRgb(roles.panel).join(' '));
  set('--bg-card', roles.card);
  set('--bg-hover', roles.hover);
  set('--bg-selected', roles.selected);
  set('--text', c.foreground ? (c.bright_foreground || c.foreground) : '#ddd');
  set('--text-rgb', text.join(' '));
  // Dim / muted text are derived from the real text and card colors, so they always
  // read as "quieter than the text" whatever the theme calls its own shades.
  // Light themes need more of the text color to stay legible (a mid-tone text on a pale
  // card washes out fast).
  set('--text-dim', `color-mix(in srgb, var(--text) ${light ? 74 : 58}%, var(--bg-card))`);
  set('--text-muted', `color-mix(in srgb, var(--text) ${light ? 54 : 32}%, var(--bg-card))`);
  set('--on-accent', luminance(accent) > 0.45 ? '#0b0b0b' : '#ffffff');
  set('--lift', light ? 'black' : 'white');
  // Omarchy draws everything in the system monospace font (fontconfig alias); follow it.
  if (theme.font) {
    const stack = `"${theme.font}", ui-monospace, monospace`;
    set('--font', stack);
    set('--font-mono', stack);
  }
  root.classList.toggle('theme-light', light);
  state.themeLight = light;
  ['w', 'a', 's', 'd'].forEach(updateKeyLabel);   // re-lift the MagKey letters for the new mode
}

let themeStamp = '';
async function refreshTheme() {
  if (!BROWSER_DEV && !(window.pywebview && window.pywebview.api)) return;
  let theme;
  try { theme = await api.get_theme(); } catch (err) { return; }
  const stamp = JSON.stringify(theme || {});
  if (stamp === themeStamp) return;   // unchanged: nothing to repaint
  themeStamp = stamp;
  applyTheme(theme);
}

/* ── Keep lights on (keyboard + WASD sleep timer) ────────────────────
   The firmware sleeps the lights 30 s after the last laptop key press. The daemon's
   keep-alive service rewrites the saved MagKey frame to hold them on; the mode
   decides when it does. Settings live in the daemon (set-keepalive). */
function kaDescribe(cfg) {
  const mins = Math.round(cfg.screensaver_seconds / 6) / 10;   // seconds -> minutes, 1 decimal
  switch (cfg.mode) {
    case 'active':  return `Lights stay on while you use the computer and sleep after ${mins} min idle (your screensaver delay).`;
    case 'timeout': return `Lights stay on until you have been idle for ${cfg.minutes} min.`;
    case 'always':  return 'Lights never go to sleep.';
    default:        return 'Firmware default: lights sleep 30 s after the last key press on the laptop.';
  }
}

async function refreshKeepalive() {
  if (!BROWSER_DEV && !(window.pywebview && window.pywebview.api)) return;
  let cfg;
  try { cfg = await api.get_keepalive(); } catch (err) { console.warn('keepalive read failed', err); return; }
  if (!cfg || !cfg.mode) return;
  document.querySelectorAll('#ka-modes [data-ka-mode]').forEach(b =>
    b.classList.toggle('selected', b.dataset.kaMode === cfg.mode));
  document.getElementById('ka-minutes-row').hidden = cfg.mode !== 'timeout';
  const minutes = document.getElementById('ka-minutes');
  if (document.activeElement !== minutes) minutes.value = cfg.minutes;
  document.getElementById('ka-ac-only').checked = !!cfg.ac_only;
  const note = document.getElementById('ka-note');
  const needsService = cfg.mode !== 'off' && cfg.service_active === false;
  note.textContent = kaDescribe(cfg) + (needsService
    ? ' The keep-alive service is not running: systemctl --user enable --now ph18-lighting-keepalive.service'
    : '');
  note.classList.toggle('warn', needsService);
}

function initKeepalive() {
  document.querySelectorAll('#ka-modes [data-ka-mode]').forEach(btn => {
    btn.addEventListener('click', async () => {
      await runDaemon(['set-keepalive', '--mode', btn.dataset.kaMode]);
      refreshKeepalive();
    });
  });
  document.getElementById('ka-minutes-set').addEventListener('click', async () => {
    const n = Math.max(1, Math.min(1440, parseInt(document.getElementById('ka-minutes').value, 10) || 5));
    await runDaemon(['set-keepalive', '--minutes', n]);
    refreshKeepalive();
  });
  document.getElementById('ka-ac-only').addEventListener('change', async ev => {
    await runDaemon(['set-keepalive', '--ac-only', ev.target.checked ? 'true' : 'false']);
    refreshKeepalive();
  });
}

/* ── Key colors: show what the keyboard is displaying ────────────────
   The firmware is write-only, so the truth is the daemon's persisted
   {baseline, overrides}. The daemon resolves that to a color per key. */
function paintKey(name, rgb) {
  const btn = document.querySelector(`.kb-key[data-name="${name}"]`);
  if (!btn || btn.classList.contains('kb-magkey')) return;
  const [r, g, b] = rgb;
  btn.style.setProperty('--kc', `${r} ${g} ${b}`);
  btn.classList.toggle('kb-lit', r + g + b > 0);
}

let keyColorTimer = null;
function scheduleKeyColorRefresh(delayMs = 150) {
  clearTimeout(keyColorTimer);
  keyColorTimer = setTimeout(refreshKeyColors, delayMs);
}

// Plain-browser dev mode (http://) uses the mock; the real app loads from file://
// and must wait for pywebview to inject the API rather than paint mock colors.
const BROWSER_DEV = location.protocol.startsWith('http');

let keyColorSeq = 0;
async function refreshKeyColors() {
  if (!BROWSER_DEV && !(window.pywebview && window.pywebview.api)) return;
  const names = KEYS.filter(k => k.kind !== 'magkey').map(k => k.name);
  const seq = ++keyColorSeq;
  try {
    const view = await api.get_keyboard_view(names);
    // A newer refresh started while this one was in flight: drop this stale answer.
    if (seq !== keyColorSeq) return;
    if (!view || !view.keys) return;
    for (const [name, rgb] of Object.entries(view.keys)) paintKey(name, rgb);
    // Saved MagKey colors. Skip while animating, and right after an apply (the
    // answer may predate the save).
    if (view.magkeys && !state.animRunning && Date.now() - (state.mkLastApplyMs || 0) > 1500) {
      state.emitterColors = view.magkeys;
      updateAllEmitterSvg();
    }
  } catch (err) {
    console.warn('key color refresh failed', err);
  }
}

/* ── MagKey frame send (always sends full 12-emitter state) ──────────*/
// Static MagKey colors go through the daemon so they are saved and survive the
// keyboard sweep, sleep and reboot. A running animation would overwrite them on its next
// frame, so a static apply stops it first. (Animation frames are never saved.)
async function sendMagkeyFrame(_label) {
  state.mkLastApplyMs = Date.now();
  const colors = state.emitterColors.flat().join(',');
  if (state.animRunning) await stopAnim({ restore: false });
  return runDaemon(['set-magkey-emitters', '--colors', colors]);
}

/* ── MagKey helpers ──────────────────────────────────────────────────*/
function mkAllNamed(colorName) {
  const map = {off:[0,0,0], red:[255,0,0], green:[0,255,0], blue:[0,0,255]};
  const [r,g,b] = map[colorName] || [0,0,0];
  state.emitterColors = Array.from({length:12}, () => [r,g,b]);
  updateAllEmitterSvg();
  sendMagkeyFrame(colorName);
}

/* ── SVG emitter color update ────────────────────────────────────────*/
function setEmitterSvg(idx, r, g, b) {
  const el = document.getElementById(`em-${idx}`);
  if (!el) return;
  const dark = r < 15 && g < 15 && b < 15;
  // Same language as the keyboard page: a muted body, with the color carried
  // by the outline, the glow and the letter. --zc is "r g b".
  el.classList.toggle('lit', !dark);
  el.dataset.rgb = dark ? '' : `${r},${g},${b}`;
  if (!dark) el.style.setProperty('--zc', `${r} ${g} ${b}`);
  // The halo sits behind the key bodies, unclipped, so it spills past the edge.
  const halo = document.getElementById(`gl-${idx}`);
  if (halo) {
    halo.classList.toggle('lit', !dark);
    if (!dark) halo.style.setProperty('--hc', `${r} ${g} ${b}`);
  }
  updateKeyLabel(el.dataset.key);
}

// The W/A/S/D letter is a gradient across the key's left / top / right zone
// colors, so a multi-color key shows all of them and a single color stays flat.
const ZONE_ORDER = ['left', 'top', 'right'];
function updateKeyLabel(key) {
  const label = document.getElementById(`kl-${key}`);
  if (!label) return;
  const zones = ZONE_ORDER.map(z => document.querySelector(`.emitter[data-key="${key}"][data-zone="${z}"]`));
  const lit = zones.some(z => z && z.classList.contains('lit'));
  label.classList.toggle('lit', lit);
  label.style.fill = lit ? `url(#klg-${key})` : '';
  label.style.stroke = lit ? `url(#klg-${key})` : '';
  // Raw zone colors (null when a zone is dark), shared by the MagKey page and the Keyboard page.
  const rgbs = zones.map(z => (z && z.classList.contains('lit')) ? z.dataset.rgb.split(',').map(Number) : null);
  // Lift toward white (black on light themes) a little so pure blues stay readable.
  const target = state.themeLight ? 0 : 255;
  const lift = c => Math.round(c + (target - c) * 0.22);
  const lifted = rgbs.map(c => c ? `rgb(${lift(c[0])} ${lift(c[1])} ${lift(c[2])})` : null);
  if (lit) {
    zones.forEach((z, i) => {
      const stop = document.getElementById(`kls-${key}-${i}`);
      if (stop) stop.style.stopColor = lifted[i] || 'rgb(40 60 75)';
    });
  }
  // Mirror onto the Keyboard page's (read-only) WASD key. While an animation runs on another
  // page that would repaint four hidden keys 30 times a second for nothing, so skip it then;
  // switching back to the Keyboard page re-syncs (see initTabs).
  if (!state.animRunning || state.panel === 'keyboard') paintMagkeyOnKeyboard(key, rgbs, lifted);
}

// The WASD keys on the Keyboard page are MagKeys: not editable there, but they show what the
// MagKey page has set, as a gradient across the key's left / top / right zone colors.
function paintMagkeyOnKeyboard(key, rgbs, lifted) {
  const btn = document.querySelector(`.kb-key.kb-magkey[data-name="${key}"]`);
  if (!btn) return;
  const lit = rgbs.some(Boolean);
  btn.classList.toggle('mk-lit', lit);
  if (!lit) return;
  ['l', 't', 'r'].forEach((side, i) => {
    btn.style.setProperty(`--mk-${side}`, (rgbs[i] || [40, 60, 75]).join(' '));
    btn.style.setProperty(`--mkt-${side}`, lifted[i] || 'rgb(40 60 75)');
  });
}
function updateAllEmitterSvg() {
  state.emitterColors.forEach(([r,g,b], i) => setEmitterSvg(i, r, g, b));
}

/* ── Animation ───────────────────────────────────────────────────────
   The background service (ph18-lighting-keepalive) plays the animation, so it keeps running
   when this window closes. Here it is only a remote control plus a preview: the clock is the
   daemon's (t = phase0 + (now - epoch) * speed), so the preview matches what the keys show. */
function animLoop(ts) {
  if (!state.animRunning) return;
  // Repaint at ~30 fps, not every display frame: WebKitGTK renders in software here, and
  // repainting at 60-250 Hz kept the app near 100% of a core.
  if (ts - (state.lastSvgMs || 0) >= 33) {
    state.lastSvgMs = ts;
    const t = Math.max(0, state.animPhase0 + (Date.now() - state.animEpoch) / 1000 * state.animSpeed);
    const fn = MODES[state.animMode];
    if (fn) fn(t).forEach(([r,g,b], i) => setEmitterSvg(i, r, g, b));
  }
  requestAnimationFrame(animLoop);
}

function showAnimRunning(running) {
  const btn = document.getElementById('btn-anim');
  btn.textContent = running ? '■ Stop' : '▶ Start';
  btn.classList.toggle('running', running);
  const tab = document.querySelector('.tab[data-panel="magkey"]');
  if (tab) tab.classList.toggle('animating', running);
}

function beginPreview() {
  const wasRunning = state.animRunning;
  state.animRunning = true;
  showAnimRunning(true);
  if (!wasRunning) requestAnimationFrame(animLoop);
}

// Adopt the daemon's animation state. Used at startup and on focus, so a window opened while an
// animation is playing shows it, and one stopped from the CLI stops previewing.
async function syncAnimation() {
  let cfg;
  try { cfg = await api.get_animation(); } catch (_) { return; }
  if (!cfg || !cfg.mode) return;
  const note = document.getElementById('anim-note');
  if (note) {
    const dead = cfg.mode !== 'none' && cfg.service_active === false;
    note.hidden = !dead;
  }
  if (cfg.mode === 'none') {
    if (state.animRunning) await stopAnim({ restore: true, send: false });
    return;
  }
  if (!MODES[cfg.mode]) return;
  state.animMode = cfg.mode;
  state.animSpeed = cfg.speed;
  state.animEpoch = cfg.epoch_ms;
  state.animPhase0 = cfg.phase0;
  const sel = document.getElementById('anim-select');
  if (sel) sel.value = cfg.mode;
  if (state.setDialSpeed) state.setDialSpeed(cfg.speed, { send: false });
  beginPreview();
}

async function startAnim(mode) {
  state.animMode = mode;
  state.animEpoch = Date.now();
  state.animPhase0 = 0;
  beginPreview();
  const r = await runDaemon(['set-animation', '--mode', mode, '--speed', state.animSpeed.toFixed(2)]);
  // Adopt the daemon's clock (it stamped its own epoch).
  if (r && r.ok) syncAnimation();
}

async function stopAnim({ restore = true, send = true } = {}) {
  state.animRunning = false;
  showAnimRunning(false);
  if (send) await runDaemon(['set-animation', '--mode', 'none']);
  if (restore) {
    // The daemon re-sends the saved static colors; show them here too.
    state.mkLastApplyMs = 0;
    refreshKeyColors();
  }
}

/* ── Slider helpers ──────────────────────────────────────────────────*/
function wireSliders(rId, gId, bId, swatchId, onChange) {
  const rEl = document.getElementById(rId);
  const gEl = document.getElementById(gId);
  const bEl = document.getElementById(bId);
  const sw  = document.getElementById(swatchId);
  const rvEl = document.getElementById(rId + '-val');
  const gvEl = document.getElementById(gId + '-val');
  const bvEl = document.getElementById(bId + '-val');

  function update() {
    const r = +rEl.value, g = +gEl.value, b = +bEl.value;
    if (rvEl) rvEl.textContent = r;
    if (gvEl) gvEl.textContent = g;
    if (bvEl) bvEl.textContent = b;
    if (sw) sw.style.background = `rgb(${r},${g},${b})`;
    if (onChange) onChange(r, g, b);
  }
  rEl.addEventListener('input', update);
  gEl.addEventListener('input', update);
  bEl.addEventListener('input', update);
  update();

  return () => [+rEl.value, +gEl.value, +bEl.value];
}

/* ── Keyboard panel init ─────────────────────────────────────────────*/
function initKeyboardPanel() {
  try {
    _initKeyboardPanelBody();
  } catch (err) {
    console.error('initKeyboardPanel failed:', err);
    setStatus('keyboard init failed: ' + (err && err.message || err), 'err');
  }
}

/* ── Key legends ─────────────────────────────────────────────────────
   Build what is printed on a keycap from its layout data (see layout.js): a shifted symbol above
   the main character, a small second legend below (numpad Home/End/...), the small "F1" over an
   icon on the function row, an optional corner mark (the € on 5), and arrow glyphs. */
const SVG_NS = 'http://www.w3.org/2000/svg';
function mk(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}
function iconEl(name, filled) {
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.setAttribute('class', 'ic' + (filled ? ' filled' : ''));
  svg.setAttribute('aria-hidden', 'true');
  const use = document.createElementNS(SVG_NS, 'use');
  use.setAttribute('href', `#ic-${name}`);
  svg.appendChild(use);
  return svg;
}
// Longest line printed at the key's main size (the sub-legend is sized separately).
// A key with an icon (or a small "F1" tag) prints no text of its own unless `main` says so.
const legendMain = (lg, label) => lg.main !== undefined ? lg.main : (lg.icon || lg.fn ? '' : label);
function legendChars(lg, label) {
  const main = legendMain(lg, label);
  const mainLine = (lg.arrow ? 2 : 0) + main.length + (lg.icon && main ? 2 : 0);
  return Math.max(mainLine, (lg.top || '').length, (lg.fn || '').length, lg.icon && !main ? 1.6 : 1);
}
function renderLegend(k) {
  if (k.kind === 'magkey') {
    // One letter in a span, so it can be filled with the key's zone colors (paintMagkeyOnKeyboard).
    return mk('span', 'lbl', k.label.toUpperCase());
  }
  const lg = k.lg, main = legendMain(lg, k.label);
  const wrap = mk('span', 'lg' + (lg.icon && main ? ' lg-row' : ''));
  if (lg.fn) wrap.appendChild(mk('span', 'lg-fn', lg.fn));
  if (lg.top) wrap.appendChild(mk('span', 'lg-top', lg.top));
  if (lg.icon) wrap.appendChild(iconEl(lg.icon, lg.fill));
  if (main || lg.arrow) wrap.appendChild(mk('span', 'lg-main', (lg.arrow ? lg.arrow + ' ' : '') + main));
  if (lg.sub) wrap.appendChild(mk('span', 'lg-sub', lg.sub));
  if (lg.corner) wrap.appendChild(mk('span', 'lg-corner', lg.corner));
  if (lg.bar) wrap.appendChild(mk('span', 'lg-bar'));
  return wrap;
}

function _initKeyboardPanelBody() {
  const grid = document.getElementById('keyboard-grid');
  if (!grid) throw new Error('#keyboard-grid not in DOM');
  KEYS.forEach(k => {
    const btn = document.createElement('button');
    const isMagkey = k.kind === 'magkey';
    btn.className = 'kb-key' + (isMagkey ? ' kb-magkey' : '') + (k.fcls ? ' ' + k.fcls : '');
    btn.type = 'button';
    btn.appendChild(renderLegend(k));
    // Size the legends to fit the key: monospace glyphs are ~0.62em wide, plus a little padding.
    const fit = (chars) => ((k.w * 4) / (chars * 0.62 + 0.8)).toFixed(3);
    btn.style.setProperty('--fit', fit(legendChars(k.lg, k.label)));
    if (k.lg.sub) btn.style.setProperty('--fit-s', fit(k.lg.sub.length));
    btn.dataset.name = k.name;
    btn.dataset.label = k.label;
    // Position from the layout data (units of one standard key); the stylesheet does the math.
    btn.style.setProperty('--x', k.x);
    btn.style.setProperty('--y', k.y);
    btn.style.setProperty('--w', k.w);
    btn.style.setProperty('--h', k.h);
    if (isMagkey) {
      btn.title = 'MagKey — edit it on the MagKey 3.0 tab';
      btn.tabIndex = -1;
    } else {
      btn.title = k.label;
    }
    grid.appendChild(btn);
  });

  function updateSelectionLabel() {
    const label = document.getElementById('kb-selected-label');
    const n = state.kbKeys.length;
    if (n === 0) {
      label.textContent = 'Select a key';
    } else if (n === 1) {
      const btn = document.querySelector(`.kb-key[data-name="${state.kbKeys[0]}"]`);
      label.textContent = btn ? btn.dataset.label : state.kbKeys[0];
    } else {
      label.textContent = `${n} keys`;
    }
  }

  function setSelection(names) {
    // Filter out MagKey names — drag-box may sweep over WASD; ignore them.
    const filtered = [...new Set(names)].filter(name => {
      const btn = document.querySelector(`.kb-key[data-name="${name}"]`);
      return btn && !btn.classList.contains('kb-magkey');
    });
    document.querySelectorAll('.kb-key.selected').forEach(x => x.classList.remove('selected'));
    filtered.forEach(name => {
      const btn = document.querySelector(`.kb-key[data-name="${name}"]`);
      if (btn) btn.classList.add('selected');
    });
    state.kbKeys = filtered;
    state.kbKey = filtered.length === 1 ? filtered[0] : null;
    updateSelectionLabel();
  }

  // Click-to-select a single key. Mousedown starts a potential box-drag
  // (see below); if the pointer moves > DRAG_THRESHOLD before mouseup,
  // it's a drag — we suppress this click via a flag on the grid.
  grid.addEventListener('click', (ev) => {
    if (grid.dataset.suppressClick === '1') {
      grid.dataset.suppressClick = '';
      return;
    }
    const btn = ev.target.closest('.kb-key');
    if (!btn || btn.classList.contains('kb-magkey') || btn.classList.contains('kb-spacer')) {
      return;
    }
    setSelection([btn.dataset.name]);
  });

  // Drag-box multi-selection. mousedown anywhere on the grid (including on
  // a key) starts a potential drag. Once the pointer crosses DRAG_THRESHOLD
  // we render a dashed selection box and switch into drag mode.
  const DRAG_THRESHOLD = 6;
  let dragState = null;

  grid.addEventListener('mousedown', (ev) => {
    if (ev.button !== 0) return; // left click only
    dragState = { startX: ev.clientX, startY: ev.clientY, boxEl: null };
  });

  window.addEventListener('mousemove', (ev) => {
    if (!dragState) return;
    const dx = ev.clientX - dragState.startX;
    const dy = ev.clientY - dragState.startY;
    if (!dragState.boxEl && Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
    if (!dragState.boxEl) {
      dragState.boxEl = document.createElement('div');
      dragState.boxEl.className = 'kb-select-box';
      document.body.appendChild(dragState.boxEl);
    }
    const left = Math.min(ev.clientX, dragState.startX);
    const top = Math.min(ev.clientY, dragState.startY);
    Object.assign(dragState.boxEl.style, {
      left: left + 'px',
      top: top + 'px',
      width: Math.abs(dx) + 'px',
      height: Math.abs(dy) + 'px',
    });
  });

  window.addEventListener('mouseup', (ev) => {
    if (!dragState) return;
    const wasDrag = dragState.boxEl !== null;
    if (wasDrag) {
      // Suppress the click event that follows mouseup on the originating
      // grid element so the click handler doesn't immediately overwrite
      // our drag selection with a single-key select.
      grid.dataset.suppressClick = '1';
      const boxRect = dragState.boxEl.getBoundingClientRect();
      const hits = [];
      document.querySelectorAll('.kb-key').forEach(btn => {
        if (btn.classList.contains('kb-magkey') || btn.classList.contains('kb-spacer')) return;
        const r = btn.getBoundingClientRect();
        const intersects = !(r.right < boxRect.left || r.left > boxRect.right ||
                             r.bottom < boxRect.top || r.top > boxRect.bottom);
        if (intersects) hits.push(btn.dataset.name);
      });
      dragState.boxEl.remove();
      setSelection(hits);
    }
    dragState = null;
  });

  // Live slider preview: when EXACTLY one key is selected, push the color
  // to the daemon at most every 100 ms while dragging. Multi-selection
  // intentionally does NOT live-preview (would spawn N daemon processes
  // per tick); user clicks Apply Key instead.
  let liveTimer = null;
  let livePending = null;
  const SLIDER_THROTTLE_MS = 100;
  function liveApply(r, g, b) {
    if (state.kbKeys.length !== 1) return;
    paintKey(state.kbKeys[0], [r, g, b]);
    livePending = [r, g, b, state.kbKeys[0]];
    if (liveTimer) return;
    liveTimer = setTimeout(() => {
      const [pr, pg, pb, key] = livePending;
      livePending = null;
      liveTimer = null;
      // Re-check selection at fire time; user may have moved on.
      if (state.kbKeys.length === 1 && state.kbKeys[0] === key) {
        runDaemon(['set-keyboard-key', '--key', key, '--red', pr, '--green', pg, '--blue', pb]);
      }
    }, SLIDER_THROTTLE_MS);
  }

  let sliderInit = true;
  const getKbRgb = wireSliders('kb-r', 'kb-g', 'kb-b', 'kb-swatch', (r, g, b) => {
    if (sliderInit) { sliderInit = false; return; }
    liveApply(r, g, b);
  });

  const wireBtn = (id, handler) => {
    const el = document.getElementById(id);
    if (!el) throw new Error(`#${id} not in DOM`);
    el.addEventListener('click', handler);
  };

  wireBtn('btn-kb-apply', () => {
    if (state.kbKeys.length === 0) {
      setStatus('pick a key first', 'err');
      return;
    }
    const [r,g,b] = getKbRgb();
    state.kbKeys.forEach(key => {
      paintKey(key, [r, g, b]);
      runDaemon(['set-keyboard-key', '--key', key, '--red', r, '--green', g, '--blue', b]);
    });
  });

  wireBtn('btn-kb-clear', () => {
    if (state.kbKeys.length === 0) {
      setStatus('pick a key first', 'err');
      return;
    }
    state.kbKeys.forEach(key => {
      runDaemon(['clear-keyboard-key', '--key', key]);
    });
  });

  wireBtn('btn-kb-reset', () => {
    runDaemon(['reset-keyboard']);
  });

  const baselineBtns = document.querySelectorAll('[data-baseline]');
  if (baselineBtns.length === 0) throw new Error('no [data-baseline] buttons in DOM');
  baselineBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      runDaemon(['set-keyboard-baseline', '--color', btn.dataset.baseline]);
    });
  });

}

/* ── MagKey panel init ───────────────────────────────────────────────*/
const EMITTER_META = [
  {key:'w',zone:'left'},{key:'w',zone:'top'},{key:'w',zone:'right'},
  {key:'a',zone:'left'},{key:'a',zone:'top'},{key:'a',zone:'right'},
  {key:'s',zone:'left'},{key:'s',zone:'top'},{key:'s',zone:'right'},
  {key:'d',zone:'left'},{key:'d',zone:'top'},{key:'d',zone:'right'},
];

function initMagkeyPanel() {
  // Wire emitter clicks
  document.querySelectorAll('.emitter').forEach(el => {
    el.addEventListener('click', () => {
      const idx = +el.dataset.idx;
      document.querySelectorAll('.emitter').forEach(e => e.classList.remove('selected'));
      el.classList.add('selected');
      state.mkEmitter = idx;
      const m = EMITTER_META[idx];
      document.getElementById('mk-selected-label').textContent =
        `${m.key.toUpperCase()} · ${m.zone.charAt(0).toUpperCase()+m.zone.slice(1)}`;
      // Load current color into sliders
      const [r,g,b] = state.emitterColors[idx];
      document.getElementById('mk-r').value = r;
      document.getElementById('mk-g').value = g;
      document.getElementById('mk-b').value = b;
      document.getElementById('mk-r-val').textContent = r;
      document.getElementById('mk-g-val').textContent = g;
      document.getElementById('mk-b-val').textContent = b;
      document.getElementById('mk-swatch').style.background = `rgb(${r},${g},${b})`;
    });
  });

  // Click empty space around the keys (not the sidebar controls) or press Esc
  // to deselect the zone.
  function deselectZone() {
    document.querySelectorAll('.emitter.selected').forEach(e => e.classList.remove('selected'));
    state.mkEmitter = null;
    document.getElementById('mk-selected-label').textContent = '— select a zone —';
  }
  document.querySelector('.magkey-main').addEventListener('click', ev => {
    if (!ev.target.closest('.emitter')) deselectZone();
  });
  document.addEventListener('keydown', ev => {
    if (ev.key === 'Escape' && document.getElementById('panel-magkey').classList.contains('active')) {
      deselectZone();
    }
  });

  wireSliders('mk-r', 'mk-g', 'mk-b', 'mk-swatch');

  // Apply zone — updates one emitter, sends full 12-emitter frame so other keys stay lit
  document.getElementById('btn-mk-apply-zone').addEventListener('click', () => {
    if (state.mkEmitter === null) { setStatus('pick a zone first', 'err'); return; }
    const r = +document.getElementById('mk-r').value;
    const g = +document.getElementById('mk-g').value;
    const b = +document.getElementById('mk-b').value;
    state.emitterColors[state.mkEmitter] = [r,g,b];
    setEmitterSvg(state.mkEmitter, r, g, b);
    sendMagkeyFrame('zone');
  });

  // Apply whole key
  document.getElementById('btn-mk-apply-key').addEventListener('click', () => {
    if (state.mkEmitter === null) { setStatus('pick a zone first', 'err'); return; }
    const r = +document.getElementById('mk-r').value;
    const g = +document.getElementById('mk-g').value;
    const b = +document.getElementById('mk-b').value;
    const m = EMITTER_META[state.mkEmitter];
    const base = {'w':0,'a':3,'s':6,'d':9}[m.key];
    state.emitterColors[base] = [r,g,b];
    state.emitterColors[base+1] = [r,g,b];
    state.emitterColors[base+2] = [r,g,b];
    updateAllEmitterSvg();
    sendMagkeyFrame('key-' + m.key);
  });

  // Apply all keys
  document.getElementById('btn-mk-apply-all').addEventListener('click', () => {
    const r = +document.getElementById('mk-r').value;
    const g = +document.getElementById('mk-g').value;
    const b = +document.getElementById('mk-b').value;
    state.emitterColors = Array.from({length:12}, () => [r,g,b]);
    updateAllEmitterSvg();
    sendMagkeyFrame('all-keys');
  });

  // Animation
  document.getElementById('btn-anim').addEventListener('click', () => {
    if (state.animRunning) {
      stopAnim();
    } else {
      const mode = document.getElementById('anim-select').value;
      if (!mode) return;
      startAnim(mode);
    }
  });
}

/* ── Cover logo panel init ───────────────────────────────────────────*/
function initCoverPanel() {
  document.querySelectorAll('.cover-zone').forEach(el => {
    el.addEventListener('click', () => {
      document.querySelectorAll('.cover-zone').forEach(z => z.classList.remove('selected'));
      el.classList.add('selected');
      state.coverSeg = el.dataset.seg;
      document.getElementById('cl-selected-label').textContent = el.textContent + ' Zone';
    });
  });
  // Select "all" by default
  document.getElementById('cz-all').classList.add('selected');

  const getClRgb = wireSliders('cl-r', 'cl-g', 'cl-b', 'cl-swatch');

  document.getElementById('btn-cl-apply').addEventListener('click', () => {
    const [r,g,b] = getClRgb();
    const args = ['set-cover-logo', '--red', r, '--green', g, '--blue', b];
    if (state.coverSeg !== 'all') args.push('--segment', state.coverSeg);
    runDaemon(args);
    // Update zone visual
    const seg = state.coverSeg === 'all'
      ? ['cz-left','cz-middle','cz-right']
      : [`cz-${state.coverSeg}`];
    seg.forEach(id => {
      const z = document.getElementById(id);
      if (z) {
        z.style.background = `rgb(${r},${g},${b})`;
        const lum = (r*299 + g*587 + b*114) / 1000;
        z.style.color = lum > 140 ? '#111' : '#eef';
      }
    });
  });

  document.getElementById('btn-brightness-apply').addEventListener('click', () => {
    const level = document.getElementById('cover-brightness').value;
    runDaemon(['set-cover-logo-brightness', '--level', level]);
  });
}

/* ── Tab switching ───────────────────────────────────────────────────*/
function initTabs() {
  document.querySelectorAll('.tab').forEach(tab => {
    tab.addEventListener('click', () => {
      const panel = tab.dataset.panel;
      document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
      document.querySelectorAll('.panel').forEach(p => p.classList.remove('active'));
      tab.classList.add('active');
      const el = document.getElementById(`panel-${panel}`);
      if (el) el.classList.add('active');
      state.panel = panel;
      if (panel === 'keyboard') ['w', 'a', 's', 'd'].forEach(updateKeyLabel);
      // Shift background glow tint per panel
      const tints = {
        keyboard: 'rgb(var(--accent-rgb) / 0.055)',
        magkey: 'rgb(var(--accent-rgb) / 0.055)',
        'cover-logo': 'rgba(255,140,0,0.04)',
        'base-logo': 'rgba(0,222,143,0.04)',
        infinity: 'rgba(160,0,255,0.04)',
      };
      const t = tints[panel] || 'rgb(var(--accent-rgb) / 0.05)';
      document.getElementById('bg-glow').style.background =
        `radial-gradient(ellipse 55% 45% at 12% 55%, ${t} 0%, transparent 70%),` +
        `radial-gradient(ellipse 40% 55% at 88% 45%, rgb(var(--accent-rgb) / 0.03) 0%, transparent 70%)`;
    });
  });
}

/* ── History drawer ──────────────────────────────────────────────────*/
function initHistory() {
  document.getElementById('btn-history-toggle').addEventListener('click', () => {
    const drawer = document.getElementById('history-drawer');
    drawer.classList.toggle('open');
    document.getElementById('btn-history-toggle').textContent =
      drawer.classList.contains('open') ? 'History ▴' : 'History ▾';
  });
}

/* ── Speed dial ──────────────────────────────────────────────────────*/
function initSpeedDial() {
  const MIN = 0.1, MAX = 4.0;
  const START_DEG = 225, SWEEP_DEG = 270;
  const R = 28, CIRC = 2 * Math.PI * R;
  const ARC_LEN = CIRC * SWEEP_DEG / 360;
  const CX = 40, CY = 40, DOT_R = 26;

  const svg    = document.getElementById('speed-dial');
  const fillEl = document.getElementById('dial-fill');
  const dotEl  = document.getElementById('dial-dot');
  const valEl  = document.getElementById('speed-val');

  let speedTimer = null;
  function setSpeed(speed, { send = true } = {}) {
    speed = Math.max(MIN, Math.min(MAX, speed));
    if (state.animRunning && send) {
      // Re-base the preview clock so the speed change is seamless, then tell the daemon once the
      // dial stops moving (it re-bases its own clock the same way).
      const now = Date.now();
      state.animPhase0 = Math.max(0, state.animPhase0 + (now - state.animEpoch) / 1000 * state.animSpeed);
      state.animEpoch = now;
      clearTimeout(speedTimer);
      speedTimer = setTimeout(async () => {
        await runDaemon(['set-animation', '--speed', state.animSpeed.toFixed(2)]);
        syncAnimation();
      }, 250);
    }
    state.animSpeed = speed;
    const t = (speed - MIN) / (MAX - MIN);
    fillEl.setAttribute('stroke-dasharray', `${(t * ARC_LEN).toFixed(2)} ${CIRC.toFixed(2)}`);
    const angleDeg = START_DEG + t * SWEEP_DEG;
    const rad = (angleDeg - 90) * Math.PI / 180;
    dotEl.setAttribute('cx', (CX + DOT_R * Math.cos(rad)).toFixed(1));
    dotEl.setAttribute('cy', (CY + DOT_R * Math.sin(rad)).toFixed(1));
    valEl.textContent = speed.toFixed(1) + '×';
  }

  function svgAngle(clientX, clientY) {
    const rect = svg.getBoundingClientRect();
    const mx = clientX - rect.left - rect.width / 2;
    const my = clientY - rect.top - rect.height / 2;
    let deg = Math.atan2(mx, -my) * 180 / Math.PI;
    if (deg < 0) deg += 360;
    return deg;
  }

  let dragging = false, lastAngle = null;
  svg.addEventListener('mousedown', e => {
    dragging = true; lastAngle = svgAngle(e.clientX, e.clientY); e.preventDefault();
  });
  window.addEventListener('mouseup', () => { dragging = false; lastAngle = null; });
  window.addEventListener('mousemove', e => {
    if (!dragging || lastAngle === null) return;
    const angle = svgAngle(e.clientX, e.clientY);
    let delta = angle - lastAngle;
    if (delta > 180) delta -= 360;
    if (delta < -180) delta += 360;
    lastAngle = angle;
    setSpeed(state.animSpeed + delta / SWEEP_DEG * (MAX - MIN));
  });
  svg.addEventListener('wheel', e => {
    e.preventDefault();
    setSpeed(state.animSpeed - e.deltaY * 0.004);
  }, { passive: false });

  state.setDialSpeed = setSpeed;
  setSpeed(1.0, { send: false });
}

/* ── Backend badge ───────────────────────────────────────────────────*/
async function initBackend() {
  try {
    const mode = await api.get_backend_mode();
    // "Real hardware" is the normal state and needs no badge; only the demo mode is worth flagging.
    // Only trust "mock" once the real API is connected (or in plain-browser dev), otherwise the
    // page's built-in fake backend would flash the badge at startup.
    const settled = BROWSER_DEV || !!(window.pywebview && window.pywebview.api);
    document.getElementById('backend-badge').hidden = !(settled && mode === 'mock');
  } catch (_) {}
  refreshKeyColors();
  refreshKeepalive();
  refreshTheme();
  syncAnimation();
}

/* ── Boot ────────────────────────────────────────────────────────────*/
document.addEventListener('DOMContentLoaded', () => {
  initTabs();
  initHistory();
  initKeyboardPanel();
  initKeepalive();
  initMagkeyPanel();
  initCoverPanel();
  initSpeedDial();
  // pywebviewready fires once the Python API is injected; re-run badge check then.
  // Also call immediately for browser dev mode where there is no pywebview.
  initBackend();
  window.addEventListener('pywebviewready', initBackend);
  // The state can change outside the UI (CLI, restore service): resync on focus.
  window.addEventListener('focus', () => { scheduleKeyColorRefresh(0); refreshKeepalive(); refreshTheme(); syncAnimation(); });
  setInterval(refreshTheme, 3000);   // pick up `omarchy theme set` while the window is open
});
