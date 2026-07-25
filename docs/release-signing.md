# Release signing

Colony verifies its own launcher self-updates with an **ed25519 signature**
before applying them. This is mandatory and fail-closed: if the detached
signature is missing, malformed, or does not verify against the embedded public
key, the self-update is refused and the running binary is left untouched.

- Verification: [`src/signing.rs`](../src/signing.rs) — embeds the public key and
  verifies with the pure-Rust `ed25519-dalek` crate (no OpenSSL in the shipped
  binary).
- Signature format: the **raw 64-byte ed25519 signature** over the asset bytes,
  exactly what `openssl pkeyutl -sign -rawin` emits (base64 text is also
  accepted). Published as `<asset>.sig` next to each release asset.
- Enforced only for the **launcher** (`colony-<platform>[.exe]`). Third-party
  app installs continue to use the optional `sha256` field in `colony.json`,
  plus `"signed": true` to require a signature (pinned client-side once an app
  has been installed with one, so a repo cannot silently stop signing).

### Why a signature alone is not enough

A signature over raw bytes proves only *these bytes came from the release key* —
not **which** artefact or **which** version they are. Anything able to control
what the release host serves could therefore replay an older, genuinely signed
build as an "update" (a downgrade), or serve the macOS asset where the Linux one
was requested. So every asset also gets a **signed metadata sidecar**:

```
<asset>.meta        version=v1.2.3
                    asset=colony-linux
                    sha256=<hex of the asset bytes>
<asset>.meta.sig    ed25519 signature over the .meta bytes
```

The launcher verifies the sidecar's signature, then requires that it names the
asset it asked for, that its digest matches the bytes actually downloaded, that
its version equals the tag the update check resolved, and that this version is
**strictly newer** than the running build. That last check is the anti-rollback.
Both the signature and the sidecar are re-verified at install time, not only at
download time, so the staging file cannot be swapped in between.

## Every release MUST ship signatures and sidecars

Because verification is fail-closed, a release published without the
`colony-<platform>.sig`, `.meta` and `.meta.sig` assets will make self-update
fail for users on that channel. The CI `sign` job checks all three exist for all
four platforms and fails the release otherwise.

## Signing (CI or local)

The private key never lives in the repo. Point `COLONY_SIGNING_KEY` at the
ed25519 private key (PEM), set `COLONY_RELEASE_VERSION` to the release tag (it is
bound into each sidecar), and run:

```sh
COLONY_SIGNING_KEY=/path/to/colony-release.pem \
COLONY_RELEASE_VERSION=v1.2.3 \
  ./scripts/sign-release.sh colony-linux colony-windows.exe colony-macos colony-macos-x86
```

For each asset this writes `<asset>.sig`, `<asset>.meta` and `<asset>.meta.sig`,
every signature self-verified before it is kept. Upload all of them as release
assets.

### In CI (the normal path)

Since the v0.7.0 incident (a release shipped unsigned because signing was a
manual step, bricking self-update for every existing install), signing is a
mandatory job in the release workflow: `.github/workflows/release-please.yml`
(`sign` job) downloads the four built binaries, signs them with the
`COLONY_SIGNING_KEY_PEM` secret (the PEM contents), verifies that every asset
came out with a `.sig`, a `.meta` and a `.meta.sig`, and uploads them. The job
**fails the release** if the secret is missing or if any of those files is
missing or empty, so a release the launcher cannot verify can no longer ship
silently. The manual procedure above remains for re-signing an old release by
hand.

## Key custody

- The current private key was generated locally and stored at
  `~/.config/colony/release-signing/colony-release.pem` (mode `600`), with the
  public key beside it (`colony-release.pub.pem`). **Back it up somewhere
  durable and secret** (password manager / offline media). If it is lost, you
  must rotate (below); if it leaks, rotate immediately.
- For CI, store the PEM contents as an encrypted secret
  (`COLONY_SIGNING_KEY_PEM`), not in the repo.

## Generating / rotating the key

```sh
# 1. New keypair
openssl genpkey -algorithm ed25519 -out colony-release.pem
openssl pkey -in colony-release.pem -pubout -out colony-release.pub.pem

# 2. Extract the raw 32-byte public key (ed25519 SPKI = 12-byte header + 32-byte key)
openssl pkey -pubin -in colony-release.pub.pem -outform DER | tail -c 32 | xxd -i
```

Paste the 32 bytes from step 2 into `RELEASE_PUBLIC_KEY` in
[`src/signing.rs`](../src/signing.rs), ship a Colony release built with the new
key, and sign all subsequent assets with the new private key. Note: clients on
an old build trust only the old key, so keep signing with the old key until
those clients have updated (or accept that they can no longer self-update and
must reinstall).

## Verifying a signature by hand

```sh
openssl pkey -pubin -in colony-release.pub.pem -out /dev/null   # sanity: key parses
openssl pkeyutl -verify -pubin -inkey colony-release.pub.pem \
  -rawin -in colony-linux -sigfile colony-linux.sig
# -> "Signature Verified Successfully"
```
