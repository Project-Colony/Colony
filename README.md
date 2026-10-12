<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/colony-logo-512.png">
  <img src="assets/brand/png/colony-logo-light-1024.png" alt="Colony" width="360">
</picture>

**The launcher and app store for Project Colony: browse, install, update and launch every Colony app from one window, on Linux, Windows and macOS.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Platforms](https://img.shields.io/badge/platforms-linux%20%7C%20windows%20%7C%20macOS-lightgrey)](#installation)
[![AUR: colony-bin](https://img.shields.io/badge/AUR-colony--bin-blue)](https://aur.archlinux.org/packages/colony-bin)
[![AUR: colony-git](https://img.shields.io/badge/AUR-colony--git-blue)](https://aur.archlinux.org/packages/colony-git)

[Project Colony](https://github.com/Project-Colony) ships small desktop programs,
one GitHub repository and one set of release files each. Getting them by hand
means watching every repository, picking the right file for your platform,
checking its signature, and doing it all again at every update. Colony does that
for the whole organisation: it lists every app, installs the build for your
platform once its signature checks out, keeps it up to date, and launches it next
to the applications already on your system.

> **Status:** Colony is developed and used on Linux, where it installs, updates
> and launches the Project Colony apps. Its self-update is fail-closed: it
> applies only a release whose ed25519 signature and signed `.meta` check out,
> and never an older version than the one running. CI builds and tests every
> change on Linux, Windows and macOS, and checks that it still builds with Rust
> 1.88. The Windows and macOS builds work but are best-effort (see
> [Platforms](#platforms)). Releases now go through the organisation's shared
> sign-and-publish workflow, which has not yet published a Colony release: the
> latest, v0.10.4, was signed by the workflow it replaced. Authenticode signing
> of the Windows build through SignPath is applied for and not active yet. Every
> Project Colony program's release downloads the latest `colony-linux`, verifies
> its signatures and runs its `validate-manifest`, so a broken Colony release
> blocks the releases of every other program.

## Why Colony

Without it, each Colony app is a GitHub release page to check by hand. With it:

- **One catalog, nothing to register.** Every repository of the Project-Colony
  organisation with a `colony.json` at its root appears in Colony, with its
  README, changelog and licence, as soon as it publishes a release.
- **Checked before it runs.** Colony verifies a release's ed25519 signature and
  its signed `.meta`, which binds the file to its name, digest and version, and
  refuses a release older than the one installed. Once an app has installed
  with a valid signature, a later unsigned release of it is refused.
- **Every update in one place.** Colony compares each installed app, and itself,
  with its latest release, and updates in one click. An interrupted download
  resumes where it stopped.
- **Your other applications too.** It lists the `.desktop` entries on Linux, the
  Start Menu on Windows and `/Applications` on macOS, so everything launches
  from the same grid.
- **No account needed.** Signing in with GitHub is optional and only raises the
  API rate limit.

```
Project-Colony on GitHub             Colony                         your machine
  <app>/colony.json        -->  catalog, README, changelog
  <app> release files      -->  check .sig and signed .meta  -->  Colony/apps/<App>/  -->  launch
  Colony release           -->  self-update, never older     -->  colony (replaced in place)
```

## What it does

- **Discover**: browse the catalog by category, search by name, and read each
  app's README, changelog and licence without leaving Colony.
- **Install and update**: one click installs the build for your platform, one
  click updates it. A badge shows when a new version is out, for apps and for
  Colony itself.
- **Launch**: Colony apps and the applications already installed on your system,
  side by side. On Linux, every app Colony installs also gets a menu entry, so
  your desktop's own launcher finds it too.
- **Self-update**: Colony downloads its new version, verifies it, swaps itself in
  and rolls back if the swap fails. An AUR install is updated by pacman instead.
- **Preferences**: click **Colony** at the top of the sidebar for the theme and
  accent colour, text size, high contrast, a dyslexia-friendly font, reduced
  motion, English or French, keyboard navigation, and the storage and update
  settings.

New to Colony? The [tutorial](docs/guide/tutorial.md) walks through a first
install.

## Installation

### Arch Linux (AUR)

Two packages, pick **one**:

| Package | Install | Notes |
|---|---|---|
| `colony-bin` | `paru -S colony-bin` | The released binary, about 30 MB to download. Updated at every Colony release. |
| `colony-git` | `paru -S colony-git` | Built from the latest commit, about 5 minutes. Rebuilt on `paru -Syu` when the repository has moved. |

Both install `/usr/bin/colony` and a `colony.desktop` entry, so GNOME, KDE,
rofi and wofi find it.

### Direct binary download

Grab the file for your platform from the
[latest release](https://github.com/Project-Colony/Colony/releases/latest):

| Platform | Asset |
|---|---|
| Linux (x86_64) | `colony-linux` |
| Windows (x86_64) | `colony-windows.exe` |
| macOS (Apple Silicon) | `colony-macos` |
| macOS (Intel) | `colony-macos-x86` |

```bash
chmod +x colony-linux && ./colony-linux
```

There is no installer. On macOS, Gatekeeper quarantines anything downloaded with
a browser, so clear the flag first: `xattr -d com.apple.quarantine colony-macos`.
To check a download yourself, see [Code signing policy](#code-signing-policy).
A new version is published when its release is cut, not after every change, and
`/releases/latest` always points at the newest one.

### Build from source

```bash
git clone https://github.com/Project-Colony/Colony
cd Colony
cargo build --release
./target/release/colony
```

Requires Rust 1.88 or newer and a C compiler. No system libraries are needed:
on Linux, the D-Bus client Colony uses for the Secret Service is compiled from
source.

## Design principles

**Single purpose, native performance.** Colony and every app in its ecosystem
follow the same philosophy:

- **Rust-native**: built with [Iced](https://iced.rs). Startup is instant,
  memory use is small, and your CPU stays cool.
- **Async everything**: network calls, file I/O and scanning run in the
  background. The UI never freezes.
- **Zero configuration needed**: Colony works out of the box. Scan directories,
  sidebar sections and themes can all be changed, but the defaults are sensible.

## Theming

Colony ships with **26 theme families and 59 palettes**, compiled into the
binary with zero runtime cost. They come from
[colony-ui](https://crates.io/crates/colony-ui) and are shared with every other
Project Colony program, so a theme looks the same wherever you meet it:

| | | | |
|---|---|---|---|
| Catppuccin (Latte, Frappé, Macchiato, Mocha) | Gruvbox | Everblush | Kanagawa (Lotus, Wave, Journal, Dragon) |
| Nord | Dracula | Solarized | Tokyo Night |
| Rosé Pine (Main, Moon, Dawn) | One Dark | Monokai Pro (Pro, Classic, Spectrum) | Ayu (Dark, Mirage, Light) |
| Everforest | Material (Oceanic, Palenight, Deep Ocean) | Flexoki | Nightfox |
| Sonokai | Oxocarbon | Night Owl | Iceberg |
| Horizon | Mélange | Synthwave '84 | Modus (Operandi, Vivendi) |
| Stellar Blade (Eve, Tachy, Lily, Enya, Kaya; fan-made, unofficial) | Parchment | | |

Each palette includes full semantic tokens: backgrounds, text layers, accents,
success, warning and error states, button states, and more.

## For app developers

Want your desktop app to appear in Colony's catalog? It takes one JSON file plus
a GitHub release with properly named assets. Colony only lists repositories of
the [Project-Colony](https://github.com/Project-Colony) GitHub organisation, so
the repository has to live there.

**Quick version**: add `colony.json` at the root of your repository:

```json
{
  "$schema": "https://raw.githubusercontent.com/Project-Colony/Project-Colony-Resources/main/generated/colony.schema.json",
  "name": "YourApp",
  "category": "utility"
}
```

Then publish a GitHub release with assets named `yourapp-linux`,
`yourapp-windows.exe`, `yourapp-macos` and `yourapp-macos-x86`. Colony picks
them up automatically.

**Full walkthrough**: [CONTRIBUTING.md, Adding your app to Colony](CONTRIBUTING.md#adding-your-app-to-colony),
and the [colony.json reference](docs/guide/colony-spec.md).

The canonical [release workflow template](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/templates/sign-and-publish-caller.yml)
lives in Project-Colony-Resources, alongside the shared signing workflow it calls
and the manifest schema.

## Platforms

Colony is developed and tested on Linux. Windows and macOS builds are produced
by the same CI, install and update apps, and are genuinely usable, but they get
far less exercise, and some desktop integration is Linux-only.

| Platform | Architecture | Status | Notes |
|---|---|---|---|
| Linux | x86_64 | Supported | Primary platform. Menu entries for installed apps, `.desktop` scan of local apps. |
| Windows | x86_64 | Best-effort | Installs, updates and launches. No Start Menu shortcut is created for installed apps. |
| macOS | ARM (Apple Silicon) | Best-effort | Installs, updates and launches single-binary apps. `.app` bundles are not supported, and nothing is registered with Launch Services. |
| macOS | x86_64 (Intel) | Best-effort | As above. |

"Best-effort" means bug reports are welcome and will be fixed, but the platform
is not part of the routine test loop. If you use Colony on Windows or macOS and
something is wrong, please [open an issue](https://github.com/Project-Colony/Colony/issues):
that is how these move up.

## Documentation

Full documentation is in [docs/](docs/). Start at
[docs/README.md](docs/README.md).

## Code signing policy

Free code signing provided by [SignPath.io](https://signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

Windows builds will be Authenticode-signed once SignPath Foundation has accepted
the project; until then they ship without Authenticode. Every release asset, on
every platform, is signed by the Project Colony organisation in CI, never on a
developer machine. Next to each asset on the release page:

| File | What it is |
|---|---|
| `<asset>.sig` | an ed25519 signature over the asset, made with the organisation's release key |
| `<asset>.meta` | three lines binding the asset to its file name, its sha256 and the release version |
| `<asset>.meta.sig` | an ed25519 signature over the `.meta` |

The private key is an organisation secret, used only by the shared
[sign-and-publish workflow](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
in a job that builds nothing; the jobs that compile Colony never see it. Colony
checks all three files before it applies its own update, and refuses a release
older than the one running. To check a download yourself with OpenSSL 3 (the
same commands work for every asset):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=colony-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

How releases are built, signed and published:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing),
and for Colony's side of it, [docs/internals/release-signing.md](docs/internals/release-signing.md).

Team roles and members:

- Committers and reviewers: [MotherSphere](https://github.com/MotherSphere)
- Approvers: [MotherSphere](https://github.com/MotherSphere)

### Privacy policy

Colony talks to GitHub and to no other server. It has no telemetry, no analytics
and no crash reporting.

- **At every start, automatically**, and again when you press Refresh: it reads
  the Project-Colony catalogue from the GitHub API (`api.github.com`): the
  organisation's repository list and, for each repository, its `colony.json`,
  README, licence, changelog and icon, plus its latest release when the manifest
  does not list its files.
- **Update checks, automatically unless turned off** (Preferences, General,
  "Check automatically", on by default) or when you press "Check for updates":
  it asks the GitHub API for the release of every installed app and the latest
  release of Colony itself. These requests name the repositories of the apps you
  have installed, so GitHub can see which Colony apps you use.
- **When you press "What's new in {version}"** on an app's page: it asks the
  GitHub API for that release, to show its notes.
- **When you install or update an app, or update Colony**: it looks the release
  up through the GitHub API, then downloads the release file and its `.sig`,
  `.meta` and `.meta.sig` from `github.com`, which serves them from GitHub's own
  download hosts.
- **When you sign in with GitHub** (optional): it uses GitHub's device flow
  (`github.com/login/device/code` and `github.com/login/oauth/access_token`,
  scope `read:user`), opens the verification page in your browser, and reads
  your user name from `api.github.com/user`. While you are signed in, your
  GitHub token is sent with the requests above, to GitHub only, to raise the API
  rate limit.

Requests carry a `Colony-Launcher` user agent. Beyond that, they tell GitHub only
which repositories and files are asked for (a download's file name shows your
platform) and, while you are signed in, which account is asking; nothing from
your settings or your files is sent. GitHub also sees what any web request shows
it, such as your IP address, under the
[GitHub General Privacy Statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).
Links you click open in your web browser. The apps you install through Colony are
separate programs with their own behaviour.

What Colony keeps on your machine, and nowhere else (Linux paths shown):

| Data | Where |
|---|---|
| Preferences and favourites | `~/.config/Colony/Colony/preferences/` |
| GitHub token, while you are signed in | the operating system's credential store: the macOS Keychain, the Windows Credential Manager, or the Secret Service on Linux (GNOME Keyring, KWallet). Only when that store is unavailable or does not give the token back, `~/.config/Colony/Colony/auth/github_token.json`, readable by your user alone (mode 600) |
| Catalogue, READMEs, icons, HTTP and scan caches, update downloads | `~/.cache/Colony/Colony/` |
| Diagnostics log, rewritten at every start | `~/.local/share/Colony/Colony/colony.log` |
| Installed apps, with their version and signature markers | `~/.local/share/Colony/apps/<App>/` |
| Menu entries for installed apps (Linux only) | `~/.local/share/applications/colony-<app>.desktop` |

On Windows the same files live under `%LOCALAPPDATA%\Colony\`, and on macOS
under `~/Library/Application Support/Colony/` and `~/Library/Caches/Colony/`.
Signing out deletes the token from the credential store and the file. A
self-update replaces the Colony executable where it is installed and keeps the
previous one next to it with an `.old` extension.

## License

[GPL-3.0-or-later](LICENSE) © 2026 Project Colony contributors

Colony is free software: you can redistribute it and/or modify it under the terms
of the GNU General Public License as published by the Free Software Foundation,
either version 3 of the License, or (at your option) any later version.

The fonts Colony embeds come from the [colony-ui](https://crates.io/crates/colony-ui)
crate and keep their own license, the SIL Open Font License 1.1, with the full
text next to each font in colony-ui:
[JetBrains Mono Nerd Font](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/crates/colony-ui/assets/fonts/JetBrainsMonoNerdFont/OFL.txt),
[Font Awesome Free](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/crates/colony-ui/assets/fonts/FontAwesome/OFL.txt)
and [OpenDyslexic](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/crates/colony-ui/assets/fonts/OpenDyslexic/OFL.txt).
