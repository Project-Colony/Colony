#!/usr/bin/env bash
#
# Sign a Colony release asset with the ed25519 release key, producing a detached
# "<asset>.sig" that the launcher verifies before applying a self-update.
#
# The signature is the raw 64-byte ed25519 signature over the asset bytes, as
# produced by `openssl pkeyutl -sign -rawin` — the same format src/signing.rs
# verifies against the embedded public key. openssl is the only dependency.
#
# A signature over raw bytes proves only "these bytes came from the org" — not
# WHICH artefact or WHICH version they are. So each asset also gets a signed
# metadata sidecar binding the bytes to a version and a filename:
#
#   <asset>.meta      version=<tag>\nasset=<basename>\nsha256=<hex>\n
#   <asset>.meta.sig  ed25519 signature over the .meta bytes
#
# The launcher verifies the sidecar and refuses anything that is not strictly
# newer than itself; app installs verify the same sidecar and refuse anything
# OLDER than what is already installed (equal is fine - an app pinned to a fixed
# tag must stay reinstallable). Either way, that is what stops a replay of an
# older org-signed build.
#
# Emitting sidecars for an app repo is safe to adopt at any time: Colony treats
# them as opportunistic, and only makes them mandatory for a given app once it
# has verified one (so a repo cannot silently stop publishing them).
#
# Usage:
#   COLONY_SIGNING_KEY=/path/to/colony-release.pem \
#   COLONY_RELEASE_VERSION=v1.2.3 ./scripts/sign-release.sh <asset> [<asset> ...]
#
# In CI, provide the private key via a secret (e.g. write it to a temp file from
# a GitHub Actions secret) and set COLONY_SIGNING_KEY to its path. Upload every
# generated "<asset>.sig", "<asset>.meta" and "<asset>.meta.sig" as release
# assets alongside their binary.
set -euo pipefail

KEY="${COLONY_SIGNING_KEY:-$HOME/.config/colony/release-signing/colony-release.pem}"
VERSION="${COLONY_RELEASE_VERSION:-}"

if [[ ! -f "$KEY" ]]; then
  echo "error: signing key not found at '$KEY'" >&2
  echo "set COLONY_SIGNING_KEY to the ed25519 private key (PEM)." >&2
  exit 1
fi
if [[ $# -eq 0 ]]; then
  echo "usage: COLONY_SIGNING_KEY=<key.pem> COLONY_RELEASE_VERSION=<tag> $0 <asset> [<asset> ...]" >&2
  exit 2
fi
if [[ -z "$VERSION" ]]; then
  echo "error: COLONY_RELEASE_VERSION is not set" >&2
  echo "it is bound into each signed .meta sidecar so the launcher can reject downgrades." >&2
  exit 2
fi

# Sign <file> into <file>.sig, then verify the signature before returning.
sign_and_verify() {
  local file="$1" sig="$1.sig" pub
  openssl pkeyutl -sign -inkey "$KEY" -rawin -in "$file" -out "$sig"
  pub="$(mktemp)"
  openssl pkey -in "$KEY" -pubout -out "$pub" 2>/dev/null
  if ! openssl pkeyutl -verify -pubin -inkey "$pub" -rawin -in "$file" -sigfile "$sig" >/dev/null 2>&1; then
    echo "error: self-verification failed for $file" >&2
    rm -f "$pub"
    return 1
  fi
  rm -f "$pub"
}

for asset in "$@"; do
  if [[ ! -f "$asset" ]]; then
    echo "error: asset not found: $asset" >&2
    exit 1
  fi
  # Detached signature over the asset bytes.
  sign_and_verify "$asset"
  echo "signed  $asset -> ${asset}.sig"

  # Signed metadata binding those bytes to a version and a filename.
  meta="${asset}.meta"
  digest="$(openssl dgst -sha256 -r "$asset" | cut -d' ' -f1)"
  printf 'version=%s\nasset=%s\nsha256=%s\n' "$VERSION" "$(basename "$asset")" "$digest" > "$meta"
  sign_and_verify "$meta"
  echo "signed  $meta -> ${meta}.sig  (version=$VERSION sha256=${digest:0:16}...)"
done
