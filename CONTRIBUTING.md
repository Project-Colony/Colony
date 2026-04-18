# Contributing to Colony

Two types of contribution land here, each with a different path:

1. **Adding your app to the Colony catalog** — jump to [Adding your app to Colony](#adding-your-app-to-colony).
2. **Contributing to Colony itself** (the launcher) — jump to [Contributing to Colony itself](#contributing-to-colony-itself).

---

## Adding your app to Colony

Colony automatically scans **all public repositories** under the `Project-Colony` GitHub organization. A repo becomes a Colony app when:

1. A `colony.json` manifest sits at the repo root.
2. A GitHub release exists with assets matching the naming convention below.

### Step 1 — Prepare a `colony.json`

Minimal form:

```json
{
  "name": "YourApp",
  "category": "Utilities"
}
```

Valid categories: `Development`, `Graphics`, `Network`, `Office`, `Multimedia`, `System`, `Utilities`, `Games`, `Other`.

For the full reference (pinned releases, explicit platforms, archive extraction, SHA256, pattern-matched asset names, etc.), read [docs/colony-spec.md](docs/colony-spec.md).

### Step 2 — Publish release assets

Colony auto-detects platforms from release asset filenames using the pattern `<repo-name-lowercase>-<platform>`:

| Asset name               | Detected platform           |
|--------------------------|-----------------------------|
| `yourapp-linux`          | Linux x86_64                |
| `yourapp-windows.exe`    | Windows x86_64              |
| `yourapp-macos`          | macOS ARM (Apple Silicon)   |
| `yourapp-macos-x86`      | macOS Intel                 |

You can upload any subset — Colony only advertises the platforms whose asset is present. Assets can be raw binaries **or** `.zip` / `.tar.gz` archives; see the spec if you archive.

### Step 3 — Wire up a release workflow (Rust apps)

A ready-to-use GitHub Actions template is in this repo at [`.github/workflows/colony-rust-release.yml.template`](.github/workflows/colony-rust-release.yml.template). Copy it to your own repo as `.github/workflows/release.yml` and replace `{{APP_NAME}}` with your binary/repo name (lowercase).

It uses [`release-please`](https://github.com/googleapis/release-please) so every merged PR tagged with a conventional-commit prefix (`feat:`, `fix:`, etc.) opens a release PR; merging that PR tags the version, builds the matrix of 4 platforms, and uploads the assets under the convention above. Zero manual release work afterwards.

Non-Rust apps: replicate the same asset naming convention with whatever tooling you prefer (electron-builder, pyinstaller, go build, etc.).

### Step 4 — Get your repo into Project-Colony

Two paths:

- **Transfer** your existing repo into the `Project-Colony` organization (GitHub → repo Settings → Transfer ownership). Simplest if you're the sole owner.
- **Fork it under the org** if you want to keep the canonical elsewhere. Colony picks up the fork as long as it has `colony.json` at root and releases with matching assets.

Either way, ping a maintainer (open an issue on `Project-Colony/Colony`) with your repo URL so they can approve the transfer/fork and confirm detection.

### Step 5 — Verify detection

On your machine:

```bash
colony   # launch Colony, wait a few seconds for the next catalog refresh
```

Your app card should appear in the category you declared. If not:

- Make sure the release is **published** (not a draft).
- Make sure asset names match the convention (lowercase, no version in the name, correct extension).
- Look in `~/.cache/colony/repos_cache.json` — if your repo is there but without platforms, the `colony.json` or the asset names are wrong. See the [spec](docs/colony-spec.md) for the precise validation rules.
- Verify you aren't hitting the GitHub rate limit: Settings → GitHub → Connect.

---

## Contributing to Colony itself

Issues, bug fixes, features, and documentation improvements to the Colony launcher are welcome.

### Dev setup

```bash
git clone https://github.com/Project-Colony/Colony.git
cd Colony
cargo build              # debug build
cargo run                # launch the launcher in debug mode
cargo test --lib         # run unit tests (62 tests across manifest parsing, scanning, i18n, etc.)
cargo build --release    # optimized build
```

**Linux runtime dependencies** (for building and running):

```
libgtk-3-dev libxdo-dev libdbus-1-dev libasound2-dev libglib2.0-dev pkg-config
```

### Code organization

See [docs/architecture.md](docs/architecture.md) for the full layout. Short version:

- `src/main.rs` — entry, Elm-architecture `update()` / `view()`.
- `src/update.rs` — `Message` handlers.
- `src/github.rs` — GitHub API, manifest fetching, release asset resolution.
- `src/scan.rs` — system app detection (Linux `.desktop`, Windows Start Menu, macOS `.app`).
- `src/sections.rs` — categories + filter logic.
- `src/ui/` — widgets and panels (sidebar, app grid, detail view, settings).
- `src/ui/theme.rs` — all 24 theme families + 50+ palettes.

### Style

- Rust edition 2021, `cargo fmt` before commit (`rustfmt.toml` is committed).
- Prefer small focused commits with [Conventional Commits](https://www.conventionalcommits.org) prefixes (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`, `ci:`) so release-please can detect release-worthy changes and bump versions automatically.
- Keep platform-specific code behind `#[cfg(target_os = "...")]` so every platform still compiles.
- If you touch the public `colony.json` schema, also update [docs/colony-spec.md](docs/colony-spec.md) in the same PR.

### Before you open a PR

1. `cargo fmt`
2. `cargo clippy -- -D warnings` (treat warnings as errors — CI does)
3. `cargo test`
4. `cargo build --release` (catches linker issues absent in debug)
5. Rebase on top of `main` to keep history clean.

### What kind of change

| Size                           | Path                                                                 |
|--------------------------------|----------------------------------------------------------------------|
| Typo / doc clarification       | PR directly.                                                         |
| Bug fix                        | PR directly, reference the issue if any.                             |
| Small feature (new theme, sidebar tweak) | PR directly, include before/after screenshots if UI-visible. |
| Large feature (new auth mode, protocol change) | Open an issue first to discuss design.              |

### Reporting bugs / feature requests

Use the templates in `.github/ISSUE_TEMPLATE/` when filing. Include Colony version (`colony --version`), OS, and relevant log excerpts from `~/.cache/colony/`.

---

## Code of conduct

Be kind. We're all here to ship nice software. Personal attacks, harassment, and hostile behaviour get the offender removed from the org. Lin / the maintainers have final say.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
