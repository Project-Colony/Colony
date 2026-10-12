# Colony FAQ

## General

### What is Colony exactly?

A desktop application launcher. It curates a catalog of small Rust-native apps (the Project-Colony ecosystem), downloads them from GitHub Releases on demand, and launches them. It also scans your system for apps you already have installed and shows them in the same grid.

### Is Colony free / open source?

Yes. [GPL-3.0-or-later licensed](../../LICENSE), no telemetry, no mandatory account, no paid tier. Source at [Project-Colony/Colony](https://github.com/Project-Colony/Colony).

### What platforms are supported?

Linux x86_64, Windows x86_64, macOS ARM (Apple Silicon), macOS x86_64 (Intel). ARM Linux and the BSDs are not built yet: building from source should work but is untested.

### Why another launcher?

Two reasons: (1) gather the small Rust-native Project-Colony apps behind one install-and-forget entry point, (2) distribute those apps without forcing each one through package managers, Flatpak, or the Microsoft Store. The full pitch is in the [README](../../README.md#why-colony).

---

## Installation & updates

### How do I install Colony on Arch / Manjaro / EndeavourOS?

```
paru -S colony-bin    # the released binary, recommended
```

For the latest commit (rebuilt when the repository moves):

```
paru -S colony-git
```

Both provide `colony` and add a `.desktop` entry so rofi, wofi, GNOME and KDE launchers find it.

### I'm on Debian/Fedora/other: what do I do?

Download `colony-linux` from the [latest release](https://github.com/Project-Colony/Colony/releases/latest), `chmod +x`, run. Details in the [tutorial](tutorial.md#other-linux-distributions).

### Does Colony update itself?

Yes. At every start (unless "Check automatically" is off in Preferences, General) and whenever you press "Check for updates", Colony asks the `Project-Colony/Colony` repository for a newer release. When there is one, a badge appears in the sidebar; clicking it downloads the new binary and checks its signature and signed `.meta`. Restarting then backs up the old binary (`.old`), swaps in the new one, and starts it. If the swap fails, the backup is restored automatically.

On Arch, `paru -Syu` handles the upgrade instead: an install pacman manages cannot replace its own binary, so Colony shows guidance rather than an update button there. The AUR package is bumped at every Colony release.

### How do apps in the catalog update?

Colony compares each installed app's stored version (in `~/.local/share/Colony/apps/<repo>/.colony_version`) with its latest GitHub release tag. When the release is newer, the detail view shows an **Update** button.

### How do I uninstall an app I installed through Colony?

Open the app in Colony, then use the uninstall action in its detail view and confirm. Or delete the app's directory under `~/.local/share/Colony/apps/<repo>/` by hand (on Linux, also its menu entry, `~/.local/share/applications/colony-<app>.desktop`).

### How do I uninstall Colony itself?

Sign out of GitHub first if you signed in, so the token leaves your credential store. Then, on the AUR: `sudo pacman -R colony-bin` (or `colony-git`). For a manual install, delete the binary. To remove everything Colony wrote, also delete `~/.config/Colony/Colony/`, `~/.cache/Colony/Colony/`, `~/.local/share/Colony/` and the `colony-*.desktop` files in `~/.local/share/applications/`. (`~/.local/share/Colony/apps/` holds the apps you installed, so delete it only if you want those gone too.)

---

## GitHub authentication

### Do I need a GitHub account?

No, Colony works without one. The cost is a rate limit of 60 requests per hour (the public GitHub API default). If you browse actively you will hit it fast.

### Connecting changes what?

You get a limit of 5000 requests per hour. Nothing else: Colony requests the `read:user` scope only, which lets it read your user name. It gets no access to your repositories, and the catalog it shows is the organisation's public repositories whether you are signed in or not.

### Where is the token stored?

In your operating system's credential store: the macOS Keychain, the Windows Credential Manager, or the Secret Service on Linux (GNOME Keyring, KWallet), through [keyring-core](https://crates.io/crates/keyring-core) and the native store of each OS. If no store is available (a headless Linux session, CI), it falls back to `~/.config/Colony/Colony/auth/github_token.json`, readable by your user alone (mode 600). That file is removed only once the token reads back from the credential store.

### How do I disconnect?

Click **GitHub** in the sidebar, then **Sign out**. The token is deleted from your machine. The authorisation on your GitHub account is not revoked server-side: revoke it from [GitHub settings, Applications](https://github.com/settings/applications) if you want.

---

## Catalog / detection

### My app isn't showing up: why?

Most common causes:

1. No `colony.json` at the root of the repository.
2. `colony.json` has an unknown `category`, which files the app under Other. Valid values: `development`, `graphics`, `network`, `office`, `multimedia`, `system`, `utility`, `security`, `game`, `other` (case-insensitive; `utilities` and `games` are accepted too).
3. No GitHub release published (drafts do not count).
4. Release assets do not match the `<repo>-<platform>` convention (see [colony-spec.md](colony-spec.md)). `colony validate-manifest` checks both of the above before you release.
5. The repository is not a public repository of the `Project-Colony` organisation (Colony only lists those).
6. GitHub rate limit hit: sign in with GitHub.

Cache file to inspect: `~/.cache/Colony/Colony/repos_cache.json`.

### Does Colony support private repos?

No. Colony lists the public repositories of the `Project-Colony` organisation, and the token it asks for (`read:user`) gives it no access to private ones.

### Can I add apps from outside Project-Colony?

Not automatically. The catalog is scoped to `Project-Colony/*`. If you want to add your own, ask a maintainer to fork your repository under the organisation (or transfer it).

### Why don't I see my system apps (installed via pacman / MSI / .app)?

Colony scans these places:

- **Linux**: `~/.local/share/applications` (or `$XDG_DATA_HOME/applications`), the `applications` folder of every `$XDG_DATA_DIRS` entry (by default `/usr/share/applications` and `/usr/local/share/applications`), and the Flatpak and Snap export folders.
- **Windows**: the Start Menu, for all users and for you.
- **macOS**: `/Applications`, `/System/Applications`, `~/Applications`.

If your app is installed but not in any of these (a portable binary without a `.desktop` file, for example), Colony cannot find it. Create a `.desktop` file for it, or add its folder to the scanned directories (see below).

---

## Configuration / data

### Where does Colony store its stuff?

| What | Path (Linux) |
|---|---|
| Preferences | `~/.config/Colony/Colony/preferences/preferences.json` |
| Favorites | `~/.config/Colony/Colony/preferences/favorites.json` |
| GitHub token (fallback only) | `~/.config/Colony/Colony/auth/github_token.json` (mode 600) |
| Installed app binaries | `~/.local/share/Colony/apps/<repo>/` |
| Installed version marker | `~/.local/share/Colony/apps/<repo>/.colony_version` |
| Resolved asset filename | `~/.local/share/Colony/apps/<repo>/.colony_asset` |
| Signature markers | `~/.local/share/Colony/apps/<repo>/.colony_signed` and `.colony_meta` |
| Menu entries for installed apps | `~/.local/share/applications/colony-<app>.desktop` |
| Repo list cache | `~/.cache/Colony/Colony/repos_cache.json` |
| Manifest docs cache | `~/.cache/Colony/Colony/repo-docs/<repo>/` |
| App icon cache | `~/.cache/Colony/Colony/repo-icons/<repo>/icon.png` |
| System app scan cache | `~/.cache/Colony/Colony/scan_cache.json` |
| Self-update staging | `~/.cache/Colony/Colony/update-staging/` |
| HTTP ETag cache | `~/.cache/Colony/Colony/http_etags.json` |
| Diagnostics log | `~/.local/share/Colony/Colony/colony.log` |

On Windows all three roots are `%LOCALAPPDATA%\Colony\`, and on macOS config
and data share `~/Library/Application Support/Colony/` while the cache sits in
`~/Library/Caches/Colony/`. Menu entries are written on Linux only. The layout
is defined once in
[Project-Colony-Resources](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/filesystem.md).

Deleting `~/.cache/Colony/Colony/` forces a full re-scan and re-fetch, and loses nothing else.

### Can I edit `preferences.json` by hand?

Yes, while Colony is closed: it reads the file at launch and rewrites it when you change a preference. The fields are named after what they hold (`selected_theme`, `font_size` and so on). Back it up first: a file Colony cannot parse is moved aside to `preferences.corrupt`, and Colony starts from defaults.

### Can I override the scan directories?

Yes, with a `colony.toml`. Copy the shipped [`config/colony.toml`](https://github.com/Project-Colony/Colony/blob/main/config/colony.toml) to `~/.config/colony/colony.toml` (on macOS `~/Library/Application Support/colony/`, on Windows `%LOCALAPPDATA%\colony\`) and edit its lists: `[scan] unix = [...]` on Linux and macOS, `windows = [...]` on Windows. Environment variables such as `${HOME}` are expanded. The list replaces the defaults; it does not add to them. Colony looks for the file in the same places, and in the same order, as `categories.json` below.

### I changed a theme and don't see it: why?

Themes apply immediately, no restart. If a palette looks off, check that Preferences, Appearance has the expected family and palette selected. All 59 palettes are compiled into the binary, so if one is missing you are on an old version: update.

### Can I rename or reorder the sidebar sections?

Yes. Colony looks for a `categories.json` override before falling back to the
copy compiled into the binary, in this order:

1. `~/.config/colony/categories.json` on Linux,
   `~/Library/Application Support/colony/categories.json` on macOS, and
   `%LOCALAPPDATA%\colony\categories.json` on Windows, where
   `%APPDATA%\colony\` is still read after it
2. `config/categories.json` next to the `colony` binary
3. `./config/categories.json`

Copy the shipped [`config/categories.json`](https://github.com/Project-Colony/Colony/blob/main/config/categories.json)
to the first path and edit it. Each entry takes `name`, `icon` (a Nerd Font
glyph), `origin` (`colony`, `windows`, `external`), `category`, and an optional
`platforms` list that hides the section on other operating systems. A Favorites
section is injected automatically if you leave it out.

---

## Troubleshooting

### Colony won't launch / crashes immediately

Run it from a terminal to see what it prints:

```bash
colony 2>&1 | tee colony-console.log
```

The same lines are in `~/.local/share/Colony/Colony/colony.log`. Things worth trying:

- Clear the cache: `rm -rf ~/.cache/Colony/Colony` and relaunch. It only holds what Colony can fetch again.
- Start from default preferences: `mv ~/.config/Colony/Colony/preferences/preferences.json{,.bak}` and relaunch.

### Download stuck / very slow

Colony gives up on a download only when no data has arrived for 60 seconds, so a slow line still finishes a large file. To stop one, press the × on its progress bar.

### Self-update fails

The old binary is kept as `.old` next to the running executable. If the swap fails, Colony restores it automatically. If you still end up with a broken binary:

```bash
mv ~/.local/bin/colony.old ~/.local/bin/colony    # adjust the path to your install
```

Or download it again from the [latest release](https://github.com/Project-Colony/Colony/releases/latest).

### Where can I get help?

Open an issue on [Project-Colony/Colony](https://github.com/Project-Colony/Colony/issues) with your OS, your Colony version (`colony --version`), and a log excerpt if possible.
