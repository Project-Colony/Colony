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
├── main.rs          — Crate root: modules, CLI flags, logging, entry point
├── state.rs         — App struct (global state), GitHubState, UI fields
├── message.rs       — Message enum (all events)
├── app.rs           — impl App: boot(), view(), subscription(), theme()
├── update/          — Message dispatch table (mod.rs) + handlers
│   ├── store.rs     — install / update / uninstall / notes / favorites
│   ├── github_auth.rs — device flow, catalog refresh
│   ├── launcher.rs  — self-update state machine
│   ├── preferences.rs, keyboard.rs, onboarding.rs
├── github/          — split by layer, mod.rs re-exports everything
│   ├── http.rs      — client, conditional-request cache, typed statuses
│   ├── types.rs     — colony.json / API wire shapes
│   ├── catalog.rs   — store listing
│   └── releases.rs  — tags, assets, platform auto-detection
├── download.rs      — Asset/archive downloads, extraction, app-signature
│                      verification, self-update
├── signing.rs       — ed25519 verification (launcher AND app releases)
├── icons.rs         — PNG decoding for per-app grid icons
├── persistence.rs   — Data dirs, install state, on-disk caches, favorites,
│                      desktop entries (Linux)
├── config.rs        — Locating external config (categories.json, colony.toml)
├── oauth.rs         — Device Flow OAuth (login, token, keychain)
├── scan.rs          — System application scanning (Linux/Windows/macOS)
├── sections.rs      — Categories, origin/category filters, JSON config
├── i18n/            — fr.rs, en.rs, and the Locale lookup (mod.rs)
└── ui/
    ├── mod.rs       — UI module declarations
    ├── theme.rs     — 26 theme families, 59 palettes, semantic tokens
    ├── sidebar.rs   — Sidebar (sections, GitHub, rescan, update badge)
    ├── app_grid.rs  — Application card grid with search
    ├── detail.rs    — Detail view (README, changelog, license, actions)
    ├── settings.rs  — Settings panel (theme, language, about, updates)
    ├── github_panel.rs — GitHub connect/disconnect, Device Flow UI
    ├── markdown_blocks.rs — Cached-block Markdown rendering
    └── tutorial.rs  — First-launch guided tour (spotlight overlay)
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
| Diagnostics log | `~/.cache/Colony/Colony/colony.log` | Truncated per run |
| Installed versions | `~/.local/share/Colony/apps/<repo>/.colony_version` | Permanent |
| Resolved asset | `~/.local/share/Colony/apps/<repo>/.colony_asset` | Permanent |
| Colony binaries | `~/.local/share/Colony/apps/<repo>/` | Permanent |
| Self-update staging | `~/.cache/Colony/Colony/update-staging/` | Temporary |

## GitHub API

- Per-URL ETag cache (304 Not Modified — avoids consuming rate limit)
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

131 unit tests covering:
- `colony.json` manifest parsing (full, minimal, with pattern, with archives)
- Platform auto-detection from release assets
- `release_files` construction from assets
- SHA256 verification
- ZIP and tar.gz extraction
- Environment variable expansion
- Application categorization
- Section filters
- Localization (EN/FR, key parity between languages)
- Preferences serialization
- Update-loop state transitions (catalog refresh, update queue, badges,
  launcher-check outcomes, cancel semantics) via a hermetic test App
- filePattern globs with exclusions, signature parsing (strict ed25519),
  typed HTTP-status classification
- Resumable downloads end to end, against a local Range server that truncates
  its first response
- Remote-string URL containment (a hostile `tag` cannot leave the org)
- Platform-gated sidebar sections, transient vs. permanently-broken manifests
