# Colony

**The hub for the Colony ecosystem.** Browse, install, update, and launch every Colony app from a single, lightweight interface.

Colony is the central piece of [Project Colony](https://github.com/Project-Colony) — an ecosystem of small, focused desktop utilities built with Rust. Instead of one monolithic tool that does everything poorly, Colony curates a growing collection of apps, each designed to do one thing exceptionally well.

## What Colony does

- **Discover** — Browse all Colony apps by category, search by name, read descriptions, changelogs, and licenses without leaving the launcher.
- **Install & Update** — One click to download, one click to update. Colony tracks versions and shows when something new is available — for apps and for itself.
- **Launch** — Colony also detects every application already installed on your system (Start Menu on Windows, `.desktop` files on Linux) and lets you launch them alongside Colony apps.
- **Self-update** — Colony keeps itself up to date. When a new version is available, a badge appears in the sidebar; click to download, then restart.

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

Adding your app to Colony is simple. Create a `colony.json` at the root of your repo:

```json
{
  "name": "YourApp",
  "category": "Utilities"
}
```

That's it. Colony auto-detects available platforms from your GitHub release assets using the naming convention:

| Asset name | Platform |
|---|---|
| `yourapp-linux` | Linux x86_64 |
| `yourapp-windows.exe` | Windows x86_64 |
| `yourapp-macos` | macOS ARM (Apple Silicon) |
| `yourapp-macos-x86` | macOS Intel |

A [release workflow template](.github/workflows/colony-rust-release.yml.template) is included for Rust apps using Release Please.

## Platforms

| Platform | Architecture | Status |
|---|---|---|
| Linux | x86_64 | Supported |
| Windows | x86_64 | Supported |
| macOS | ARM (Apple Silicon) | Supported |
| macOS | x86_64 (Intel) | Supported |

## License

MIT
