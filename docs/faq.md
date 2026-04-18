# Colony — FAQ

## General

### What is Colony exactly?

A desktop application launcher. It curates a catalog of small Rust-native apps (the Project-Colony ecosystem), downloads them from GitHub Releases on demand, and launches them. It also scans your system for apps you already have installed and surfaces them in the same grid.

### Is Colony free / open source?

Yes. [MIT licensed](../LICENSE), no telemetry, no mandatory account, no paid tier. Source at [Project-Colony/Colony](https://github.com/Project-Colony/Colony).

### What platforms are supported?

Linux x86_64, Windows x86_64, macOS ARM (Apple Silicon), macOS x86_64 (Intel). ARM Linux and BSDs aren't built yet — build-from-source should work but is untested.

### Why another launcher?

Two reasons: (1) centralize the ecosystem of small Rust-native Project-Colony apps into one install-and-forget entry point, (2) distribute those apps without forcing each one through package managers, flatpak, or the Microsoft Store. The full pitch is in the [README](../README.md#design-principles).

---

## Installation & updates

### How do I install Colony on Arch / Manjaro / EndeavourOS?

```
paru -S colony-bin    # prebuilt, recommended
```

For the bleeding edge (built from HEAD every upgrade):

```
paru -S colony-git
```

Both provide `colony` and add a `.desktop` entry so rofi/wofi/GNOME/KDE launchers find it.

### I'm on Debian/Fedora/other — what do I do?

Download `colony-linux` from the [latest release](https://github.com/Project-Colony/Colony/releases/latest), `chmod +x`, run. Details in the [tutorial](tutorial.md#other-linux-distributions).

### Does Colony update itself?

Yes. Colony periodically checks the `Project-Colony/Colony` repo for a newer release. When available, a badge appears in the sidebar; clicking it downloads the new binary, backs up the old one (`.old`), swaps them, and respawns. If the swap fails the backup is restored automatically.

On Arch via `paru -S colony-bin`, `paru -Syu` handles the upgrade — Colony's in-app self-update and the AUR package stay in sync because the AUR package auto-bumps from every Colony GitHub Release.

### How do apps in the catalog update?

Colony compares each installed app's stored version (under `~/.local/share/Colony/apps/<repo>/.colony_version`) against the latest GitHub Release tag. When newer, the detail view shows an **Update** button.

### How do I uninstall an app I installed through Colony?

Open the app in Colony → detail view → **Uninstall**. Or delete the app's directory under `~/.local/share/Colony/apps/<repo>/` manually.

### How do I uninstall Colony itself?

AUR: `sudo pacman -R colony-bin` (or `colony-git`). Manual install: delete the binary plus `~/.config/colony/`, `~/.cache/colony/`, and `~/.local/share/Colony/`.

---

## GitHub authentication

### Do I need a GitHub account?

No, Colony works without one. The cost is a 60 request/hour rate limit (the public GitHub API default). If you browse actively you'll hit it fast.

### Connecting changes what?

You get a 5000 req/h limit and Colony can read private repos you have access to (useful for testing an unpublished app). Only public-repo read scopes are requested.

### Where is the token stored?

In your OS keychain (GNOME keyring, KWallet, macOS Keychain, Windows Credential Manager) via the [keyring](https://crates.io/crates/keyring) crate. If no keychain is available (headless Linux, CI) it falls back to `~/.config/colony/github_token.json` with `chmod 600`.

### How do I disconnect?

Settings → GitHub → **Disconnect**. The token is deleted locally. The app lease on your GitHub account is not revoked server-side — you can revoke it from [GitHub settings → Applications](https://github.com/settings/applications) if you want.

---

## Catalog / detection

### My app isn't showing up — why?

Most common causes:

1. No `colony.json` at the root of the repo.
2. `colony.json` has an invalid `category` (valid: `Development`, `Graphics`, `Network`, `Office`, `Multimedia`, `System`, `Utilities`, `Games`, `Other`).
3. No GitHub release published (drafts don't count).
4. Release assets don't match the `<repo>-<platform>` convention (see [colony-spec.md](colony-spec.md)).
5. The repo is not under the `Project-Colony` org (Colony only scans that org).
6. GitHub rate limit hit — connect your account.

Cache file to inspect: `~/.cache/colony/repos_cache.json`.

### Does Colony support private repos?

Yes, if you authenticate with an account that has access. The app cards and downloads go through your token.

### Can I add apps from outside Project-Colony?

Not automatically. The catalog is scoped to `Project-Colony/*`. If you want to add your own, ask a maintainer to fork your repo under the org (or transfer it).

### Why don't I see my system apps (installed via pacman / MSI / .app)?

Colony scans these paths:

- **Linux**: `~/.local/share/applications`, `/usr/share/applications`, flatpak user/system applications.
- **Windows**: Start Menu.
- **macOS**: `/Applications`, `~/Applications`.

If your app is installed but not in any of these (portable binary without `.desktop`, for example) Colony can't find it. Ask your distro package or create a `.desktop` file.

---

## Configuration / data

### Where does Colony store its stuff?

| What                       | Path                                                    |
|----------------------------|---------------------------------------------------------|
| Preferences                | `~/.config/colony/preferences.json`                     |
| Favorites                  | `~/.config/colony/favorites.json`                       |
| GitHub token (fallback)    | `~/.config/colony/github_token.json` (`chmod 600`)      |
| Installed app binaries     | `~/.local/share/Colony/apps/<repo>/`                    |
| Installed version marker   | `~/.local/share/Colony/apps/<repo>/.colony_version`     |
| Resolved asset filename    | `~/.local/share/Colony/apps/<repo>/.colony_asset`       |
| Repo list cache            | `~/.cache/colony/repos_cache.json`                      |
| Manifest docs cache        | `~/.cache/colony/docs/<repo>/`                          |
| System app scan cache      | `~/.cache/colony/scan_cache.json`                       |
| Self-update staging        | `~/.local/share/Colony/update-staging/`                 |

Purging the `~/.cache/colony/` directory forces a full re-scan and re-fetch.

### Can I edit `preferences.json` by hand?

Yes. Colony re-reads it on launch. The file is JSON with field names matching what the Settings UI displays. Back it up first — malformed JSON resets to defaults.

### Can I override the scan directories?

Yes. Edit `~/.config/colony/preferences.json` and add custom paths under the platform-appropriate scan settings. The Settings UI also exposes this for Linux.

### I changed a theme and don't see it — why?

Themes apply immediately, no restart. If a palette looks off, verify Settings → Theme has the expected family + palette selected. All 50+ palettes are compiled into the binary, so if it's missing you're on an old version — update.

---

## Troubleshooting

### Colony won't launch / crashes immediately

Run from a terminal to see stderr:

```bash
colony 2>&1 | tee colony.log
```

Common causes:

- Missing Linux runtime libs (GTK, dbus, xdo) — on AUR the deps are pulled automatically. For manual downloads, install them via your package manager.
- Corrupted cache — `rm -rf ~/.cache/colony` and relaunch.
- Corrupted preferences — `mv ~/.config/colony/preferences.json ~/.config/colony/preferences.json.bak` and relaunch to regenerate defaults.

### Download stuck / very slow

Colony uses a 300s timeout per download. If you're on slow network this may not be enough; interrupt with Esc and retry.

### Self-update fails

The old binary is backed up as `.old` next to the running executable. If the swap fails, Colony restores it automatically. If for some reason you end up with a broken binary:

```bash
mv ~/.local/bin/colony.old ~/.local/bin/colony    # (adjust path to your install)
```

Or just re-download from the [latest release](https://github.com/Project-Colony/Colony/releases/latest).

### Where can I get help?

Open an issue on [Project-Colony/Colony](https://github.com/Project-Colony/Colony/issues) with your OS, Colony version (`colony --version`), and a log excerpt if possible.
