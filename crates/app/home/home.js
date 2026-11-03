// The home page of mini-browser. It asks the app to run commands with same-origin requests to
// `mini://home/cmd/<name>`; the app answers each one with the current state.
'use strict';

(() => {
  const CARD_OPEN_KEY = 'cardOpen';

  const field = document.getElementById('address');
  const toggle = document.getElementById('toggle');
  const card = document.getElementById('card');
  const rows = document.getElementById('rows');

  let state = {
    platform: /Mac/.test(navigator.platform) ? 'macos' : 'windows',
    padding: null,
    topBar: null,
  };

  /** Asks the app for `path` (`state` or `cmd/<name>?…`) and shows the state it returns. */
  async function request(path) {
    try {
      const response = await fetch(`/${path}`, { cache: 'no-store' });
      if (response.ok) {
        state = await response.json();
        render();
      }
    } catch {
      // The tab is closing or the app refused the request: nothing to show.
    }
  }

  const run = (name) => request(`cmd/${name}`);

  function focusAddress() {
    field.focus();
    field.select();
  }
  // The app calls it for `Ctrl/Cmd+L` on this page.
  window.miniFocusAddress = focusAddress;

  /** The rows of the card: keys, action and what a press does. */
  function shortcuts() {
    const mac = state.platform === 'macos';
    const primary = mac ? 'Cmd' : 'Ctrl';
    const mode = state.topBar === 'pinned' ? 'Pinned' : state.topBar === 'auto' ? 'Auto' : '…';
    return [
      { keys: `${primary}+T`, label: 'New tab', press: () => run('new-tab') },
      { keys: `${primary}+L`, label: 'Enter address', press: focusAddress },
      { keys: `${primary}+W`, label: 'Close tab', press: () => run('close-tab') },
      { keys: 'Ctrl+Tab', label: 'Next tab', press: () => run('next-tab') },
      { keys: 'Ctrl+Shift+Tab', label: 'Previous tab', press: () => run('previous-tab') },
      { keys: mac ? 'Cmd+[' : 'Alt+Left', label: 'Back', press: () => run('back') },
      { keys: mac ? 'Cmd+]' : 'Alt+Right', label: 'Forward', press: () => run('forward') },
      { keys: `${primary}+R`, label: 'Reload', press: () => run('reload') },
      {
        keys: `${primary}+Shift+F`,
        label: 'Top bar',
        mode,
        press: () => run('top-bar'),
      },
      { keys: `${primary}+,`, label: 'Padding', padding: true },
      { keys: `${primary}+Q`, label: 'Quit', press: () => run('quit') },
    ];
  }

  function element(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  }

  /** The padding row's `Padding − 8 px +` controls; `index` keeps their focus across renders. */
  function paddingControls(index) {
    const stepper = element('span', 'stepper');
    const minus = element('button', '', '\u2212');
    const plus = element('button', '', '+');
    const value = element('output', '', state.padding === null ? '…' : `${state.padding} px`);
    minus.type = plus.type = 'button';
    minus.setAttribute('aria-label', 'Decrease padding');
    plus.setAttribute('aria-label', 'Increase padding');
    value.setAttribute('aria-live', 'polite');
    value.setAttribute('aria-label', 'Padding');
    minus.addEventListener('click', () => run('padding?step=-1'));
    plus.addEventListener('click', () => run('padding?step=1'));
    minus.dataset.focusId = `${index}-minus`;
    plus.dataset.focusId = `${index}-plus`;
    stepper.append(element('span', 'label', 'Padding'), minus, value, plus);
    return stepper;
  }

  /** Draws the rows again, keeping the keyboard focus on the same control. */
  function render() {
    const focused = document.activeElement;
    const focusId = focused && focused.dataset ? focused.dataset.focusId : undefined;
    rows.replaceChildren();
    shortcuts().forEach((shortcut, index) => {
      const row = element(shortcut.padding ? 'div' : 'button', 'row');
      row.append(element('kbd', '', shortcut.keys));
      if (shortcut.padding) {
        row.append(paddingControls(index));
      } else {
        const label = element('span', 'label', shortcut.label);
        if (shortcut.mode) {
          label.append(': ', element('span', 'mode', shortcut.mode));
        }
        row.append(label);
        row.type = 'button';
        row.dataset.focusId = String(index);
        const mode = shortcut.mode ? ` (${shortcut.mode})` : '';
        row.setAttribute('aria-label', `${shortcut.label}${mode}, ${shortcut.keys}`);
        row.addEventListener('click', shortcut.press);
      }
      rows.append(row);
    });
    if (focusId !== undefined) {
      const again = rows.querySelector(`[data-focus-id="${focusId}"]`);
      if (again) again.focus();
    }
  }

  function setCardOpen(open) {
    card.hidden = !open;
    toggle.setAttribute('aria-expanded', String(open));
    toggle.setAttribute('aria-label', open ? 'Hide shortcuts' : 'Show shortcuts');
    try {
      localStorage.setItem(CARD_OPEN_KEY, open ? '1' : '0');
    } catch {
      // Storage unavailable: the card still toggles for this page.
    }
  }

  field.addEventListener('keydown', (event) => {
    if (event.key !== 'Enter' || event.isComposing) return;
    event.preventDefault();
    const input = field.value.trim();
    if (input) run(`go?input=${encodeURIComponent(input)}`);
  });

  toggle.addEventListener('click', () => setCardOpen(card.hidden));

  // Settings can change elsewhere (the settings box, another tab): refresh when shown again.
  window.addEventListener('focus', () => request('state'));
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') request('state');
  });

  let open = false;
  try {
    open = localStorage.getItem(CARD_OPEN_KEY) === '1';
  } catch {
    open = false;
  }
  setCardOpen(open);
  render();
  request('state');
  focusAddress();
})();
