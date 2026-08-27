# Colony

**The hub for the Colony ecosystem.** Browse, install, update, and launch every Colony app from a single, lightweight interface.

Colony is the central piece of [Project Colony](https://github.com/Project-Colony) — an ecosystem of small, focused desktop utilities built with Rust. Instead of one monolithic tool that does everything poorly, Colony curates a growing collection of apps, each designed to do one thing exceptionally well.

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](LICENSE)
[![AUR: colony-bin](https://img.shields.io/badge/AUR-colony--bin-blue)](https://aur.archlinux.org/packages/colony-bin)
[![AUR: colony-git](https://img.shields.io/badge/AUR-colony--git-blue)](https://aur.archlinux.org/packages/colony-git)
[![Platforms](https://img.shields.io/badge/platforms-linux%20%7C%20windows*%20%7C%20macOS*-lightgrey)](#platforms)

<!-- Screenshots — drop PNG files into assets/screenshots/ and replace the comment below. -->
<!--
![Colony app grid](assets/screenshots/grid.png)
![Colony app detail view](assets/screenshots/detail.png)
-->

---

## Installation

### Arch Linux (AUR)

Two variants maintained on the AUR. Pick **one**:

| Package       | Install                      | Notes                                          |
|---------------|------------------------------|------------------------------------------------|
| `colony-bin`  | `paru -S colony-bin`         | Prebuilt binary, instant install (~40 MB DL). Auto-updates at every upstream release. |
| `colony-git`  | `paru -S colony-git`         | Builds from HEAD, ~5 min compile, always at latest commit. Recompiles on each `paru -Syu` if upstream advanced. |

Both provide a `/usr/bin/colony` binary and a `colony.desktop` entry so GNOME/KDE/rofi/wofi launchers pick it up automatically.

### Direct binary download (Linux / Windows / macOS)

Grab the single-file executable for your platform from the [latest release](https://github.com/Project-Colony/Colony/releases/latest):

| Platform              | Asset                    |
|-----------------------|--------------------------|
| Linux (x86_64)        | `colony-linux`           |
| Windows (x86_64)      | `colony-windows.exe`     |
| macOS (Apple Silicon) | `colony-macos`           |
| macOS (Intel)         | `colony-macos-x86`       |

No installer — download, `chmod +x` on Unix, and run. A new release is published automatically by `release-please` after each merged change, so the latest binary is always at `/releases/latest`.

### Build from source

```bash
git clone https://github.com/Project-Colony/Colony.git
cd Colony
cargo build --release
./target/release/colony
```

Requires Rust 1.88+ and, on Linux, `libgtk-3-dev`, `libxdo-dev`, `libdbus-1-dev`, `pkg-config`.

---

## What Colony does

- **Discover** — Browse all Colony apps by category, search by name, read descriptions, changelogs, and licenses without leaving the launcher.
- **Install & Update** — One click to download, one click to update. Colony tracks versions and shows when something new is available — for apps and for itself.
- **Launch** - Colony also detects every application already installed on your system (`.desktop` files on Linux, the Start Menu on Windows, `/Applications` on macOS) and lets you launch them alongside Colony apps.
- **Self-update** — Colony keeps itself up to date. When a new version is available, a badge appears in the sidebar; click to download, then restart.

➡️ New user? Start with the **[Tutorial](docs/tutorial.md)** for a step-by-step walkthrough.

## Design principles

**Single purpose, native performance.** Colony and every app in its ecosystem follow the same philosophy:

- **Rust-native** — Built with [Iced](https://iced.rs). Startup is instant, memory usage is minimal, and your CPU stays cool.
- **Async everything** — Network calls, file I/O, and scanning run in the background. The UI never freezes.
- **Zero configuration needed** — Colony works out of the box. Scan directories, sections, and themes are all configurable, but sensible defaults are provided.

## Theming

Colony ships with **25 theme families and 57 palettes**, all compiled into the binary with zero runtime cost:

| | | | |
|---|---|---|---|
| Catppuccin (Latte, Frappé, Macchiato, Mocha) | Gruvbox | Everblush | Kanagawa (Wave, Dragon, Lotus) |
| Nord | Dracula | Solarized | Tokyo Night |
| Rosé Pine (Main, Moon, Dawn) | One Dark | Monokai Pro (Pro, Classic, Spectrum) | Ayu (Dark, Mirage, Light) |
| Everforest | Material (Oceanic, Palenight, Deep Ocean) | Flexoki | Nightfox |
| Sonokai | Oxocarbon | Night Owl | Iceberg |
| Horizon | Mélange | Synthwave '84 | Modus (Operandi, Vivendi) |
| Stellar Blade (Eve, Tachy, Lily, Enya, Kaya) | | | |

Each palette includes full semantic tokens: backgrounds, text layers, accents, success/warning/error states, button states, and more.

## For app developers

Want your Rust (or any) desktop app to appear in Colony's catalog? It takes a single JSON file plus a GitHub release with properly named assets.

**Quick version** — add `colony.json` at the root of your repo:

```json
{
  "name": "YourApp",
  "category": "Utilities"
}
```

Then publish a GitHub Release with assets named `yourapp-linux`, `yourapp-windows.exe`, `yourapp-macos`, `yourapp-macos-x86`. Colony picks them up automatically.

**Full walkthrough**: see [CONTRIBUTING.md § Adding your app to Colony](CONTRIBUTING.md#adding-your-app-to-colony).

A [release workflow template](.github/workflows/colony-rust-release.yml.template) is included for Rust apps using Release Please.

## Platforms

Colony is developed and tested on Linux. Windows and macOS builds are produced
by the same CI, install and update apps, and are genuinely usable, but they get
far less exercise, and some desktop integration is Linux-only.

| Platform | Architecture | Status | Notes |
|---|---|---|---|
| Linux | x86_64 | Supported | Primary platform. `.desktop` entries for installed apps, `.desktop` scan of local apps. |
| Windows | x86_64 | Best-effort | Installs, updates and launches. No Start Menu shortcut is created for installed apps. |
| macOS | ARM (Apple Silicon) | Best-effort | Installs, updates and launches single-binary apps. `.app` bundles are not supported, and nothing is registered with Launch Services. |
| macOS | x86_64 (Intel) | Best-effort | As above. |

"Best-effort" means bug reports are welcome and will be fixed, but the platform
is not part of the routine test loop. If you use Colony on Windows or macOS and
something is wrong, please [open an issue](https://github.com/Project-Colony/Colony/issues): that is
how these move up.

## Documentation

| Document                           | Purpose                                               |
|------------------------------------|-------------------------------------------------------|
| [Tutorial](docs/tutorial.md)       | End-user walkthrough: install → first app → launch    |
| [FAQ](docs/faq.md)                 | Common questions + troubleshooting                    |
| [Architecture](docs/architecture.md) | Internal structure, tech stack, data flow             |
| [Colony spec](docs/colony-spec.md) | Full `colony.json` manifest reference                 |
| [Contributing](CONTRIBUTING.md)    | How to add your app + how to contribute to Colony itself |

## License

[GPL-3.0-or-later](LICENSE) © 2026 Project Colony contributors

Colony is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
