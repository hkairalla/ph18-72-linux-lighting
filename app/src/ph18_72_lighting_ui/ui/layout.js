/* Physical keyboard layout of the Acer Predator Helios 18 (PH18-72).

   Units: 1 = one standard key. The board is 19 units wide: a 15-unit main block plus a
   4-unit numpad, and 6 units tall (a 0.78-unit function row, then five full rows). Positions
   were measured from a photo of the real keyboard, so widths match the keycaps:
   Backspace 2, Tab 1.5, Caps 1.75, Enter 2.25, left Shift 2.25, right Shift 1.75 (the Up arrow
   fills the rest of that row), Ctrl 1.25, Space 5.25, Copilot 1.5, backslash 1.5, numpad Enter
   2 tall. The Right arrow sits in the numpad's first column, and the Predator key is the
   numpad's first column on the number row.

   `name` is the daemon's key name (daemon/src/main.rs `keyboard_key_index`): never change a
   name without changing the daemon. `label` is the plain-text name shown in the sidebar.
   `lg` is the printed legend:
     main  the character (defaults to the label)   top  the shifted symbol above it
     sub   a small second legend below it          icon an entry of the sprite in index.html
     fn    the small "F1" printed above an icon
   kind:'magkey' marks the four Hall-effect keys (W A S D): not editable on this page. */

const KEYS = (() => {
  const keys = [];
  const add = (name, label, x, y, w, h, lg = {}, extra = {}) =>
    keys.push({ name, label, x, y, w, h, lg, ...extra });

  // A row is a list of [name, label, width, legend?, extra?]; a bare number is a gap.
  const row = (y, x0, specs, h = 1) => {
    let x = x0;
    for (const s of specs) {
      if (typeof s === 'number') { x += s; continue; }
      const [name, label, w, lg, extra] = s;
      add(name, label, x, y, w, h, lg, extra);
      x += w;
    }
  };
  const L = (ch, top) => [ch, ch.toUpperCase(), 1, top ? { top, main: ch.toUpperCase() } : {}];
  const letter = ch => [ch, ch.toUpperCase(), 1, {}, ['w', 'a', 's', 'd'].includes(ch) ? { kind: 'magkey' } : {}];

  // ── Function row ──
  // Measured from a photo, then corrected: the photo is taken at an angle, so upper rows sit a little
  // left of where they really are (about 0.17 of a key above the number row). Checked against the
  // physical constraints: Del ends exactly on the main block's right edge (15.0), and the four
  // media/power keys are centred over the four numpad columns beneath them.
  const FN_W = 0.75, FN_H = 0.8, FN_Y = 0.12;
  const fn = (name, label, x, lg, w = FN_W) => add(name, label, x, FN_Y, w, FN_H, lg, { fcls: 'f-key' });
  fn('esc', 'Esc', 0.4, { main: 'Esc' });
  fn('f1',  'F1',  1.5,  { fn: 'F1',  icon: 'mute' });
  fn('f2',  'F2',  2.4,  { fn: 'F2',  icon: 'vol-down' });
  fn('f3',  'F3',  3.3,  { fn: 'F3',  icon: 'vol-up' });
  fn('f4',  'F4',  4.2,  { fn: 'F4',  icon: 'mic-mute' });
  fn('f5',  'F5',  5.25, { fn: 'F5',  icon: 'bright-down' });
  fn('f6',  'F6',  6.15, { fn: 'F6',  icon: 'bright-up' });
  fn('f7',  'F7',  7.05, { fn: 'F7',  icon: 'display' });
  fn('f8',  'F8',  7.95, { fn: 'F8',  icon: 'cast' });
  fn('f9',  'F9',  9.0,  { fn: 'F9',  icon: 'airplane' });
  fn('f10', 'F10', 9.9,  { fn: 'F10', icon: 'touchpad-off' });
  fn('f11', 'F11', 10.8, { fn: 'F11', icon: 'kbd-down' });
  fn('f12', 'F12', 11.7, { fn: 'F12', icon: 'kbd-up' });
  fn('print_screen', 'Print Screen', 12.65, { fn: 'PrtSc', icon: 'crop' });
  fn('insert', 'Insert', 13.45, { main: 'Ins' });
  fn('delete', 'Delete', 14.25, { main: 'Del' });
  // Media / power: each centred over its numpad column (columns start at x = 15, 16, 17, 18).
  const MW = 0.88, MX = (col) => col + (1 - MW) / 2;
  fn('media_prev',       'Previous track', MX(15), { icon: 'prev', fill: true }, MW);
  fn('media_play_pause', 'Play / pause',   MX(16), { icon: 'play-pause', fill: true }, MW);
  fn('media_next',       'Next track',     MX(17), { icon: 'next', fill: true }, MW);
  fn('power',            'Power',          MX(18), { icon: 'power' }, MW);

  // ── Number row ──
  row(1, 0, [
    ['grave', 'Grave', 1, { top: '~', main: '`' }],
    ['1', '1', 1, { top: '!', main: '1' }], ['2', '2', 1, { top: '@', main: '2' }],
    ['3', '3', 1, { top: '#', main: '3' }], ['4', '4', 1, { top: '$', main: '4' }],
    ['5', '5', 1, { top: '%', main: '5', corner: '€' }], ['6', '6', 1, { top: '^', main: '6' }],
    ['7', '7', 1, { top: '&', main: '7' }], ['8', '8', 1, { top: '*', main: '8' }],
    ['9', '9', 1, { top: '(', main: '9' }], ['0', '0', 1, { top: ')', main: '0' }],
    ['minus', 'Minus', 1, { top: '_', main: '-' }], ['equal', 'Equal', 1, { top: '+', main: '=' }],
    ['backspace', 'Backspace', 2, { main: 'Backspace', arrow: '←' }],
    ['predator_sense', 'Predator key', 1, { icon: 'predator' }],
    ['keypad_num_lock', 'Num Lock', 1, { main: 'NumLk' }],
    ['keypad_divide', 'Keypad /', 1, { main: '/' }],
    ['keypad_multiply', 'Keypad *', 1, { main: '*' }],
  ]);

  // ── QWERTY row ──
  row(2, 0, [
    ['tab', 'Tab', 1.5, { main: 'Tab', icon: 'tab' }],
    letter('q'), letter('w'), letter('e'), letter('r'), letter('t'), letter('y'), letter('u'), letter('i'), letter('o'), letter('p'),
    ['left_bracket', 'Left bracket', 1, { top: '{', main: '[' }],
    ['right_bracket', 'Right bracket', 1, { top: '}', main: ']' }],
    ['backslash', 'Backslash', 1.5, { top: '|', main: '\\' }],
    ['keypad_7', 'Keypad 7', 1, { main: '7', sub: 'Home' }],
    ['keypad_8', 'Keypad 8', 1, { main: '8', sub: '▲' }],
    ['keypad_9', 'Keypad 9', 1, { main: '9', sub: 'PgUp' }],
    ['keypad_minus', 'Keypad -', 1, { main: '-' }],
  ]);

  // ── Home row ──
  row(3, 0, [
    ['caps_lock', 'Caps Lock', 1.75, { main: 'Caps Lock' }],
    letter('a'), letter('s'), letter('d'), letter('f'), letter('g'), letter('h'), letter('j'), letter('k'), letter('l'),
    ['semicolon', 'Semicolon', 1, { top: ':', main: ';' }],
    ['apostrophe', 'Apostrophe', 1, { top: '"', main: "'" }],
    ['enter', 'Enter', 2.25, { main: 'Enter', arrow: '↵' }],
    ['keypad_4', 'Keypad 4', 1, { main: '4', sub: '◀' }],
    ['keypad_5', 'Keypad 5', 1, { main: '5' }],
    ['keypad_6', 'Keypad 6', 1, { main: '6', sub: '▶' }],
    ['keypad_plus', 'Keypad +', 1, { main: '+' }],
  ]);

  // ── Bottom letter row (right Shift is short: the Up arrow takes the last slot) ──
  row(4, 0, [
    ['left_shift', 'Left Shift', 2.25, { main: 'Shift', arrow: '⇧' }],
    letter('z'), letter('x'), letter('c'), letter('v'), letter('b'), letter('n'), letter('m'),
    ['comma', 'Comma', 1, { top: '<', main: ',' }],
    ['period', 'Period', 1, { top: '>', main: '.' }],
    ['slash', 'Slash', 1, { top: '?', main: '/' }],
    ['right_shift', 'Right Shift', 1.75, { main: 'Shift', arrow: '⇧' }],
    ['arrow_up', 'Up', 1, { main: '▲' }],
    ['keypad_1', 'Keypad 1', 1, { main: '1', sub: 'End' }],
    ['keypad_2', 'Keypad 2', 1, { main: '2', sub: '▼' }],
    ['keypad_3', 'Keypad 3', 1, { main: '3', sub: 'PgDn' }],
  ]);
  add('keypad_enter', 'Keypad Enter', 18, 4, 1, 2, { main: 'Enter' });

  // ── Space row ──
  row(5, 0, [
    ['left_ctrl', 'Left Ctrl', 1.25, { main: 'Ctrl' }],
    ['fn', 'Fn', 1, { main: 'Fn' }],
    ['left_windows', 'Windows', 1, { icon: 'windows', fill: true }],
    ['left_alt', 'Left Alt', 1, { main: 'Alt' }],
    ['space', 'Space', 5.25, { main: '' , bar: true }],
    ['right_alt', 'Alt Gr', 1, { main: 'Alt Gr' }],
    ['menu', 'Menu', 1, { icon: 'menu' }],
    ['copilot', 'Copilot', 1.5, { icon: 'copilot' }],
    ['arrow_left', 'Left', 1, { main: '◀' }],
    ['arrow_down', 'Down', 1, { main: '▼' }],
    ['arrow_right', 'Right', 1, { main: '▶' }],
    ['keypad_0', 'Keypad 0', 1, { main: '0', sub: 'Ins' }],
    ['keypad_decimal', 'Keypad .', 1, { main: '.', sub: 'Del' }],
  ]);

  return keys;
})();

/* Board size in key units, for CSS. */
const BOARD_W = 19, BOARD_H = 6;
