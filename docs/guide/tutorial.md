# Colony tutorial

This is a step-by-step walkthrough for first-time Colony users. By the end of
it you will have Colony installed, know its interface, have installed your
first app from the catalog, and have set the theme and your preferences.

## Prerequisites

A Linux, Windows, or macOS desktop and about 50 MB of free disk space for Colony
itself, plus room for each Colony app you install (typically 20 to 100 MB).

## 1. Install Colony

Pick the path that matches your system.

### Arch Linux

```bash
paru -S colony-bin     # the released binary, fastest
# or
paru -S colony-git     # built from the latest commit
```

### Other Linux distributions

Download `colony-linux` from the
[latest release](https://github.com/Project-Colony/Colony/releases/latest),
make it executable, and run it:

```bash
curl -L -o colony https://github.com/Project-Colony/Colony/releases/latest/download/colony-linux
chmod +x colony
./colony
```

Optional: move it to `~/.local/bin/colony` to have it in your `$PATH`, and
create a matching `~/.local/share/applications/colony.desktop` so your desktop's
launcher picks it up (the AUR packages install this file for you).

To check the download against its signature first, follow
[Code signing policy](../../README.md#code-signing-policy) in the README.

### Windows

Download `colony-windows.exe` from the
[latest release](https://github.com/Project-Colony/Colony/releases/latest) and
double-click it. Windows Defender may warn you, because the file is not
Authenticode-signed yet (it is signed with the Project Colony release key, which
Windows does not know): click "More info", then "Run anyway".

### macOS

Download `colony-macos` (Apple Silicon) or `colony-macos-x86` (Intel) from the
[latest release](https://github.com/Project-Colony/Colony/releases/latest).

```bash
chmod +x colony-macos
xattr -d com.apple.quarantine colony-macos   # remove the Gatekeeper quarantine
./colony-macos
```

## 2. First launch: the main interface

When you open Colony for the first time, a short guided tour points at each
part of the window. It looks like this:

```
┌─────────────┬──────────────────────────────────────────┐
│  Colony     │  Search...                               │
│             │                                          │
│  All        │   ┌────────┐ ┌────────┐ ┌────────┐       │
│  Favorites  │   │ App 1  │ │ App 2  │ │ App 3  │       │
│  Linux      │   └────────┘ └────────┘ └────────┘       │
│  Develop.   │                                          │
│  Graphics   │   ┌────────┐ ┌────────┐ ┌────────┐       │
│  Network    │   │ App 4  │ │ App 5  │ │ App 6  │       │
│  ...        │   └────────┘ └────────┘ └────────┘       │
│             │                                          │
│  GitHub     │                                          │
│  Rescan     │                                          │
└─────────────┴──────────────────────────────────────────┘
```

- **Colony**: the name at the top of the sidebar opens Preferences: General,
  Appearance, Accessibility, Storage, Shortcuts and About. Click it again, or
  Close, to go back.
- **Sidebar sections**: **All** lists the whole Colony catalog, and
  **Favorites** the apps you starred. The section named after your system
  (**Linux**, **Windows** or **macOS**) lists the applications already
  installed on it, next to the catalog apps that have a build for it. Below
  come the Colony categories: Development, Graphics, Network, Office,
  Multimedia, System, Utilities, Security, Games and Other.
- **App grid**: a card for every app in the selected section. Click a card to
  open its detail view.
- **Search**: the bar above the grid filters by app name.
- **GitHub** and **Rescan**, at the bottom of the sidebar: sign in with GitHub
  (see section 6), and scan your system for installed applications again.

## 3. Install your first Colony app

1. Pick a category in the sidebar (for example **Multimedia**).
2. Click the app card you want.
3. The detail view opens:
   - the app's README, licence and changelog, fetched from its repository
   - the installed version, if any, and the latest release
   - a **Download** button (or **Update** and **Launch** once it is installed)
4. Click **Download**. Colony downloads the file for your platform (for
   example `grape-linux`), checks its signature when the release has one and
   its SHA-256 when the app's manifest gives one, and stores it under
   `~/.local/share/Colony/apps/<App>/`.
5. When the download completes, the button becomes **Launch**. Click it: the
   app starts as a separate process, independent of Colony.

On Linux, Colony also writes a menu entry for the app
(`~/.local/share/applications/colony-<app>.desktop`), so you can start it later
from your desktop's launcher (rofi, wofi, GNOME Activities, KDE) too.

## 4. System apps, not just Colony apps

Colony scans your system for applications that are already installed:

- **Linux**: `.desktop` files in `~/.local/share/applications` (or
  `$XDG_DATA_HOME/applications`), the `applications` folder of every
  `$XDG_DATA_DIRS` entry (by default `/usr/share/applications` and
  `/usr/local/share/applications`), and the Flatpak and Snap export folders.
- **Windows**: Start Menu entries, for all users and for you.
- **macOS**: `.app` bundles under `/Applications`, `/System/Applications` and
  `~/Applications`.

Open the section named after your system to see them. A scan runs at startup
unless you turn that off in Preferences, Storage; **Rescan** runs one on demand.

## 5. Change the theme

Click **Colony** at the top of the sidebar, then **Appearance**. Colony ships 26
theme families and 59 palettes (Catppuccin, Gruvbox, Nord, Dracula, Rosé Pine,
Tokyo Night and more), plus an accent colour. Changes apply instantly, no
restart needed. **Accessibility**, next to it, has the text size, high
contrast, a dyslexia-friendly font and reduced motion.

## 6. Connect your GitHub account (optional)

Without an account Colony uses the public GitHub API, which allows 60 requests
per hour. If you browse a lot of apps, sign in for a limit of 5000 per hour:

1. Click **GitHub** in the sidebar.
2. Click **Sign in with GitHub**.
3. Colony shows a code and opens GitHub's device page in your browser. Enter
   the code there and authorise Colony.
4. Colony keeps the token in your operating system's credential store (the
   macOS Keychain, the Windows Credential Manager, or the Secret Service on
   Linux). Only when that store is unavailable does it fall back to a file that
   only your user can read.
5. **Sign out**, in the same panel, deletes the token from your machine.

Colony asks GitHub for the `read:user` scope only: enough to read your user
name. It gets no access to your repositories, private or public, and the catalog
it shows is the organisation's public repositories either way.

## 7. Keep Colony up to date

Colony looks for a new version of itself at every start (unless you turn off
"Check automatically" in Preferences, General) and whenever you press "Check for
updates". When one is available, a badge appears at the bottom of the sidebar:
click it to download the update, then restart. On an AUR install, `paru -Syu`
upgrades Colony with the rest of your system instead.

## Troubleshooting

- **App card not showing up?** Check that the repository has a `colony.json` at
  its root and a published release with assets named `<repo>-<platform>`. See
  [colony-spec.md](colony-spec.md) for the full manifest reference.
- **Rate limit hit?** Sign in with GitHub (section 6).
- **Download fails?** Check your network. Colony tries three times and resumes
  where it stopped. The log, `~/.local/share/Colony/Colony/colony.log`, says
  what went wrong.
- **More questions?** See the [FAQ](faq.md).
