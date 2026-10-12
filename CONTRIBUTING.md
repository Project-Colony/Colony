# Contributing to Colony

Two types of contribution land here, each with a different path:

1. **Adding your app to the Colony catalog**: jump to [Adding your app to Colony](#adding-your-app-to-colony).
2. **Contributing to Colony itself** (the launcher): jump to [Contributing to Colony itself](#contributing-to-colony-itself).

---

## Adding your app to Colony

Colony automatically scans **all public repositories** under the `Project-Colony` GitHub organization. A repo becomes a Colony app when:

1. A `colony.json` manifest sits at the repo root.
2. A GitHub release exists with assets matching the naming convention below.

### Step 1: Prepare a `colony.json`

Minimal form:

```json
{
  "$schema": "https://raw.githubusercontent.com/Project-Colony/Project-Colony-Resources/main/generated/colony.schema.json",
  "name": "YourApp",
  "category": "utility"
}
```

Valid categories: `development`, `graphics`, `network`, `office`, `multimedia`, `system`, `utility`, `security`, `game`, `other` (case-insensitive; `utilities` and `games` are accepted too). The `$schema` line lets your editor check the file as you type.

**App icon (recommended):** put a square PNG at `assets/icons/icon.png` and declare it in the manifest: `"icon": "assets/icons/icon.png"`. Colony shows it in the grid; without it your app gets a generated category tile. The same folder is the standard home for your `icon.ico` (Windows) and `icon.icns` (macOS). See the [App icon](docs/guide/colony-spec.md#app-icon) section of the spec.

For the full reference (pinned releases, explicit platforms, archive extraction, SHA256, pattern-matched asset names, etc.), read [docs/guide/colony-spec.md](docs/guide/colony-spec.md).

### Step 2: Publish release assets

Colony auto-detects platforms from release asset filenames using the pattern `<repo-name-lowercase>-<platform>`:

| Asset name               | Detected platform           |
|--------------------------|-----------------------------|
| `yourapp-linux`          | Linux x86_64                |
| `yourapp-windows.exe`    | Windows x86_64              |
| `yourapp-macos`          | macOS ARM (Apple Silicon)   |
| `yourapp-macos-x86`      | macOS Intel                 |

You can upload any subset: Colony only advertises the platforms whose asset is present. Assets can be raw binaries **or** `.zip` / `.tar.gz` archives; see the spec if you archive.

### Step 3: Wire up a release workflow (Rust apps)

The ready-to-use GitHub Actions template lives in Project-Colony-Resources at [`templates/sign-and-publish-caller.yml`](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/templates/sign-and-publish-caller.yml). Copy it to your own repo as `.github/workflows/release.yml` and replace `{{APP_NAME}}` with your binary/repo name (lowercase). Its build jobs never see a signing key: signing and publishing happen in the organisation's [shared signing workflow](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#shared-signing-workflow), which the template calls.

It uses [`release-please`](https://github.com/googleapis/release-please): merged pull requests with a conventional-commit title (`feat:`, `fix:` and so on) are collected into a release PR, and merging that PR tags the version, builds the 4 platforms, then signs and publishes the assets under the convention above. A release ships only when its release PR is merged; there is no manual release work.

Non-Rust apps: replicate the same asset naming convention with whatever tooling you prefer (electron-builder, pyinstaller, go build, etc.).

### Step 4: Get your repo into Project-Colony

Two paths:

- **Transfer** your existing repo into the `Project-Colony` organization (GitHub → repo Settings → Transfer ownership). Simplest if you're the sole owner.
- **Fork it under the org** if you want to keep the canonical elsewhere. Colony picks up the fork as long as it has `colony.json` at root and releases with matching assets.

Either way, ping a maintainer (open an issue on `Project-Colony/Colony`) with your repo URL so they can approve the transfer/fork and confirm detection.

### Step 5: Verify detection

On your machine:

```bash
colony   # launch Colony, wait a few seconds for the next catalog refresh
```

Your app card should appear in the category you declared. If not:

- Make sure the release is **published** (not a draft).
- Make sure asset names match the convention (lowercase, no version in the name, correct extension).
- Look in `~/.cache/Colony/Colony/repos_cache.json` (on macOS `~/Library/Caches/Colony/Colony/`, on Windows `%LOCALAPPDATA%\Colony\Colony\cache\`): if your repo is there but without platforms, the `colony.json` or the asset names are wrong. `colony validate-manifest colony.json <asset names>` says which. See the [spec](docs/guide/colony-spec.md) for the precise validation rules.
- Verify you aren't hitting the GitHub rate limit: click **GitHub** in Colony's sidebar, then **Sign in with GitHub**.

---

## Contributing to Colony itself

Issues, bug fixes, features, and documentation improvements to the Colony launcher are welcome.

### Dev setup

```bash
git clone https://github.com/Project-Colony/Colony.git
cd Colony
cargo build              # debug build
cargo run                # launch the launcher in debug mode
cargo test               # run the unit tests (manifest parsing, scanning, i18n, signing, etc.)
cargo build --release    # optimized build
```

You need Rust 1.88 or newer (the `rust-version` floor, checked by CI) and a C compiler. No system libraries are needed: on Linux, the D-Bus client used for the Secret Service is compiled from source.

### Code organization

See [docs/internals/architecture.md](docs/internals/architecture.md) for the full layout. Short version:

- `src/main.rs` - entry point: CLI flags, startup order, the iced launch.
- `src/app.rs` - Elm-architecture `view()`, `subscription()` and `theme()`; `src/update/` - `Message` handlers.
- `src/github/` - GitHub API layer: manifest fetching, release asset resolution, update checks.
- `src/download/` - release/asset downloads, archive extraction, launcher self-update.
- `src/signing.rs` - ed25519 verification of launcher updates and app releases (see [docs/internals/release-signing.md](docs/internals/release-signing.md)).
- `src/persistence.rs` - data directories, preferences, install state, on-disk caches, favorites.
- `src/config.rs` - locating external config files (`categories.json`, `colony.toml`).
- `src/scan.rs` - system app detection (Linux `.desktop`, Windows Start Menu, macOS `.app`).
- `src/sections.rs` - categories + filter logic.
- `src/ui/` - widgets and panels (sidebar, app grid, detail view, Preferences), on colony-ui's kit.
- `src/ui/theme.rs`: a re-export of [colony-ui](https://crates.io/crates/colony-ui)
  plus Colony's own button styling. The 26 theme families and 59 palettes live
  in that crate, generated from the design tokens in
  [Project-Colony-Resources](https://github.com/Project-Colony/Project-Colony-Resources).

### Style

- Rust edition 2021, `cargo fmt` before commit (default rustfmt config; CI enforces `cargo fmt --check`).
- Prefer small focused commits with [Conventional Commits](https://www.conventionalcommits.org) prefixes (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`, `ci:`) so release-please can detect release-worthy changes and bump versions automatically.
- Keep platform-specific code behind `#[cfg(target_os = "...")]` so every platform still compiles.
- If you touch the public `colony.json` schema, also update [docs/guide/colony-spec.md](docs/guide/colony-spec.md) in the same PR.

### Before you open a PR

1. `cargo fmt`
2. `cargo clippy --all-targets -- -D warnings` (treat warnings as errors, as CI does)
3. `cargo test`
4. `cargo build --release` (catches linker issues absent in debug)
5. Sync with `main` (merge or rebase) if it has advanced: every PR is squash-merged, so your branch collapses into a single commit on merge.

### What kind of change

| Size                           | Path                                                                 |
|--------------------------------|----------------------------------------------------------------------|
| Typo / doc clarification       | PR directly.                                                         |
| Bug fix                        | PR directly, reference the issue if any.                             |
| Small feature (sidebar tweak, new setting) | PR directly, include before/after screenshots if UI-visible. |
| A new theme family                          | Not here: add the TOML to [Project-Colony-Resources](https://github.com/Project-Colony/Project-Colony-Resources), regenerate, and bump `colony-ui`. Colony needs no change at all. |
| Large feature (new auth mode, protocol change) | Open an issue first to discuss design.              |

### Reporting bugs / feature requests

Use the templates in `.github/ISSUE_TEMPLATE/` when filing. Include Colony version (`colony --version`), OS, and relevant log excerpts from `~/.local/share/Colony/Colony/colony.log`.

---

## Code of conduct

Be kind. We're all here to ship nice software. Personal attacks, harassment, and hostile behaviour get the offender removed from the org. Lin / the maintainers have final say.

## License

By contributing, you agree that your contributions will be licensed under the [GNU General Public License v3.0 or later](LICENSE) (SPDX: `GPL-3.0-or-later`).
