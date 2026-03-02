# Colony Architecture

## Overview

Colony is an application launcher written in Rust with Iced 0.14 (Elm architecture). It handles discovery, installation, updating, and launching of both local and remote applications (via GitHub).

## Tech stack

| Component | Technology |
|-----------|-----------|
| Language | Rust (edition 2021) |
| UI | Iced 0.14 (Elm architecture) |
| Async | Tokio (integrated via Iced runtime) |
| HTTP | reqwest (async + streaming) |
| Auth | GitHub Device Flow OAuth |
| Secret storage | keyring (OS keychain) + file fallback |
| Serialization | serde + serde_json |
| Configuration | TOML (scan dirs), JSON (categories, preferences) |
| Versioning | semver |
| Integrity | SHA256 (sha2 crate) |
| Archives | zip, flate2 + tar |

## File structure

```
src/
├── main.rs          — Bootstrap, App state, boot(), update(), view()
├── state.rs         — App struct (global state), GitHubState, UI fields
├── message.rs       — Message enum (all events)
├── update.rs        — Handlers for each Message variant
├── github.rs        — GitHub API, ETag cache, manifests, downloads,
│                      platform auto-detection, launcher self-update
├── oauth.rs         — Device Flow OAuth (login, token, keychain)
├── scan.rs          — System application scanning (Linux/Windows/macOS)
├── sections.rs      — Categories, origin/category filters, JSON config
├── i18n.rs          — FR/EN localization with variable substitution
└── ui/
    ├── mod.rs       — UI module declarations
    ├── theme.rs     — 24 theme families, 50+ palettes, semantic tokens
    ├── sidebar.rs   — Sidebar (sections, GitHub, rescan, update badge)
    ├── app_grid.rs  — Application card grid with search
    ├── detail.rs    — Detail view (README, changelog, license, actions)
    ├── settings.rs  — Settings panel (theme, language, about, updates)
    └── github_panel.rs — GitHub connect/disconnect, Device Flow UI
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
- **Tokens**: Stored in OS keychain, file fallback (chmod 600)
- **Downloads**: HTTPS only, optional SHA256 verification
- **Timeouts**: 30s API requests, 10s connect, 300s downloads
- **Self-update**: Binary backup before replacement, automatic rollback on failure

## Cache and persistence

| Data | Location | Duration |
|------|----------|----------|
| Colony repos (cache) | `~/.cache/colony/repos_cache.json` | Offline fallback |
| Scanned apps (cache) | `~/.cache/colony/scan_cache.json` | Session |
| Repo docs (cache) | `~/.cache/colony/docs/<repo>/` | Offline fallback |
| Preferences | `~/.config/colony/preferences.json` | Permanent |
| Favorites | `~/.config/colony/favorites.json` | Permanent |
| OAuth token | OS Keychain / `~/.config/colony/github_token.json` | Permanent |
| Installed versions | `~/.local/share/Colony/apps/<repo>/.colony_version` | Permanent |
| Resolved asset | `~/.local/share/Colony/apps/<repo>/.colony_asset` | Permanent |
| Colony binaries | `~/.local/share/Colony/apps/<repo>/` | Permanent |
| Self-update staging | `~/.local/share/Colony/update-staging/` | Temporary |

## GitHub API

- Per-URL ETag cache (304 Not Modified — avoids consuming rate limit)
- Per-URL locks to prevent race conditions
- Automatic pagination (`per_page=100`, loops until empty page)
- Rate-limit aware (warning at <10 remaining, error at 0)
- Works without token (public rate limit) and with token (5000 req/h)

## Launcher self-update

1. Compares `CARGO_PKG_VERSION` vs latest release from `Project-Colony/Colony`
2. Downloads the platform-specific binary to `update-staging/`
3. Replacement sequence: backup to `.old` → copy new binary → chmod 755
4. Automatic rollback if copy fails
5. Spawns the new binary → exits the old one

## Tests

62 unit tests covering:
- `colony.json` manifest parsing (full, minimal, with pattern, with archives)
- Platform auto-detection from release assets
- `release_files` construction from assets
- SHA256 verification
- ZIP and tar.gz extraction
- Environment variable expansion
- Application categorization
- Section filters
- Localization (EN/FR)
- Preferences serialization
