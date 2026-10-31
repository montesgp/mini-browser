# Third-party notices

mini-browser bundles and builds on third-party software. Their licenses apply to those parts, independently of mini-browser's own `MIT OR Apache-2.0` license.

## Chromium and its dependencies

Every release archive contains `CREDITS.html`, shipped with CEF, which lists Chromium and each of its third-party components with their full license texts.

## cef-rs

The app layout (`crates/app`) and the bundling tool (`xtask`) are derived from the `cefsimple` example and the `bundle-cef-app` tool of [cef-rs](https://github.com/tauri-apps/cef-rs), licensed `Apache-2.0 OR MIT`. The `cef` crate itself is a dependency under the same terms.

## Rust crates

Other Rust dependencies are listed in `Cargo.lock`; each crate's license is declared in its own package metadata on [crates.io](https://crates.io/).

## Chromium Embedded Framework (CEF)

Source: <https://bitbucket.org/chromiumembedded/cef> — BSD-3-Clause:

```text
Copyright (c) 2008-2020 Marshall A. Greenblatt. Portions Copyright (c)
2006-2009 Google Inc. All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

   * Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.
   * Redistributions in binary form must reproduce the above
copyright notice, this list of conditions and the following disclaimer
in the documentation and/or other materials provided with the
distribution.
   * Neither the name of Google Inc. nor the name Chromium Embedded
Framework nor the names of its contributors may be used to endorse
or promote products derived from this software without specific prior
written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```
