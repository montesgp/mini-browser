#!/usr/bin/env bash
# Smoke test for a built bundle: launches the app on a real page, checks that it is still
# running with its helper processes after a while, saves a screenshot, and on Linux checks
# that Chromium's sandbox is active. Usage: smoke-test.sh <bundle-dir> <output-dir>
# Set SMOKE_NO_SCREENSHOT=1 on a personal machine to avoid capturing the whole screen.
set -euo pipefail

bundle="$(cd "${1:?bundle dir}" && pwd)"
out="${2:?output dir}"
mkdir -p "${out}"
url="https://example.com/"
settle=20

fail() { echo "::error::$*"; exit 1; }

case "$(uname -s)" in
  Linux*)
    # Ubuntu 24.04 restricts unprivileged user namespaces, so use Chromium's setuid sandbox
    # helper, as the README tells users to do. Never --no-sandbox.
    sudo chown root:root "${bundle}/chrome-sandbox"
    sudo chmod 4755 "${bundle}/chrome-sandbox"
    Xvfb :99 -screen 0 1280x800x24 >/dev/null 2>&1 &
    export DISPLAY=:99
    sleep 2
    "${bundle}/minibrowser" --url="${url}" >"${out}/app.log" 2>&1 &
    app=$!
    sleep "${settle}"
    kill -0 "${app}" 2>/dev/null || { tail -50 "${out}/app.log"; fail "app exited early"; }
    procs=$(pgrep -f "${bundle}/minibrowser" | wc -l)
    sandboxed=0
    for p in $(pgrep -f "${bundle}/minibrowser"); do
      if [ "$(awk '/^Seccomp:/{print $2}' "/proc/${p}/status" 2>/dev/null)" = "2" ]; then
        sandboxed=$((sandboxed + 1))
      fi
    done
    echo "processes=${procs} seccomp-filtered=${sandboxed}"
    if [ -z "${SMOKE_NO_SCREENSHOT:-}" ] && command -v import >/dev/null; then import -window root "${out}/screen.png" || true; fi
    kill "${app}" 2>/dev/null || true
    pkill -f "${bundle}/minibrowser" 2>/dev/null || true
    [ "${procs}" -ge 3 ] || fail "expected browser, GPU and renderer processes, found ${procs}"
    [ "${sandboxed}" -ge 1 ] || fail "no seccomp-sandboxed child process: the sandbox is not active"
    ;;
  Darwin*)
    app_bundle="$(ls -d "${bundle}"/*.app | head -1)"
    open -n "${app_bundle}" --args --url="${url}"
    sleep "${settle}"
    procs=$(pgrep -f "${app_bundle}/Contents" | wc -l | tr -d ' ')
    echo "processes=${procs}"
    [ -n "${SMOKE_NO_SCREENSHOT:-}" ] || screencapture -x "${out}/screen.png" || true
    pkill -f "${app_bundle}/Contents" 2>/dev/null || true
    [ "${procs}" -ge 3 ] || fail "expected browser and helper processes, found ${procs}"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    "${bundle}/minibrowser.exe" --url="${url}" >"${out}/app.log" 2>&1 &
    sleep "${settle}"
    procs=$(tasklist //FI "IMAGENAME eq minibrowser.exe" | grep -c minibrowser || true)
    echo "processes=${procs}"
    [ -n "${SMOKE_NO_SCREENSHOT:-}" ] || powershell -NoProfile -Command "Add-Type -AssemblyName System.Windows.Forms,System.Drawing; \$b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \$i=New-Object System.Drawing.Bitmap \$b.Width,\$b.Height; [System.Drawing.Graphics]::FromImage(\$i).CopyFromScreen(\$b.Location,[System.Drawing.Point]::Empty,\$b.Size); \$i.Save('${out}\screen.png')" || true
    taskkill //IM minibrowser.exe //F >/dev/null 2>&1 || true
    [ "${procs}" -ge 3 ] || fail "expected browser, GPU and renderer processes, found ${procs}"
    ;;
  *) fail "unsupported OS $(uname -s)" ;;
esac
echo "smoke test passed"
