'use strict';

// Picks the download for the visitor's system and shows the latest release. The page works
// without this script: the button then downloads the Windows archive and the links offer the rest.
(() => {
  const REPO = 'montesgp/mini-browser';
  const DOWNLOAD = `https://github.com/${REPO}/releases/latest/download/`;
  const SYSTEMS = {
    windows: { name: 'Windows', detail: '64-bit · .zip', file: 'mini-browser-windows-x64.zip', help: 'windows' },
    macos: { name: 'macOS', detail: 'Apple Silicon · .zip', file: 'mini-browser-macos-arm64.zip', help: 'macos' },
    'macos-intel': { name: 'macOS', detail: 'Intel · .zip', file: 'mini-browser-macos-x64.zip', help: 'macos' },
    linux: { name: 'Linux', detail: 'x64 · .tar.gz', file: 'mini-browser-linux-x64.tar.gz', help: 'linux' },
  };
  const HELP = ['windows', 'macos', 'linux'];
  // Systems the latest release has a file for: all of them until the release says otherwise.
  let available = Object.keys(SYSTEMS);
  const $ = (id) => document.getElementById(id);

  /** `windows`, `macos`, `linux`, or null for phones, tablets and anything unknown. */
  function detectSystem() {
    const hints = navigator.userAgentData;
    if (hints && hints.mobile) return null;
    // Client hints name the platform exactly; the user agent string is the fallback (Safari, Firefox).
    const text = ((hints && hints.platform) || navigator.userAgent || '').toLowerCase();
    if (/android|iphone|ipad|ipod|cros|chrome os/.test(text)) return null;
    // macOS first: "Darwin" contains "win".
    // iPads also say "Macintosh"; Macs have no touch screen.
    if (/mac/.test(text)) return navigator.maxTouchPoints > 1 ? null : 'macos';
    if (/win/.test(text)) return 'windows';
    if (/linux|x11/.test(text)) return 'linux';
    return null;
  }

  /**
   * For a Mac: `macos-intel` when the browser shows an Intel Mac, else `macos` (Apple Silicon,
   * every Mac sold since 2020). The user agent says "Intel Mac OS X" on both, so look further.
   */
  async function detectMac() {
    // The WebGL renderer names the GPU ("Apple M1 Pro", "Intel Iris Plus", "AMD Radeon Pro"), and
    // goes first: an Intel browser running under Rosetta still sees the Apple GPU.
    // Safari may only say "Apple GPU" on any Mac, which decides nothing.
    try {
      const gl = document.createElement('canvas').getContext('webgl');
      const info = gl && gl.getExtension('WEBGL_debug_renderer_info');
      const renderer = info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : '';
      if (/apple m\d/i.test(renderer)) return 'macos';
      if (/intel|amd|radeon|nvidia/i.test(renderer)) return 'macos-intel';
    } catch {
      // No WebGL: try the client hints.
    }
    // Chromium browsers report the CPU architecture.
    const hints = navigator.userAgentData;
    if (hints && hints.getHighEntropyValues) {
      try {
        const { architecture } = await hints.getHighEntropyValues(['architecture']);
        if (architecture === 'x86') return 'macos-intel';
      } catch {
        // Refused: keep the default.
      }
    }
    return 'macos';
  }

  function showSystem(key) {
    const system = SYSTEMS[key];
    const button = $('download');
    button.href = DOWNLOAD + system.file;
    $('download-os').textContent = system.name;
    $('download-detail').textContent = system.detail;
    const others = Object.keys(SYSTEMS).filter((k) => k !== key && available.includes(k));
    Object.keys(SYSTEMS).forEach((k) => {
      $(`link-${k}`).hidden = !others.includes(k);
      // A separator follows every shown link except the last one.
      const separator = $(`sep-${k}`);
      if (separator) separator.hidden = !others.includes(k) || k === others[others.length - 1];
    });
    HELP.forEach((h) => { $(`help-${h}`).open = h === system.help; });
  }

  /** File names of the latest release, or null when it cannot be read. */

  async function showLatestRelease() {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 8000);
    try {
      const response = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
        headers: { Accept: 'application/vnd.github+json' },
        signal: controller.signal,
      });
      if (!response.ok) return null;
      const release = await response.json();
      const tag = typeof release.tag_name === 'string' ? release.tag_name : '';
      const url = typeof release.html_url === 'string' ? release.html_url : '';
      if (!/^v\d+\.\d+\.\d+$/.test(tag) || !url.startsWith(`https://github.com/${REPO}/releases/`)) return null;
      $('release-tag').textContent = tag;
      $('release-link').href = url;
      const date = new Date(release.published_at);
      if (!Number.isNaN(date.getTime())) {
        const day = date.toLocaleDateString('en', { year: 'numeric', month: 'short', day: 'numeric', timeZone: 'UTC' });
        $('release-date').textContent = ` · ${day}`;
      }
      $('release').hidden = false;
      return Array.isArray(release.assets) ? release.assets.map((asset) => asset && asset.name) : null;
    } catch {
      // Offline, rate-limited or blocked: the buttons still download the latest release.
      return null;
    } finally {
      clearTimeout(timer);
    }
  }

  const initial = detectSystem() || 'windows';
  showSystem(initial);
  // Then tell Apple Silicon from Intel, and hide what the latest release has no file for
  // (releases before the Intel build); if the release cannot be read, offer everything.
  Promise.all([initial === 'macos' ? detectMac() : initial, showLatestRelease()]).then(([key, assets]) => {
    if (assets) available = Object.keys(SYSTEMS).filter((k) => assets.includes(SYSTEMS[k].file));
    showSystem(available.includes(key) ? key : initial);
  });
})();
