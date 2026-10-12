# colony.json specification: how Colony discovers apps on GitHub

## Context

Colony is an application launcher that downloads, updates, and launches software from the Colony ecosystem. Applications are organized into **internal Colony categories**, independent of platforms.

## Official source

- **Official GitHub organization:** `Project-Colony` (https://github.com/Project-Colony)
- Colony automatically scans **every public repository** of the Project-Colony organization to detect compatible software.

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
  "$schema": "https://raw.githubusercontent.com/Project-Colony/Project-Colony-Resources/main/generated/colony.schema.json",
  "name": "orCAL",
  "category": "utility"
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
  "$schema": "https://raw.githubusercontent.com/Project-Colony/Project-Colony-Resources/main/generated/colony.schema.json",
  "name": "Lilypad",
  "category": "security",
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
  "$schema": "https://raw.githubusercontent.com/Project-Colony/Project-Colony-Resources/main/generated/colony.schema.json",
  "name": "Lilypad",
  "category": "security",
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
| `category` | `string` | yes | Colony category, which picks the sidebar section (case-insensitive). Canonical values: `development`, `graphics`, `network`, `office`, `multimedia`, `system`, `utility`, `security`, `game`, `other`; `utilities` and `games` are accepted too. Unknown values fall back to `other`, and `colony validate-manifest` reports them. |
| `platforms` | `string[]` | no | Supported platforms. Values: `"windows"`, `"linux"`, `"macos"` (Apple Silicon), `"macos-x86"` (Intel). Auto-detected if omitted. |
| `releaseFiles` | `object` | no | Map of platform → release entry. Each key is a platform. Auto-detected if omitted. |
| `icon` | `string` | no | Path (relative to the repo root) to a square **PNG** app icon shown in the Colony grid. See [App icon](#app-icon). |
| `signed` | `boolean` | no | When `true`, every release asset MUST ship a valid `<asset>.sig`; a missing signature aborts the install instead of falling back to the legacy unsigned path. Defaults to `false`. See [Signed releases](#signed-releases). |

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

1. The `icon` manifest field: a path relative to the repo root (e.g. `"assets/icons/icon.png"`), fetched from the repo like the README.
2. If `icon` is absent, a conventional **`icon.png`** at the repo root.
3. If neither exists, Colony draws a generated category tile (a tinted hexagon with the category glyph).

The icon is fetched once and cached on disk (`<cache>/repo-icons/<repo>/icon.png`), decoded to an image, and rendered at a fixed ~54 px tile.

**Format**: a square **PNG** (transparent or opaque). PNG is the only format Colony decodes, not `.ico` / `.svg` / `.icns` (those are for a platform's *own* application icon, which is separate from the Colony grid icon). A 48 to 128 px source works well; pixel-art icons should be authored at their native size.

### Standard icon layout

Project-Colony repos keep every icon format in one uniform folder, so tooling (release workflows, `build.rs` resource embedding, future icon pipelines) can rely on the same path in every repo:

```
assets/icons/
├── icon.png    canonical square PNG; the Colony grid icon (declared via `icon`)
├── icon.ico    Windows application icon (multi-size container, 16 to 256 px)
└── icon.icns   macOS application icon (multi-size container)
```

```json
{ "icon": "assets/icons/icon.png" }
```

Declaring the path explicitly is preferred over relying on the root-`icon.png` convention: it documents the layout and spares Colony a probing request. The `.ico` / `.icns` files are not read by Colony: they are staged there for each app's own platform packaging (executable icon, taskbar, dock).

## GitHub OAuth authentication

Colony offers a **Sign in with GitHub** button, in the panel the sidebar's **GitHub** button opens, that starts a GitHub Device Flow OAuth.

### Flow

- **Device Flow**: user receives a code, opens a URL in their browser, enters the code to authorize Colony.
- Scope `read:user` only: enough to read the user name. Public repositories need no scope, and signing in only raises the rate limit to 5000 requests per hour.

### Token management

- Stored in the OS credential store (keyring-core with the macOS Keychain, Windows Credential Manager or Secret Service store), with a file fallback (chmod 600).
- **Sign out** deletes it locally; the authorisation itself is revoked from the user's GitHub settings.
- Authenticated calls use: `Authorization: Bearer <token>`.

### Unauthenticated mode

- Without a token, Colony uses the public GitHub API.
- Public rate limits apply (60 req/h). A message is displayed when the quota is reached.

## Release signatures (optional, recommended)

When a release publishes a detached signature named `<asset>.sig` next to an
asset, Colony **requires** it to verify: the raw 64-byte ed25519 signature over
the asset bytes, made with the Project-Colony release key (the same key that
signs the launcher itself: see [release-signing.md](../internals/release-signing.md) and the embedded
public key in [`src/signing.rs`](../../src/signing.rs)). An invalid signature aborts the install.

An asset without a `.sig` sibling installs as before (legacy unsigned app) -
signing is adopted per-app, no flag day - **unless** the manifest declares:

```json
{ "signed": true }
```

With `"signed": true`, a missing signature ABORTS the install. Declare it once
every release of the app ships `.sig` assets. Not every Project-Colony app
signs its releases yet, so check an app's release assets rather than assuming.
An app signs nothing itself: its release workflow, copied from the
[release workflow template](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/templates/sign-and-publish-caller.yml),
hands the built binaries to the organisation's shared signing workflow. That
workflow writes `<asset>.sig` and a signed `<asset>.meta` sidecar binding the
bytes to a version and a filename. The launcher's self-update requires the
sidecar, and app installs check it whenever it is published (see below).

Note the trust boundary. Signatures protect against tampered release assets
(e.g. an asset swapped after publication), because forging one requires the
org's private signing key, which never lives in any repository. Enforcement is
implemented: `"signed": true` refuses an install with no signature, and the
launcher **pins** the requirement client-side once an app has been installed with
a verified signature, so flipping `signed` back to false in a compromised
repository no longer downgrades that app to unsigned. Since Colony 0.10.0 the
binding gap is closed on the CLIENT side: an app asset
that publishes a signed `<asset>.meta` sidecar (plus `<asset>.meta.sig`) has its
bytes bound to an asset name, a digest and the resolved tag, and Colony refuses
anything older than what is installed. That stops the replay of a *different*
org-signed artefact under a new tag.

Sidecars are **opportunistic and pinned**: an app that publishes none still
installs, but once Colony has verified one for an app, a later release that
stops publishing them is refused. So the ecosystem can adopt them repo by repo
with no flag day, and no repo can quietly stop. The release template's
`sign-and-publish` job, which calls the shared signing workflow, emits them.

## Validating your manifest

Colony validates its own manifest format, so you do not have to guess:

```sh
colony validate-manifest colony.json
```

Pass the asset names your release publishes to also check that every platform
actually **resolves** - which is the failure that matters, because a manifest
can be structurally perfect and still leave your app listed with no Download
button:

```sh
gh release view v1.2.3 --json assets --jq '.assets[].name' > names.txt
colony validate-manifest colony.json $(tr '\n' ' ' < names.txt)
```

It exits non-zero on any problem, so it works as a CI gate. The shipped
[release workflow template](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/templates/sign-and-publish-caller.yml)
already runs it on every release, in its Linux build job, before anything is
signed.
