# Security Policy

Colony is an application launcher and store: it downloads and executes
binaries. Security reports are taken seriously and handled with priority.

## Supported versions

Only the **latest release** receives security fixes. Colony self-updates (and
the `colony-bin` AUR package tracks releases automatically), so staying current
is one update away.

| Version        | Supported |
| -------------- | --------- |
| latest release | ✅        |
| anything older | ❌        |

## Reporting a vulnerability

Please report vulnerabilities **privately** via
[GitHub Security Advisories](https://github.com/Project-Colony/Colony/security/advisories/new)
("Report a vulnerability"). Do not open a public issue for exploitable bugs.

What to expect:

- **Acknowledgement** within a few days.
- A fix (or a mitigation plan) before any public disclosure, coordinated with
  you. Given the project's release automation, a patched release usually ships
  as soon as the fix lands.
- Credit in the release notes if you want it.

## Scope and trust model

Reports of particular interest:

- **Self-update chain**: the launcher only applies updates carrying a valid
  ed25519 detached signature (`<asset>.sig`) verified against the public key
  embedded in the binary (`src/signing.rs`). Verification is fail-closed.
  Anything that bypasses or weakens this is critical.
- **App installs**: archive extraction (zip-slip, symlink escapes), path
  traversal via manifest fields, and launch-path redirection are guarded -
  holes in those guards are high severity.
- **Manifest trust**: `colony.json` files come from third-party repositories
  and are treated as untrusted input.

Out of scope: vulnerabilities in the applications Colony installs (report
those to the respective projects), and denial-of-service against the GitHub
API rate limit.
