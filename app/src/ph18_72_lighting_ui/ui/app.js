/* ── API bridge ──────────────────────────────────────────────────────
   In production: window.pywebview.api.<method>() returns a Promise.
   In browser dev mode: mock shim so the UI is inspectable without Python.
──────────────────────────────────────────────────────────────────────── */
const api = (() => {
  if (window.pywebview) return window.pywebview.api;
  // dev shim
  return {
    get_backend_mode:    () => Promise.resolve('mock'),
    get_history:         () => Promise.resolve([]),
    run_daemon:          (args) => Promise.resolve({ ok: true, title: args[0], output: `mock: ${args.join(' ')}` }),
    send_magkey_frame:   (_e)  => Promise.resolve('ok'),
  };
})();

/* ── Keyboard layout ─────────────────────────────────────────────────*/
const KB_ROWS = [
  [['Esc','esc'],['1','1'],['2','2'],['3','3'],['4','4'],['5/%','5'],['6','6'],
   ['7','7'],['8','8'],['9','9'],['0','0'],['-','minus'],['=','equal'],['Bksp','backspace']],
  [['Tab','tab'],['Q','q'],['W','w'],['E','e'],['R','r'],['T','t'],['Y','y'],
   ['U','u'],['I','i'],['O','o'],['P','p'],['[','left_bracket'],[']','right_bracket'],['\\','backslash']],
  [['Caps','caps_lock'],['A','a'],['S','s'],['D','d'],['F','f'],['G','g'],['H','h'],
   ['J','j'],['K','k'],['L','l'],[';/:','semicolon'],["'",'apostrophe'],['Enter','enter']],
  [['Shift','left_shift'],['Z','z'],['X','x'],['C','c'],['V','v'],['B','b'],['N','n'],
   ['M','m'],[',','comma'],['.','.period'],['/','/slash'],['RShift','right_shift']],
  [['Ctrl','left_ctrl'],['Win','left_windows'],['Alt','left_alt'],['Space','space'],
   ['RAlt','right_alt'],['←','arrow_left'],['↓','arrow_down'],['→','arrow_right'],['↑','arrow_up']],
];
const KB_ENABLED = new Set(['5','semicolon','keypad_6','arrow_down']);

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
  animStart: null,
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

/* ── MagKey helpers ──────────────────────────────────────────────────*/
function mkAllNamed(colorName) {
  const map = {off:[0,0,0], red:[255,0,0], green:[0,255,0], blue:[0,0,255]};
  const [r,g,b] = map[colorName] || [0,0,0];
  state.emitterColors = Array.from({length:12}, () => [r,g,b]);
  updateAllEmitterSvg();
  runDaemon(['set-magkeys-whole', '--color', colorName]);
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
  if (!state.animStart) state.animStart = ts;
  const t = (ts - state.animStart) / 1000;

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
  state.animMode   = mode;
  state.animRunning = true;
  state.animStart  = null;
  const btn = document.getElementById('btn-anim');
  btn.textContent = '■ Stop';
  btn.classList.add('running');
  requestAnimationFrame(animLoop);
}

function stopAnim() {
  state.animRunning = false;
  state.animStart   = null;
  const btn = document.getElementById('btn-anim');
  btn.textContent = '▶ Start';
  btn.classList.remove('running');
  // Send all-off
  api.send_magkey_frame(Array.from({length:12}, () => [0,0,0]));
  state.emitterColors.forEach((_,i) => setEmitterSvg(i, 0, 0, 0));
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
  const grid = document.getElementById('keyboard-grid');
  KB_ROWS.forEach(row => {
    const rowEl = document.createElement('div');
    rowEl.className = 'kb-row';
    row.forEach(([label, name]) => {
      const btn = document.createElement('button');
      btn.className = 'kb-key' + (KB_ENABLED.has(name) ? '' : ' disabled');
      btn.textContent = label;
      btn.dataset.name = name;
      btn.addEventListener('click', () => {
        if (!KB_ENABLED.has(name)) return;
        document.querySelectorAll('.kb-key').forEach(k => k.classList.remove('selected'));
        btn.classList.add('selected');
        state.kbKey = name;
        document.getElementById('kb-selected-label').textContent = label;
      });
      rowEl.appendChild(btn);
    });
    grid.appendChild(rowEl);
  });

  const getKbRgb = wireSliders('kb-r', 'kb-g', 'kb-b', 'kb-swatch');

  document.getElementById('btn-kb-apply').addEventListener('click', () => {
    if (!state.kbKey) return;
    const [r,g,b] = getKbRgb();
    runDaemon(['set-keyboard-key', '--key', state.kbKey, '--red', r, '--green', g, '--blue', b]);
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

  wireSliders('mk-r', 'mk-g', 'mk-b', 'mk-swatch');

  // Apply zone
  document.getElementById('btn-mk-apply-zone').addEventListener('click', () => {
    if (state.mkEmitter === null) return;
    const r = +document.getElementById('mk-r').value;
    const g = +document.getElementById('mk-g').value;
    const b = +document.getElementById('mk-b').value;
    state.emitterColors[state.mkEmitter] = [r,g,b];
    setEmitterSvg(state.mkEmitter, r, g, b);
    // Build zones for the key
    const m = EMITTER_META[state.mkEmitter];
    const base = {'w':0,'a':3,'s':6,'d':9}[m.key];
    const [lr,lg,lb] = state.emitterColors[base];
    const [tr,tg,tb] = state.emitterColors[base+1];
    const [rr,rg,rb] = state.emitterColors[base+2];
    runDaemon([
      'set-magkey-zones', '--key', m.key,
      '--left',  `${lr},${lg},${lb}`,
      '--top',   `${tr},${tg},${tb}`,
      '--right', `${rr},${rg},${rb}`,
    ]);
  });

  // Apply whole key
  document.getElementById('btn-mk-apply-key').addEventListener('click', () => {
    if (state.mkEmitter === null) return;
    const r = +document.getElementById('mk-r').value;
    const g = +document.getElementById('mk-g').value;
    const b = +document.getElementById('mk-b').value;
    const m = EMITTER_META[state.mkEmitter];
    const base = {'w':0,'a':3,'s':6,'d':9}[m.key];
    state.emitterColors[base] = state.emitterColors[base+1] = state.emitterColors[base+2] = [r,g,b];
    updateAllEmitterSvg();
    runDaemon(['set-magkeys-pattern',
      '--w', state.emitterColors.slice(0,3).map(c=>c.join(',')).join(' '),
      '--a', state.emitterColors.slice(3,6).map(c=>c.join(',')).join(' '),
      '--s', state.emitterColors.slice(6,9).map(c=>c.join(',')).join(' '),
      '--d', state.emitterColors.slice(9,12).map(c=>c.join(',')).join(' '),
    ]);
    // Simpler: use set-magkey-whole-key
    runDaemon(['set-magkey-whole-key', '--key', m.key, '--color',
      r===255&&g===0&&b===0?'red' : r===0&&g===255&&b===0?'green' : r===0&&g===0&&b===255?'blue' : 'off'
    ]);
  });

  // Apply all keys
  document.getElementById('btn-mk-apply-all').addEventListener('click', () => {
    const r = +document.getElementById('mk-r').value;
    const g = +document.getElementById('mk-g').value;
    const b = +document.getElementById('mk-b').value;
    state.emitterColors = Array.from({length:12}, () => [r,g,b]);
    updateAllEmitterSvg();
    runDaemon(['set-magkeys', '--all', `${r},${g},${b}`]);
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
  initBackend();
  setStatus('Ready');
});
