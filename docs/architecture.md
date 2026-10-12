# Colony Architecture

## Overview

Colony is an application launcher written in Rust with Iced 0.14 (Elm architecture). It handles discovery, installation, updating, and launching of both local and remote applications (via GitHub).

## Tech stack

| Component | Technology |
|-----------|-----------|
| Language | Rust (edition 2021) |
| UI | Iced 0.14 (Elm architecture) |
| Shared UI | colony-ui: theme palettes, fonts, text sizing, motion, the Preferences kit |
| Async | Tokio (integrated via Iced runtime) |
| HTTP | reqwest (async + streaming), rustls checking certificates against the OS trust store (no OpenSSL) |
| Auth | GitHub Device Flow OAuth |
| Secret storage | keyring-core with the native store of each OS (Keychain, Credential Manager, Secret Service) + file fallback |
| Serialization | serde + serde_json |
| Configuration | TOML (scan dirs), JSON (categories, preferences) |
| Versioning | semver |
| Integrity | SHA256 (sha2 crate) |
| Archives | zip, flate2 + tar |

## File structure

```
src/
├── main.rs          - Entry point: CLI flags (--version, --help,
│                      validate-manifest), startup order, iced launch
├── manifest.rs      - `colony validate-manifest`: checks an app's colony.json
├── logging.rs       - Log file + stderr, with the startup migration's lines
├── state.rs         - App struct (global state), GitHubState, UI fields
├── message.rs       - Message enum (all events)
├── app.rs           - impl App: boot(), view(), subscription(), theme()
├── update/          - Message dispatch table (mod.rs) + handlers
│   ├── store.rs     - install / update / uninstall / notes / favorites
│   ├── github_auth.rs - device flow, catalog refresh
│   ├── launcher.rs  - self-update state machine
│   ├── preferences.rs, keyboard.rs, onboarding.rs
├── github/          - split by layer, mod.rs re-exports everything
│   ├── http.rs      - client, conditional-request cache, typed statuses
│   ├── types.rs     - colony.json / API wire shapes
│   ├── catalog.rs   - store listing
│   └── releases.rs  - tags, assets, platform auto-detection
├── download/        - split by concern, mod.rs re-exports the public API
│   ├── http.rs      - download client, resumable transfers, bounded sidecar
│   │                  reads, safe URL building
│   ├── files.rs     - path-component checks, no-follow creates, replacing a
│   │                  running binary, zip / tar.gz extraction
│   ├── verify.rs    - SHA-256, signed .meta bindings, downgrade rules
│   ├── install.rs   - app install: download, verify, extract, promote
│   └── launcher.rs  - self-update: staging, apply with rollback
├── signing.rs       - ed25519 verification (launcher AND app releases)
├── icons.rs         - PNG decoding for per-app grid icons
├── persistence.rs   - Data dirs, preferences (with colony-ui's
│                      StandardPreferences), install state, on-disk caches,
│                      favorites, desktop entries (Linux)
├── config.rs        - Locating external config (categories.json, colony.toml)
├── oauth.rs         - Device Flow OAuth (login, token, keychain)
├── scan.rs          - System application scanning (Linux/Windows/macOS)
├── sections.rs      - Categories, origin/category filters, JSON config
├── i18n/            - fr.rs, en.rs, and the Locale lookup (mod.rs); the
│                      shared Preferences labels come from colony-ui
└── ui/
    ├── mod.rs       - UI module declarations, the glyph + label helper
    ├── theme.rs     - colony-ui's 26 theme families and 59 palettes, Colony's
    │                  button styles
    ├── sidebar.rs   - Sidebar (the Colony name into Preferences, sections,
    │                  GitHub, rescan, update badge)
    ├── app_grid.rs  - Application card grid with search
    ├── detail.rs    - Detail view (README, changelog, license, actions)
    ├── settings.rs  - Preferences page on colony-ui's kit (General,
    │                  Appearance, Accessibility, Storage, Shortcuts, About)
    ├── github_panel.rs - GitHub connect/disconnect, Device Flow UI
    ├── markdown_blocks.rs - Cached-block Markdown rendering
    └── tutorial.rs  - First-launch guided tour (spotlight overlay)

config/
├── categories.json     - Sidebar sections (embedded; can be overridden)
├── colony.toml         - Scan directories
└── shields-colors.json - shields.io colour names for README badges (embedded)
```

## Data flow (Elm architecture)

```
User Action → Message → update() → State mutation + Task::perform()
                                         ↓
                              view() → Element tree → Render
```

All async operations (API calls, downloads, scanning) return a `Task<Message>` that, once completed, sends a `Message` back to `update()`.

## Security

- **OAuth**: Device Flow (no client_secret exposed)
- **Tokens**: Stored in the OS credential store, file fallback (chmod 600). The file is deleted only after the token reads back from the store
- **Downloads**: HTTPS only, optional SHA256 verification, resumable via Range with an ETag+length identity check before any partial file is continued
- **URLs**: every remote-controlled path segment is percent-encoded, never interpolated - the WHATWG parser collapses `..` before the request is sent
- **Timeouts**: 30s total for API requests, 10s connect, 60s read (inactivity) for downloads - a total deadline on a download makes a large asset impossible to fetch on a slow line
- **Self-update**: Binary backup before replacement, automatic rollback on failure
- **Signed updates**: Launcher updates verified against an embedded ed25519 public key (fail-closed) both at download and at apply time; see [release-signing.md](release-signing.md)

## Cache and persistence

| Data | Location | Duration |
|------|----------|----------|
| Colony repos (cache) | `~/.cache/Colony/Colony/repos_cache.json` | Offline fallback |
| Scanned apps (cache) | `~/.cache/Colony/Colony/scan_cache.json` | Session |
| HTTP ETag cache | `~/.cache/Colony/Colony/http_etags.json` | Conditional requests across launches |
| Repo docs (cache) | `~/.cache/Colony/Colony/repo-docs/<repo>/` | Offline fallback |
| Repo icons (cache) | `~/.cache/Colony/Colony/repo-icons/<repo>/` | Offline fallback |
| Preferences | `~/.config/Colony/Colony/preferences/preferences.json` | Permanent |
| Favorites | `~/.config/Colony/Colony/preferences/favorites.json` | Permanent |
| OAuth token | OS Keychain / `~/.config/Colony/Colony/auth/github_token.json` | Permanent |
| Diagnostics log | `~/.local/share/Colony/Colony/colony.log` | Truncated per run |
| Installed versions | `~/.local/share/Colony/apps/<repo>/.colony_version` | Permanent |
| Resolved asset | `~/.local/share/Colony/apps/<repo>/.colony_asset` | Permanent |
| Colony binaries | `~/.local/share/Colony/apps/<repo>/` | Permanent |
| Self-update staging | `~/.cache/Colony/Colony/update-staging/` | Emptied at every start |

## GitHub API

- Per-URL ETag cache, persisted across launches (a 304 Not Modified is free
  for authenticated requests; anonymous 304s still count against the quota)
- Per-URL locks to prevent race conditions
- Automatic pagination (`per_page=100`, loops until empty page)
- Rate-limit aware (warning at <10 remaining, error at 0)
- Works without token (public rate limit) and with token (5000 req/h)

## Launcher self-update

1. Compares `CARGO_PKG_VERSION` vs latest release from `Project-Colony/Colony`
2. Downloads the platform-specific binary to `update-staging/`
3. Replacement sequence: backup to `.old` → write the signature-verified
   bytes (never a re-read of the staged file) → chmod 755
4. Automatic rollback if copy fails
5. Spawns the new binary → exits the old one ONLY if the spawn succeeded

## Tests

Unit tests (`cargo test`) cover:
- `colony.json` manifest parsing (full, minimal, with pattern, with archives)
- Platform auto-detection from release assets
- `release_files` construction from assets
- SHA256 verification
- ZIP and tar.gz extraction
- Environment variable expansion
- Application categorization
- Section filters
- Localization (EN/FR, key parity between languages)
- Preferences serialization, and the migration of a file an older Colony wrote
- Preferences category order
- `validate-manifest` exit codes (0 clean, 1 invalid, 2 unreadable)
- Escaped Nerd Font glyphs in categories.json, shields colour names
- Update-loop state transitions (catalog refresh, update queue, badges,
  launcher-check outcomes, cancel semantics) via a hermetic test App
- filePattern globs with exclusions, signature parsing (strict ed25519),
  typed HTTP-status classification
- Resumable downloads end to end, against a local Range server that truncates
  its first response
- Remote-string URL containment (a hostile `tag` cannot leave the org)
- Platform-gated sidebar sections, transient vs. permanently-broken manifests
