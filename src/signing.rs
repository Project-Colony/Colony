//! Detached ed25519 signature verification for signed launcher self-updates.
//!
//! Colony's own release binaries are signed with an ed25519 private key held
//! off-machine (see `docs/release-signing.md`); the matching public key is
//! embedded below. Before a launcher self-update is applied, the downloaded
//! binary is verified against a detached `<asset>.sig` signature. Verification
//! is mandatory and fail-closed: a missing, malformed, or invalid signature
//! aborts the update rather than installing untrusted code.
//!
//! The signature format is the raw 64-byte ed25519 signature emitted by
//! `openssl pkeyutl -sign -rawin` (base64 text is also accepted), so releases
//! can be signed with the ubiquitous `openssl` CLI in CI — no special tooling.

use anyhow::Result;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Colony release signing public key (ed25519, raw 32 bytes).
///
/// To rotate: generate a new key (see `docs/release-signing.md`), replace these
/// bytes with the new raw public key, and sign all subsequent releases with the
/// matching private key.
const RELEASE_PUBLIC_KEY: [u8; 32] = [
    0x44, 0xd8, 0xe0, 0xdc, 0xd9, 0xfc, 0x1f, 0xaf, 0xda, 0x06, 0x0d, 0x6e, 0x9f, 0x01, 0xa3, 0x91,
    0x44, 0xdc, 0xad, 0xd4, 0xf1, 0x11, 0x13, 0x5e, 0x7d, 0x56, 0xaa, 0x53, 0xc7, 0x05, 0xbb, 0x4b,
];

/// Filename suffix of the detached signature published alongside each asset.
pub const SIGNATURE_SUFFIX: &str = ".sig";

/// Verify a detached ed25519 signature over `data` against the embedded Colony
/// release key. Returns Ok only if the signature is valid.
pub fn verify_release_signature(data: &[u8], signature_bytes: &[u8]) -> Result<()> {
    verify_with_key(&RELEASE_PUBLIC_KEY, data, signature_bytes)
}

fn verify_with_key(pubkey: &[u8; 32], data: &[u8], signature_bytes: &[u8]) -> Result<()> {
    let sig = parse_signature(signature_bytes)?;
    let vk = VerifyingKey::from_bytes(pubkey)
        .map_err(|e| anyhow::anyhow!("invalid release public key: {e}"))?;
    vk.verify(data, &sig)
        .map_err(|_| anyhow::anyhow!("signature verification failed (untrusted or corrupt update)"))
}

/// Parse a signature that is either raw 64 bytes or base64-encoded text.
fn parse_signature(bytes: &[u8]) -> Result<Signature> {
    if bytes.len() == 64 {
        let arr: [u8; 64] = bytes.try_into().expect("checked len == 64");
        return Ok(Signature::from_bytes(&arr));
    }
    let text: String = std::str::from_utf8(bytes)
        .map_err(|_| anyhow::anyhow!("signature is neither 64 raw bytes nor UTF-8 base64"))?
        .split_whitespace()
        .collect();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(text.as_bytes())
        .map_err(|e| anyhow::anyhow!("invalid base64 signature: {e}"))?;
    let arr: [u8; 64] = decoded
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("signature must be 64 bytes, got {}", decoded.len()))?;
    Ok(Signature::from_bytes(&arr))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Independent throwaway test key + vector (NOT the release key), generated
    // with `openssl genpkey -algorithm ed25519` + `openssl pkeyutl -sign -rawin`.
    const TEST_PUBKEY: [u8; 32] = [
        0x8a, 0x91, 0x39, 0x21, 0xcf, 0x5f, 0x62, 0x2f, 0x03, 0x5d, 0x2e, 0x89, 0x1e, 0xae, 0xb3,
        0x53, 0x33, 0xde, 0x28, 0xd3, 0x03, 0xdf, 0xba, 0x3c, 0xdd, 0x86, 0x42, 0x28, 0x61, 0x86,
        0x24, 0x3c,
    ];
    const TEST_SIG: [u8; 64] = [
        0x3c, 0x4a, 0xb2, 0x48, 0xca, 0x68, 0x96, 0x9a, 0x0b, 0xe3, 0x04, 0x69, 0xd5, 0xa2, 0xce,
        0x9a, 0xf2, 0x91, 0x2f, 0x01, 0x1e, 0xca, 0x1e, 0xf3, 0xbe, 0x78, 0xc3, 0x56, 0xa7, 0xb6,
        0x15, 0xfd, 0x83, 0xe2, 0x6b, 0x50, 0xca, 0x44, 0x5b, 0x80, 0x33, 0xef, 0x56, 0x1d, 0x3c,
        0xd0, 0xf6, 0xca, 0x66, 0xf5, 0xd8, 0x41, 0xe1, 0xc6, 0xfb, 0x62, 0xa7, 0xa1, 0x54, 0xdc,
        0x7f, 0x1e, 0x33, 0x0b,
    ];
    const TEST_MSG: &[u8] = b"the quick brown fox jumps over the lazy dog";

    #[test]
    fn valid_signature_accepted() {
        assert!(verify_with_key(&TEST_PUBKEY, TEST_MSG, &TEST_SIG).is_ok());
    }

    #[test]
    fn base64_signature_accepted() {
        let b64 = base64::engine::general_purpose::STANDARD.encode(TEST_SIG);
        assert!(verify_with_key(&TEST_PUBKEY, TEST_MSG, b64.as_bytes()).is_ok());
    }

    #[test]
    fn tampered_message_rejected() {
        let mut bad = TEST_MSG.to_vec();
        bad[0] ^= 0xff;
        assert!(verify_with_key(&TEST_PUBKEY, &bad, &TEST_SIG).is_err());
    }

    #[test]
    fn wrong_key_rejected() {
        // The embedded release key must not validate the unrelated test vector.
        assert!(verify_with_key(&RELEASE_PUBLIC_KEY, TEST_MSG, &TEST_SIG).is_err());
    }

    #[test]
    fn malformed_signature_rejected() {
        assert!(verify_with_key(&TEST_PUBKEY, TEST_MSG, &[0u8; 10]).is_err());
        assert!(verify_with_key(&TEST_PUBKEY, TEST_MSG, b"not-base64-!!!").is_err());
    }
}
