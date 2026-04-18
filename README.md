# Colony

**The hub for the Colony ecosystem.** Browse, install, update, and launch every Colony app from a single, lightweight interface.

Colony is the central piece of [Project Colony](https://github.com/Project-Colony) — an ecosystem of small, focused desktop utilities built with Rust. Instead of one monolithic tool that does everything poorly, Colony curates a growing collection of apps, each designed to do one thing exceptionally well.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![AUR: colony-bin](https://img.shields.io/badge/AUR-colony--bin-blue)](https://aur.archlinux.org/packages/colony-bin)
[![AUR: colony-git](https://img.shields.io/badge/AUR-colony--git-blue)](https://aur.archlinux.org/packages/colony-git)
[![Platforms](https://img.shields.io/badge/platforms-linux%20%7C%20windows%20%7C%20macOS-lightgrey)](#installation)

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

Requires Rust 1.80+ and, on Linux, `libgtk-3-dev`, `libxdo-dev`, `libdbus-1-dev`, `libasound2-dev`, `libglib2.0-dev`, `pkg-config`.

---

## What Colony does

- **Discover** — Browse all Colony apps by category, search by name, read descriptions, changelogs, and licenses without leaving the launcher.
- **Install & Update** — One click to download, one click to update. Colony tracks versions and shows when something new is available — for apps and for itself.
- **Launch** — Colony also detects every application already installed on your system (Start Menu on Windows, `.desktop` files on Linux) and lets you launch them alongside Colony apps.
- **Self-update** — Colony keeps itself up to date. When a new version is available, a badge appears in the sidebar; click to download, then restart.

➡️ New user? Start with the **[Tutorial](docs/tutorial.md)** for a step-by-step walkthrough.

## Design principles

**Single purpose, native performance.** Colony and every app in its ecosystem follow the same philosophy:

- **Rust-native** — Built with [Iced](https://iced.rs). Startup is instant, memory usage is minimal, and your CPU stays cool.
- **Async everything** — Network calls, file I/O, and scanning run in the background. The UI never freezes.
- **Zero configuration needed** — Colony works out of the box. Scan directories, sections, and themes are all configurable, but sensible defaults are provided.

## Theming

Colony ships with **24 theme families and 50+ palettes**, all compiled into the binary with zero runtime cost:

| | | | |
|---|---|---|---|
| Catppuccin (Latte, Frappé, Macchiato, Mocha) | Gruvbox | Everblush | Kanagawa (Wave, Dragon, Lotus) |
| Nord | Dracula | Solarized | Tokyo Night |
| Rosé Pine (Main, Moon, Dawn) | One Dark | Monokai Pro (Pro, Classic, Spectrum) | Ayu (Dark, Mirage, Light) |
| Everforest | Material (Oceanic, Palenight, Deep Ocean) | Flexoki | Nightfox |
| Sonokai | Oxocarbon | Night Owl | Iceberg |
| Horizon | Mélange | Synthwave '84 | Modus (Operandi, Vivendi) |

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

| Platform | Architecture | Status |
|---|---|---|
| Linux | x86_64 | Supported |
| Windows | x86_64 | Supported |
| macOS | ARM (Apple Silicon) | Supported |
| macOS | x86_64 (Intel) | Supported |

## Documentation

| Document                           | Purpose                                               |
|------------------------------------|-------------------------------------------------------|
| [Tutorial](docs/tutorial.md)       | End-user walkthrough: install → first app → launch    |
| [FAQ](docs/faq.md)                 | Common questions + troubleshooting                    |
| [Architecture](docs/architecture.md) | Internal structure, tech stack, data flow             |
| [Colony spec](docs/colony-spec.md) | Full `colony.json` manifest reference                 |
| [Contributing](CONTRIBUTING.md)    | How to add your app + how to contribute to Colony itself |

## License

[MIT](LICENSE) © 2026 Project Colony contributors
