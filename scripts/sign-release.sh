#!/usr/bin/env bash
#
# Sign a Colony release asset with the ed25519 release key, producing a detached
# "<asset>.sig" that the launcher verifies before applying a self-update.
#
# The signature is the raw 64-byte ed25519 signature over the asset bytes, as
# produced by `openssl pkeyutl -sign -rawin` — the same format src/signing.rs
# verifies against the embedded public key. openssl is the only dependency.
#
# Usage:
#   COLONY_SIGNING_KEY=/path/to/colony-release.pem ./scripts/sign-release.sh <asset> [<asset> ...]
#
# In CI, provide the private key via a secret (e.g. write it to a temp file from
# a GitHub Actions secret) and set COLONY_SIGNING_KEY to its path. Upload each
# generated "<asset>.sig" as a release asset alongside its binary.
set -euo pipefail

KEY="${COLONY_SIGNING_KEY:-$HOME/.config/colony/release-signing/colony-release.pem}"

if [[ ! -f "$KEY" ]]; then
  echo "error: signing key not found at '$KEY'" >&2
  echo "set COLONY_SIGNING_KEY to the ed25519 private key (PEM)." >&2
  exit 1
fi
if [[ $# -eq 0 ]]; then
  echo "usage: COLONY_SIGNING_KEY=<key.pem> $0 <asset> [<asset> ...]" >&2
  exit 2
fi

for asset in "$@"; do
  if [[ ! -f "$asset" ]]; then
    echo "error: asset not found: $asset" >&2
    exit 1
  fi
  sig="${asset}.sig"
  openssl pkeyutl -sign -inkey "$KEY" -rawin -in "$asset" -out "$sig"
  # Self-check: verify what we just produced before publishing it.
  pub="$(mktemp)"
  openssl pkey -in "$KEY" -pubout -out "$pub" 2>/dev/null
  if openssl pkeyutl -verify -pubin -inkey "$pub" -rawin -in "$asset" -sigfile "$sig" >/dev/null 2>&1; then
    echo "signed  $asset -> $sig"
  else
    echo "error: self-verification failed for $asset" >&2
    rm -f "$pub"
    exit 1
  fi
  rm -f "$pub"
done
