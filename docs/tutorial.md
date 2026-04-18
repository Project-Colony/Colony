# Colony — Tutorial

This is a step-by-step walkthrough for first-time Colony users. By the end of
it you'll have Colony installed, understand its interface, installed your first
app from the catalog, and customized themes + preferences.

## Prerequisites

A Linux, Windows, or macOS desktop and ~50 MB of free disk space for Colony
itself. Additional space per Colony app you install (typically 20–100 MB each).

## 1. Install Colony

Pick the path that matches your system. All roads lead to the same `colony`
binary and desktop entry.

### Arch Linux

```bash
paru -S colony-bin     # prebuilt, fastest
# or
paru -S colony-git     # builds from source, always at HEAD
```

### Other Linux distributions

Download `colony-linux` from the
[latest release](https://github.com/Project-Colony/Colony/releases/latest),
make it executable, and run:

```bash
curl -L -o colony https://github.com/Project-Colony/Colony/releases/latest/download/colony-linux
chmod +x colony
./colony
```

Optional: move it to `~/.local/bin/colony` to have it in your `$PATH`, and
create a matching `~/.local/share/applications/colony.desktop` so your launcher
picks it up (Colony ships this file automatically when installed via AUR).

### Windows

Download `colony-windows.exe` from the
[latest release](https://github.com/Project-Colony/Colony/releases/latest) and
double-click to run. Windows Defender may prompt — the binary is unsigned for
now; click "More info → Run anyway".

### macOS

Download `colony-macos` (Apple Silicon) or `colony-macos-x86` (Intel) from the
[latest release](https://github.com/Project-Colony/Colony/releases/latest).

```bash
chmod +x colony-macos
xattr -d com.apple.quarantine colony-macos   # remove Gatekeeper quarantine
./colony-macos
```

## 2. First launch — the main interface

When you open Colony for the first time you'll see three zones:

```
┌───────────┬──────────────────────────────────────────┐
│           │                                          │
│  Sidebar  │            Main panel (app grid)         │
│           │                                          │
│  ─────    │   ┌────────┐ ┌────────┐ ┌────────┐       │
│  All      │   │        │ │        │ │        │       │
│  Windows  │   │ App 1  │ │ App 2  │ │ App 3  │       │
│  Linux    │   │        │ │        │ │        │       │
│  ─────    │   └────────┘ └────────┘ └────────┘       │
│  Develop. │                                          │
│  Graphics │   ┌────────┐ ┌────────┐ ┌────────┐       │
│  Network  │   │        │ │        │ │        │       │
│  …        │   │ App 4  │ │ App 5  │ │ App 6  │       │
│           │   │        │ │        │ │        │       │
│  GitHub ⚙ │   └────────┘ └────────┘ └────────┘       │
└───────────┴──────────────────────────────────────────┘
```

- **Sidebar** — Categories. The top group (`All` / `Windows` / `Linux`) is
  the *origin* filter: Colony apps, apps already installed on your system, or
  both. The middle group is Colony categories (`Development`, `Graphics`,
  `Network`, `Office`, `Multimedia`, `System`, `Utilities`, `Games`, `Other`).
- **App grid** — Cards for every app matching the current filters. Click a
  card to open its detail view.
- **Search** — A search bar above the grid filters by app name.
- **Settings gear** — Bottom of the sidebar opens preferences (theme,
  language, scan directories, about).

## 3. Install your first Colony app

1. Pick a category in the sidebar (for example **Multimedia**).
2. Click the app card you want.
3. The detail view opens:
   - Name, description (pulled from the upstream README)
   - Current installed version vs. latest available
   - Category, supported platforms, license, changelog
   - A large **Install** button (or **Update** / **Launch** if already installed)
4. Click **Install**. Colony downloads the appropriate asset for your platform
   (e.g. `grape-linux`), verifies SHA256 when provided, and stores the binary
   under `~/.local/share/Colony/apps/<repo>/`.
5. When the download completes, **Install** becomes **Launch**. Click it —
   the app starts as a separate process, independent of Colony.

You can also launch the app later from your system launcher (rofi/wofi/GNOME
activities) if the app installed its own `.desktop` file.

## 4. System apps — not just Colony apps

Colony scans your system for already-installed applications:

- **Linux**: `.desktop` files in `~/.local/share/applications`, `/usr/share/applications`, and flatpak locations.
- **Windows**: Start Menu entries.
- **macOS**: `.app` bundles under `/Applications` and `~/Applications`.

Switch the sidebar origin to **Linux** (or **Windows**) to see only those.
**All** merges Colony's catalog with your local apps.

## 5. Customize the theme

Click the **gear icon** at the bottom of the sidebar → **Theme**. Colony ships
24 theme families and 50+ palettes (Catppuccin, Gruvbox, Nord, Dracula, Rosé
Pine, Tokyo Night, etc.). Theme changes apply instantly, no restart needed.

## 6. Connect your GitHub account (optional)

Without authentication Colony uses the public GitHub API: 60 requests per hour.
If you browse a lot of apps, connect your account for a 5000 req/h rate limit:

1. Open Settings → **GitHub** tab (or click the sidebar footer).
2. Click **Connect**.
3. Colony displays a device code. Open the URL it shows, paste the code, authorize.
4. Colony stores the token in your OS keychain (with a chmod-600 file fallback).
5. Disconnect any time — the token is deleted locally.

No scopes beyond public-repo reads are requested.

## 7. Keep Colony up to date

Colony checks for its own updates periodically. When a new version is
available, a badge appears next to the sidebar footer — click it to download,
then restart. On AUR you can also just run `paru -Syu` and Colony will be
upgraded alongside the rest of your system.

## Troubleshooting

- **App card not showing up?** Verify the repo has a `colony.json` at root and
  a published release with assets matching `<repo>-<platform>`. See
  [colony-spec.md](colony-spec.md) for the full manifest reference.
- **Rate limit hit?** Connect a GitHub account (section 6).
- **Download fails?** Check your network. Colony retries automatically; logs
  go to `~/.cache/colony/` if you need to dig in.
- **More questions?** See the [FAQ](faq.md).
