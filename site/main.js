'use strict';

// Picks the download for the visitor's system and shows the latest release. The page works
// without this script: the button then downloads the Windows archive and the links offer the rest.
(() => {
  const REPO = 'montesgp/mini-browser';
  const DOWNLOAD = `https://github.com/${REPO}/releases/latest/download/`;
  const SYSTEMS = {
    windows: { name: 'Windows', detail: '64-bit · .zip', file: 'mini-browser-windows-x64.zip' },
    macos: { name: 'macOS', detail: 'Apple Silicon · .zip', file: 'mini-browser-macos-arm64.zip' },
    linux: { name: 'Linux', detail: 'x64 · .tar.gz', file: 'mini-browser-linux-x64.tar.gz' },
  };
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

  function showSystem(key) {
    const system = SYSTEMS[key];
    const button = $('download');
    button.href = DOWNLOAD + system.file;
    $('download-os').textContent = system.name;
    $('download-detail').textContent = system.detail;
    const others = Object.keys(SYSTEMS).filter((k) => k !== key);
    Object.keys(SYSTEMS).forEach((k) => {
      $(`link-${k}`).hidden = k === key;
      // A separator follows every shown link except the last one.
      const separator = $(`sep-${k}`);
      if (separator) separator.hidden = k === key || k === others[others.length - 1];
    });
    Object.keys(SYSTEMS).forEach((k) => { $(`help-${k}`).open = k === key; });
  }

  async function showLatestRelease() {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 8000);
    try {
      const response = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
        headers: { Accept: 'application/vnd.github+json' },
        signal: controller.signal,
      });
      if (!response.ok) return;
      const release = await response.json();
      const tag = typeof release.tag_name === 'string' ? release.tag_name : '';
      const url = typeof release.html_url === 'string' ? release.html_url : '';
      if (!/^v\d+\.\d+\.\d+$/.test(tag) || !url.startsWith(`https://github.com/${REPO}/releases/`)) return;
      $('release-tag').textContent = tag;
      $('release-link').href = url;
      const date = new Date(release.published_at);
      if (!Number.isNaN(date.getTime())) {
        const day = date.toLocaleDateString('en', { year: 'numeric', month: 'short', day: 'numeric', timeZone: 'UTC' });
        $('release-date').textContent = ` · ${day}`;
      }
      $('release').hidden = false;
    } catch {
      // Offline, rate-limited or blocked: the buttons still download the latest release.
    } finally {
      clearTimeout(timer);
    }
  }

  showSystem(detectSystem() || 'windows');
  showLatestRelease();
})();
