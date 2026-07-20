# Specification — Colony App Discovery via GitHub

## Context

Colony is an application launcher that downloads, updates, and launches software from the Colony ecosystem. Applications are organized into **internal Colony categories**, independent of platforms.

## Official source

- **Official GitHub organization:** `Project-Colony` (https://github.com/Project-Colony)
- Colony automatically scans **all GitHub repositories** in the Project-Colony organization to detect compatible software.

## Detecting a Colony app

A repository is recognized as a Colony app **only if** a manifest file named **`colony.json`** is present **at the root** of the repository.

The manifest serves to:
- identify the software,
- define the **category** in which it appears in Colony,
- optionally define compatible platforms and download files.

### `colony.json` format

#### Minimal format (auto-detection)

```json
{
  "name": "orCAL",
  "category": "Utilities"
}
```

When `platforms` and `releaseFiles` are omitted, Colony auto-detects available platforms by inspecting the latest GitHub release assets. It matches the naming convention `{repo-name}-{platform}`:

| Asset name | Detected platform |
|---|---|
| `orcal-linux` | linux |
| `orcal-windows.exe` | windows |
| `orcal-macos` | macos (Apple Silicon) |
| `orcal-macos-x86` | macos-x86 (Intel) |

This is the **recommended format** for new Colony apps using the standard release workflow.

#### Full format (explicit files)

```json
{
  "name": "Lilypad",
  "category": "Security",
  "platforms": ["windows", "linux", "macos"],
  "releaseFiles": {
    "windows": {
      "tag": "latest",
      "file": "lilypad-x86_64-pc-windows-msvc.zip",
      "binary": "lilypad-cli.exe",
      "sha256": "..."
    },
    "linux": {
      "tag": "latest",
      "file": "lilypad-x86_64-unknown-linux-gnu.tar.gz",
      "binary": "lilypad-cli"
    },
    "macos": {
      "tag": "latest",
      "file": "lilypad-aarch64-apple-darwin.tar.gz",
      "binary": "lilypad-cli"
    }
  }
}
```

#### Full format with pattern matching

```json
{
  "name": "Lilypad",
  "category": "Security",
  "platforms": ["windows", "linux", "macos"],
  "releaseFiles": {
    "windows": {
      "tag": "latest",
      "filePattern": "windows",
      "binary": "lilypad-cli.exe"
    },
    "linux": {
      "tag": "latest",
      "filePattern": "linux",
      "binary": "lilypad-cli"
    },
    "macos": {
      "tag": "latest",
      "filePattern": "darwin",
      "binary": "lilypad-cli"
    }
  }
}
```

With `filePattern`, Colony fetches the release asset list and finds the one whose name contains the pattern (case-insensitive). If the asset name changes between releases (e.g. `lilypad-v1-windows.zip` → `lilypad-v2-windows.zip`), the pattern `"windows"` still works.

Three matching modes are supported (an exact asset-name match always wins and is never ambiguous):

- **Substring** (legacy): a pattern without `*` matches any asset whose name *contains* it.
- **Anchored glob**: a pattern containing `*` must match the *whole* asset name (`*` spans any characters). Example: `"colony-*.tar.gz"`.
- **Glob with exclusions**: comma-separated terms where a leading `!` excludes. Example for an electron-builder layout that publishes both architectures: `"SphereCord-*.AppImage, !*-arm64*"`.

Signature and checksum siblings (`.sig`, `.sha256`, `.yml`, ...) are never matched, so adding release signing to an app cannot break its pattern.

### Manifest fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | yes | Display name in Colony. Should match the repository name. |
| `category` | `string` | yes | Colony category, as shown in the sidebar (case-insensitive). Values: `Development`, `Graphics`, `Network`, `Office`, `Multimedia`, `System`, `Utility` (alias `Utilities`), `Security`, `Game` (alias `Games`), `Other`. Unknown values fall back to `Other`. |
| `platforms` | `string[]` | no | Supported platforms. Values: `"windows"`, `"linux"`, `"macos"` (Apple Silicon), `"macos-x86"` (Intel). Auto-detected if omitted. |
| `releaseFiles` | `object` | no | Map of platform → release entry. Each key is a platform. Auto-detected if omitted. |
| `icon` | `string` | no | Path (relative to the repo root) to a square **PNG** app icon shown in the Colony grid. See [App icon](#app-icon). |

### `releaseFiles` entry fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `tag` | `string` | yes | GitHub release tag. Can be a versioned tag (`"v1.0.0"`) or `"latest"` for automatic resolution via `GET /releases/latest`. |
| `file` | `string` | no* | Exact filename to download from the release. Can be a raw binary or an archive (.zip, .tar.gz). |
| `filePattern` | `string` | no* | Pattern matched against release asset names (case-insensitive): substring, anchored glob (`*`), or comma-separated glob terms with `!` exclusions. Error if 0 or >1 assets match. |
| `binary` | `string` | no | Binary name to extract from the archive. If absent, the downloaded file is the final binary. If present, Colony extracts the binary from the archive. |
| `sha256` | `string` | no | SHA256 hash of the downloaded file for integrity verification. |

\* `file` or `filePattern`: one of the two is required when `releaseFiles` is specified explicitly. `file` for an exact name, `filePattern` for dynamic detection.

## Versioning and updates

- Colony retrieves versions via **GitHub tags** in `vX.Y.Z` format.
- If the `tag` field is `"latest"`, Colony dynamically resolves the latest tag via `GET /repos/{owner}/{repo}/releases/latest` before downloading. The resolved tag (e.g. `v0.1.1`) is then saved locally for version tracking.
- Colony automatically downloads the **latest compatible release** for the user's platform.
- **Local vs remote version** comparison:
  - if a newer version exists → **update prompt**,
  - otherwise → **direct launch**.

## Archive support

Colony supports downloading archives containing the binary:
- **Supported formats**: `.zip`, `.tar.gz`, `.tgz`
- When the `binary` field is present in a `releaseFiles` entry, Colony treats `file` as an archive and extracts the named binary.
- The archive is automatically deleted after extraction.
- SHA256 verification applies to the downloaded archive (before extraction).

## Metadata enrichment

Colony extracts information from the repository to enrich the display:

- **Title**: repository name (e.g. `orCAL`). Matches the `name` field in the manifest.
- **Description**: **README** content from the repository (fetched via GitHub API). Falls back to the GitHub repository description if the README is absent or empty.
- **Primary language**: via the GitHub API (dominant language of the repository).
- **Platforms**: from the `platforms` field of the manifest (or auto-detected). Displayed as tags in the detail view.
- **Category**: from the `category` field. Determines which sidebar section the application appears in.
- **License**: fetched from `LICENSE`, `LICENSE.md`, or `LICENSE.txt`.
- **Changelog**: fetched from `CHANGELOG.md`, `CHANGES.md`, or `CHANGELOG`.

## App icon

Colony shows a per-app icon in the grid. It is resolved, in order:

1. The `icon` manifest field — a path relative to the repo root (e.g. `"assets/icons/icon.png"`), fetched from the repo like the README.
2. If `icon` is absent, a conventional **`icon.png`** at the repo root.
3. If neither exists, Colony draws a generated category tile (a tinted hexagon with the category glyph).

The icon is fetched once and cached on disk (`<data>/repo-icons/<repo>/icon.png`), decoded to an image, and rendered at a fixed ~54 px tile.

**Format**: a square **PNG** (transparent or opaque). PNG is the only format Colony decodes — not `.ico` / `.svg` / `.icns` (those are for a platform's *own* application icon, which is separate from the Colony grid icon). A 48–128 px source works well; pixel-art icons should be authored at their native size.

### Standard icon layout

Project-Colony repos keep every icon format in one uniform folder, so tooling (release workflows, `build.rs` resource embedding, future icon pipelines) can rely on the same path in every repo:

```
assets/icons/
├── icon.png    — canonical square PNG; the Colony grid icon (declared via `icon`)
├── icon.ico    — Windows application icon (multi-size container, 16–256 px)
└── icon.icns   — macOS application icon (multi-size container)
```

```json
{ "icon": "assets/icons/icon.png" }
```

Declaring the path explicitly is preferred over relying on the root-`icon.png` convention: it documents the layout and spares Colony a probing request. The `.ico` / `.icns` files are not read by Colony — they are staged there for each app's own platform packaging (executable icon, taskbar, dock).

## GitHub OAuth authentication

Colony offers a **"Connect"** button that initiates a GitHub Device Flow OAuth.

### Flow

- **Device Flow**: user receives a code, opens a URL in their browser, enters the code to authorize Colony.
- Minimal scopes (public repos need no scope; auth increases rate limits to 5000 req/h).

### Token management

- Stored in the OS keychain (keyring crate), with a file fallback (chmod 600).
- Revocation via disconnect button (local deletion).
- Authenticated calls use: `Authorization: Bearer <token>`.

### Unauthenticated mode

- Without a token, Colony uses the public GitHub API.
- Public rate limits apply (60 req/h). A message is displayed when the quota is reached.

## Release signatures (optional, recommended)

When a release publishes a detached signature named `<asset>.sig` next to an
asset, Colony **requires** it to verify: the raw 64-byte ed25519 signature over
the asset bytes, made with the Project-Colony release key (the same key that
signs the launcher itself - see `docs/release-signing.md` and the embedded
public key in `src/signing.rs`). An invalid signature aborts the install.

An asset without a `.sig` sibling installs as before (legacy unsigned app) -
signing is adopted per-app, no flag day - **unless** the manifest declares:

```json
{ "signed": true }
```

With `"signed": true`, a missing signature ABORTS the install: this closes the
remaining hole where a compromised repository could simply omit signatures.
Declare it once every release of the app ships `.sig` assets (all Project-
Colony apps have signed releases as of 2026-07-20). Sign with:

```sh
COLONY_SIGNING_KEY=/path/to/colony-release.pem ./scripts/sign-release.sh <asset>
```

Note the trust boundary: signatures protect against tampered release assets
(e.g. an asset swapped after publication), because forging one requires the
org's private signing key, which never lives in any repository. They do not
yet protect against a fully compromised repository *omitting* signatures;
enforcement ("this app must be signed") is a planned follow-up.
