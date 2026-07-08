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
  app installs continue to use the optional `sha256` field in `colony.json`.

## Every release MUST ship signatures

Because verification is fail-closed, a release published **without** the
`colony-<platform>.sig` assets will make self-update fail for users on that
channel. Sign every launcher asset and upload the `.sig` files alongside them.

## Signing (CI or local)

The private key never lives in the repo. Point `COLONY_SIGNING_KEY` at the
ed25519 private key (PEM) and run:

```sh
COLONY_SIGNING_KEY=/path/to/colony-release.pem \
  ./scripts/sign-release.sh colony-linux colony-windows.exe colony-macos colony-macos-x86
```

This writes `colony-linux.sig`, `colony-windows.exe.sig`, … each self-verified
before it is written. Upload every `.sig` as a release asset.

### GitHub Actions sketch

```yaml
- name: Sign release assets
  env:
    KEY: ${{ secrets.COLONY_SIGNING_KEY_PEM }}   # the PEM contents
  run: |
    printf '%s' "$KEY" > /tmp/colony-release.pem
    chmod 600 /tmp/colony-release.pem
    COLONY_SIGNING_KEY=/tmp/colony-release.pem \
      ./scripts/sign-release.sh dist/colony-*
    rm -f /tmp/colony-release.pem
- name: Upload signatures
  run: gh release upload "$TAG" dist/colony-*.sig
```

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
