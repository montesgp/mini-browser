# Security policy

## Supported versions

Only the [latest release](https://github.com/montesgp/mini-browser/releases/latest) receives fixes. mini-browser bundles Chromium, so Chromium security updates ship as new mini-browser releases; always run the latest one.

## Reporting a vulnerability

Please report vulnerabilities **privately**, never in a public issue, pull request or discussion:

1. Open the repository's [private vulnerability report form](https://github.com/montesgp/mini-browser/security/advisories/new) (Security → Report a vulnerability).
2. Describe the problem, the affected version and platform, and the steps to reproduce it. A proof of concept helps.

The maintainer will acknowledge the report as soon as possible, keep you informed while a fix is prepared, and credit you in the advisory unless you prefer otherwise.

## Scope

In scope: mini-browser's own code, for example the address box letting a non-`http(s)` scheme load, the sandbox or profile isolation being weakened, or the release pipeline publishing tampered builds.

Out of scope, please report upstream instead:

- Vulnerabilities in Chromium itself: [Chromium security](https://www.chromium.org/Home/chromium-security/reporting-security-bugs/). Once fixed upstream, mini-browser picks them up through a CEF update.
- Vulnerabilities in CEF: [CEF issue tracker](https://github.com/chromiumembedded/cef/issues).

## Verifying downloads

Every release archive is listed in `SHA256SUMS.txt` and carries a signed build-provenance attestation. Verify with:

```sh
gh attestation verify <archive> -R montesgp/mini-browser
```
