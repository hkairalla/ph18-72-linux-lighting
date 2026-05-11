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
    send_magkey_frame: (_e)  => Promise.resolve('ok'),
  };
  return new Proxy({}, {
    get(_, prop) {
      const real = window.pywebview && window.pywebview.api;
      const target = real || mock;
      return typeof target[prop] === 'function' ? target[prop].bind(target) : target[prop];
    },
  });
})();

/* ── Keyboard layout ─────────────────────────────────────────────────
   The whole keyboard is a single CSS grid: 82 quarter-unit columns by
   6 rows. 1 unit (1u) = 4 columns. Each key declares its grid-column
   start + span and its grid-row (with an optional rowSpan for the
   numpad Enter, which is two rows tall on PH18-72).

   Each entry: { name, label, col, row, span, kind?, rowSpan?, fcls? }
     name    = daemon key identifier (must match keyboard_key_index in main.rs)
     col     = grid-column-start (1-indexed)
     span    = number of columns to span (1u = 4)
     row     = grid-row (1 = F-row, 2 = number, 3 = QWERTY, 4 = home,
               5 = bottom, 6 = spacebar)
     kind    = 'magkey' for WASD (inert; MagKey panel owns these)
     rowSpan = optional vertical span (used by numpad Enter)
     fcls    = optional extra class (e.g. 'f-key' for shorter row 1 keys)
*/
const KEYS = [
  // F-row (row 1)
  { name:'esc',          label:'Esc',  col:1,  span:4, row:1, fcls:'f-key' },
  { name:'f1',           label:'F1',   col:7,  span:3, row:1, fcls:'f-key' },
  { name:'f2',           label:'F2',   col:10, span:3, row:1, fcls:'f-key' },
  { name:'f3',           label:'F3',   col:13, span:3, row:1, fcls:'f-key' },
  { name:'f4',           label:'F4',   col:16, span:3, row:1, fcls:'f-key' },
  { name:'f5',           label:'F5',   col:21, span:3, row:1, fcls:'f-key' },
  { name:'f6',           label:'F6',   col:24, span:3, row:1, fcls:'f-key' },
  { name:'f7',           label:'F7',   col:27, span:3, row:1, fcls:'f-key' },
  { name:'f8',           label:'F8',   col:30, span:3, row:1, fcls:'f-key' },
  { name:'f9',           label:'F9',   col:35, span:3, row:1, fcls:'f-key' },
  { name:'f10',          label:'F10',  col:38, span:3, row:1, fcls:'f-key' },
  { name:'f11',          label:'F11',  col:41, span:3, row:1, fcls:'f-key' },
  { name:'f12',          label:'F12',  col:44, span:3, row:1, fcls:'f-key' },
  { name:'print_screen', label:'Prnt', col:49, span:4, row:1, fcls:'f-key' },
  { name:'insert',       label:'Ins',  col:54, span:4, row:1, fcls:'f-key' },
  { name:'delete',       label:'Del',  col:59, span:4, row:1, fcls:'f-key' },
  { name:'media_prev',       label:'◀◀', col:65, span:4, row:1, fcls:'f-key' },
  { name:'media_play_pause', label:'▶∥', col:69, span:4, row:1, fcls:'f-key' },
  { name:'media_next',       label:'▶▶', col:73, span:4, row:1, fcls:'f-key' },
  { name:'power',            label:'⏻',  col:77, span:4, row:1, fcls:'f-key' },

  // Number row (row 2)
  { name:'grave',     label:'`',    col:1,  span:4,  row:2 },
  { name:'1',         label:'1',    col:5,  span:4,  row:2 },
  { name:'2',         label:'2',    col:9,  span:4,  row:2 },
  { name:'3',         label:'3',    col:13, span:4,  row:2 },
  { name:'4',         label:'4',    col:17, span:4,  row:2 },
  { name:'5',         label:'5',    col:21, span:4,  row:2 },
  { name:'6',         label:'6',    col:25, span:4,  row:2 },
  { name:'7',         label:'7',    col:29, span:4,  row:2 },
  { name:'8',         label:'8',    col:33, span:4,  row:2 },
  { name:'9',         label:'9',    col:37, span:4,  row:2 },
  { name:'0',         label:'0',    col:41, span:4,  row:2 },
  { name:'minus',     label:'-',    col:45, span:4,  row:2 },
  { name:'equal',     label:'=',    col:49, span:4,  row:2 },
  { name:'backspace', label:'Bksp', col:53, span:12, row:2 },
  { name:'predator_sense',  label:'Pred', col:65, span:4, row:2 },
  { name:'keypad_num_lock', label:'NL',   col:69, span:4, row:2 },
  { name:'keypad_divide',   label:'/',    col:73, span:4, row:2 },
  { name:'keypad_multiply', label:'*',    col:77, span:4, row:2 },

  // QWERTY (row 3)
  { name:'tab',           label:'Tab', col:1,  span:6,  row:3 },
  { name:'q',             label:'Q',   col:7,  span:4,  row:3 },
  { name:'w',             label:'W',   col:11, span:4,  row:3, kind:'magkey' },
  { name:'e',             label:'E',   col:15, span:4,  row:3 },
  { name:'r',             label:'R',   col:19, span:4,  row:3 },
  { name:'t',             label:'T',   col:23, span:4,  row:3 },
  { name:'y',             label:'Y',   col:27, span:4,  row:3 },
  { name:'u',             label:'U',   col:31, span:4,  row:3 },
  { name:'i',             label:'I',   col:35, span:4,  row:3 },
  { name:'o',             label:'O',   col:39, span:4,  row:3 },
  { name:'p',             label:'P',   col:43, span:4,  row:3 },
  { name:'left_bracket',  label:'[',   col:47, span:4,  row:3 },
  { name:'right_bracket', label:']',   col:51, span:4,  row:3 },
  { name:'backslash',     label:'\\',  col:55, span:10, row:3 },
  { name:'keypad_7',     label:'7', col:65, span:4, row:3 },
  { name:'keypad_8',     label:'8', col:69, span:4, row:3 },
  { name:'keypad_9',     label:'9', col:73, span:4, row:3 },
  { name:'keypad_minus', label:'-', col:77, span:4, row:3 },

  // Home row (row 4)
  { name:'caps_lock',  label:'Caps',  col:1,  span:7,  row:4 },
  { name:'a',          label:'A',     col:8,  span:4,  row:4, kind:'magkey' },
  { name:'s',          label:'S',     col:12, span:4,  row:4, kind:'magkey' },
  { name:'d',          label:'D',     col:16, span:4,  row:4, kind:'magkey' },
  { name:'f',          label:'F',     col:20, span:4,  row:4 },
  { name:'g',          label:'G',     col:24, span:4,  row:4 },
  { name:'h',          label:'H',     col:28, span:4,  row:4 },
  { name:'j',          label:'J',     col:32, span:4,  row:4 },
  { name:'k',          label:'K',     col:36, span:4,  row:4 },
  { name:'l',          label:'L',     col:40, span:4,  row:4 },
  { name:'semicolon',  label:';',     col:44, span:4,  row:4 },
  { name:'apostrophe', label:'"',     col:48, span:4,  row:4 },
  { name:'enter',      label:'Enter', col:52, span:13, row:4 },
  { name:'keypad_4',    label:'4', col:65, span:4, row:4 },
  { name:'keypad_5',    label:'5', col:69, span:4, row:4 },
  { name:'keypad_6',    label:'6', col:73, span:4, row:4 },
  { name:'keypad_plus', label:'+', col:77, span:4, row:4 },

  // Bottom row (row 5) — arrow_up sits in this row at col 61
  { name:'left_shift',  label:'Shift', col:1,  span:9,  row:5 },
  { name:'z',           label:'Z',     col:10, span:4,  row:5 },
  { name:'x',           label:'X',     col:14, span:4,  row:5 },
  { name:'c',           label:'C',     col:18, span:4,  row:5 },
  { name:'v',           label:'V',     col:22, span:4,  row:5 },
  { name:'b',           label:'B',     col:26, span:4,  row:5 },
  { name:'n',           label:'N',     col:30, span:4,  row:5 },
  { name:'m',           label:'M',     col:34, span:4,  row:5 },
  { name:'comma',       label:',',     col:38, span:4,  row:5 },
  { name:'period',      label:'.',     col:42, span:4,  row:5 },
  { name:'slash',       label:'/',     col:46, span:4,  row:5 },
  { name:'right_shift', label:'RShift',col:50, span:11, row:5 },
  { name:'arrow_up',    label:'Up',    col:61, span:4,  row:5 },
  { name:'keypad_1',     label:'1',   col:65, span:4, row:5 },
  { name:'keypad_2',     label:'2',   col:69, span:4, row:5 },
  { name:'keypad_3',     label:'3',   col:73, span:4, row:5 },
  { name:'keypad_enter', label:'Ent', col:77, span:4, row:5, rowSpan:2 },

  // Spacebar row (row 6) — arrow_down sits at col 61 (directly under arrow_up)
  { name:'left_ctrl',    label:'Ctrl',   col:1,  span:5,  row:6 },
  { name:'fn',           label:'Fn',     col:6,  span:4,  row:6 },
  { name:'left_windows', label:'Win',    col:10, span:4,  row:6 },
  { name:'left_alt',     label:'Alt',    col:14, span:4,  row:6 },
  { name:'space',        label:'Space',  col:18, span:20, row:6 },
  { name:'right_alt',    label:'AltGr',  col:38, span:4,  row:6 },
  { name:'menu',         label:'Menu',   col:42, span:4,  row:6 },
  { name:'copilot',      label:'Cpilot', col:46, span:11, row:6 },
  { name:'arrow_left',   label:'Lft',    col:57, span:4,  row:6 },
  { name:'arrow_down',   label:'Dn',     col:61, span:4,  row:6 },
  { name:'arrow_right',     label:'Rt',  col:65, span:4, row:6 },
  { name:'keypad_0',        label:'0',   col:69, span:4, row:6 },
  { name:'keypad_decimal',  label:'.',   col:73, span:4, row:6 },
];

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
  kbKey: null,
  mkEmitter: null,
  coverSeg: 'all',
  emitterColors: Array.from({length:12}, () => [0,0,0]),
  animRunning: false,
  animMode: 'wheel',
  animTime: 0,
  animLastTs: null,
  animSpeed: 1.0,
  hidBusy: false,
  lastHidMs: 0,
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
async function runDaemon(args) {
  setStatus(args[0], 'busy');
  const result = await api.run_daemon(args);
  pushHistory(result);
  setStatus(result.ok ? result.title + ' — ok' : result.title + ' — failed', result.ok ? 'ok' : 'err');
  return result;
}

/* ── MagKey frame send (always sends full 12-emitter state) ──────────*/
function sendMagkeyFrame(label) {
  setStatus(label, 'busy');
  api.send_magkey_frame(state.emitterColors).then(result => {
    const ok = result === 'ok';
    setStatus(label + (ok ? ' — ok' : ' — ' + result), ok ? 'ok' : 'err');
    pushHistory({ ok, title: label, output: ok ? 'frame sent' : result });
  });
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
  el.style.fill   = dark ? '' : `rgb(${r},${g},${b})`;
  el.style.filter = dark ? '' : `drop-shadow(0 0 7px rgb(${r},${g},${b}))`;
}
function updateAllEmitterSvg() {
  state.emitterColors.forEach(([r,g,b], i) => setEmitterSvg(i, r, g, b));
}

/* ── Animation loop ──────────────────────────────────────────────────*/
function animLoop(ts) {
  if (!state.animRunning) return;
  if (!state.animLastTs) state.animLastTs = ts;
  const dt = (ts - state.animLastTs) / 1000;
  state.animLastTs = ts;
  state.animTime += dt * state.animSpeed;
  const t = state.animTime;

  const fn = MODES[state.animMode];
  if (!fn) return;
  const emitters = fn(t);

  // Update SVG every frame (cheap)
  emitters.forEach(([r,g,b], i) => setEmitterSvg(i, r, g, b));

  // HID frame at ~25 fps — fire-and-forget, skip if previous in flight
  if (!state.hidBusy && ts - state.lastHidMs > 40) {
    state.hidBusy = true;
    state.lastHidMs = ts;
    api.send_magkey_frame(emitters)
      .finally(() => { state.hidBusy = false; });
  }

  requestAnimationFrame(animLoop);
}

function startAnim(mode) {
  state.animMode    = mode;
  state.animRunning = true;
  state.animTime    = 0;
  state.animLastTs  = null;
  const btn = document.getElementById('btn-anim');
  btn.textContent = '■ Stop';
  btn.classList.add('running');
  requestAnimationFrame(animLoop);
}

function stopAnim() {
  state.animRunning = false;
  state.animLastTs  = null;
  const btn = document.getElementById('btn-anim');
  btn.textContent = '▶ Start';
  btn.classList.remove('running');
  state.emitterColors = Array.from({length:12}, () => [0,0,0]);
  updateAllEmitterSvg();
  api.send_magkey_frame(state.emitterColors);
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

function _initKeyboardPanelBody() {
  const grid = document.getElementById('keyboard-grid');
  if (!grid) throw new Error('#keyboard-grid not in DOM');
  KEYS.forEach(k => {
    const btn = document.createElement('button');
    const isMagkey = k.kind === 'magkey';
    btn.className = 'kb-key' + (isMagkey ? ' kb-magkey' : '') + (k.fcls ? ' ' + k.fcls : '');
    btn.type = 'button';
    btn.textContent = k.label;
    btn.dataset.name = k.name;
    btn.style.gridColumn = `${k.col} / span ${k.span}`;
    btn.style.gridRow = k.rowSpan ? `${k.row} / span ${k.rowSpan}` : String(k.row);
    if (isMagkey) {
      btn.title = 'MagKey — use the MagKey 3.0 panel';
      btn.tabIndex = -1;
    } else {
      btn.addEventListener('click', () => {
        document.querySelectorAll('.kb-key.selected').forEach(x => x.classList.remove('selected'));
        btn.classList.add('selected');
        state.kbKey = k.name;
        document.getElementById('kb-selected-label').textContent = k.label;
      });
    }
    grid.appendChild(btn);
  });

  // Live slider preview: as the user drags R/G/B sliders, push the new
  // color to the selected key at most every 100 ms. Per-key writes are
  // ~50 ms each on the fast path, so 10 Hz keeps the UI responsive
  // without saturating the daemon process spawn.
  let liveTimer = null;
  let livePending = null;
  const SLIDER_THROTTLE_MS = 100;
  function liveApply(r, g, b) {
    if (!state.kbKey) return;
    livePending = [r, g, b];
    if (liveTimer) return;
    liveTimer = setTimeout(() => {
      const [pr, pg, pb] = livePending;
      livePending = null;
      liveTimer = null;
      if (state.kbKey) {
        runDaemon(['set-keyboard-key', '--key', state.kbKey, '--red', pr, '--green', pg, '--blue', pb]);
      }
    }, SLIDER_THROTTLE_MS);
  }

  let sliderInit = true;
  const getKbRgb = wireSliders('kb-r', 'kb-g', 'kb-b', 'kb-swatch', (r, g, b) => {
    // wireSliders fires update() once on init to seed the swatch; skip
    // that synthetic call so the daemon isn't pinged on UI startup.
    if (sliderInit) { sliderInit = false; return; }
    liveApply(r, g, b);
  });

  const wireBtn = (id, handler) => {
    const el = document.getElementById(id);
    if (!el) throw new Error(`#${id} not in DOM`);
    el.addEventListener('click', handler);
  };

  wireBtn('btn-kb-apply', () => {
    if (!state.kbKey) {
      setStatus('pick a key first', 'err');
      return;
    }
    const [r,g,b] = getKbRgb();
    runDaemon(['set-keyboard-key', '--key', state.kbKey, '--red', r, '--green', g, '--blue', b]);
  });

  wireBtn('btn-kb-clear', () => {
    if (!state.kbKey) {
      setStatus('pick a key first', 'err');
      return;
    }
    runDaemon(['clear-keyboard-key', '--key', state.kbKey]);
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

  setStatus(`keyboard panel ready (${KEYS.length} keys, ${baselineBtns.length} baselines)`);
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

  wireSliders('mk-r', 'mk-g', 'mk-b', 'mk-swatch');

  // Apply zone — updates one emitter, sends full 12-emitter frame so other keys stay lit
  document.getElementById('btn-mk-apply-zone').addEventListener('click', () => {
    if (state.mkEmitter === null) return;
    const r = +document.getElementById('mk-r').value;
    const g = +document.getElementById('mk-g').value;
    const b = +document.getElementById('mk-b').value;
    state.emitterColors[state.mkEmitter] = [r,g,b];
    setEmitterSvg(state.mkEmitter, r, g, b);
    sendMagkeyFrame('zone');
  });

  // Apply whole key
  document.getElementById('btn-mk-apply-key').addEventListener('click', () => {
    if (state.mkEmitter === null) return;
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
      // Shift background glow tint per panel
      const tints = {
        keyboard: 'rgba(0,196,222,0.055)',
        magkey: 'rgba(0,196,222,0.055)',
        'cover-logo': 'rgba(255,140,0,0.04)',
        'base-logo': 'rgba(0,222,143,0.04)',
        infinity: 'rgba(160,0,255,0.04)',
      };
      const t = tints[panel] || 'rgba(0,196,222,0.05)';
      document.getElementById('bg-glow').style.background =
        `radial-gradient(ellipse 55% 45% at 12% 55%, ${t} 0%, transparent 70%),` +
        `radial-gradient(ellipse 40% 55% at 88% 45%, rgba(0,80,160,0.03) 0%, transparent 70%)`;
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

  function setSpeed(speed) {
    speed = Math.max(MIN, Math.min(MAX, speed));
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

  setSpeed(1.0);
}

/* ── Backend badge ───────────────────────────────────────────────────*/
async function initBackend() {
  try {
    const mode = await api.get_backend_mode();
    const badge = document.getElementById('backend-badge');
    if (mode === 'cargo') {
      badge.textContent = 'Real Hardware';
      badge.classList.add('real');
    } else {
      badge.textContent = 'Mock';
    }
  } catch (_) {}
}

/* ── Boot ────────────────────────────────────────────────────────────*/
document.addEventListener('DOMContentLoaded', () => {
  initTabs();
  initHistory();
  initKeyboardPanel();
  initMagkeyPanel();
  initCoverPanel();
  initSpeedDial();
  setStatus('Ready');
  // pywebviewready fires once the Python API is injected; re-run badge check then.
  // Also call immediately for browser dev mode where there is no pywebview.
  initBackend();
  window.addEventListener('pywebviewready', initBackend);
});
