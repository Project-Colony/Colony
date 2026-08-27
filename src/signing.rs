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
use ed25519_dalek::{Signature, VerifyingKey};

/// Colony release signing public keys (ed25519, raw 32 bytes each).
///
/// A LIST, not a single key, because rotation is otherwise not expressible.
/// With one embedded key, `sign-release.sh` emits exactly one `<asset>.sig`,
/// which is either old-key (refused by every updated client) or new-key
/// (refused by every client in the field) - and verification is fail-closed, so
/// the refusal is permanent. The documented rotation procedure could not be
/// carried out in either direction: a planned rotation stranded every existing
/// install, and an emergency one after a leak left the defender with nothing
/// anyone could verify while the attacker held a key every client trusts.
///
/// Accepting any listed key makes rotation a real three-release sequence:
///
/// 1. Ship N embedding `[old, new]`, still SIGNED with `old` - every client in
///    the field accepts it, and afterwards trusts both.
/// 2. Sign N+1 with `new` - clients on N accept it; clients still on N-1 do not
///    update, which is the cost of the overlap window.
/// 3. Drop `old` from this list in N+2 to complete the revocation.
///
/// Order is irrelevant to verification; keep the newest first for readability.
/// See `docs/release-signing.md`.
const RELEASE_PUBLIC_KEYS: &[[u8; 32]] = &[[
    0x44, 0xd8, 0xe0, 0xdc, 0xd9, 0xfc, 0x1f, 0xaf, 0xda, 0x06, 0x0d, 0x6e, 0x9f, 0x01, 0xa3, 0x91,
    0x44, 0xdc, 0xad, 0xd4, 0xf1, 0x11, 0x13, 0x5e, 0x7d, 0x56, 0xaa, 0x53, 0xc7, 0x05, 0xbb, 0x4b,
]];

/// Filename suffix of the detached signature published alongside each asset.
pub const SIGNATURE_SUFFIX: &str = ".sig";

/// Filename suffix of the signed metadata sidecar published alongside each asset
/// (itself signed as `<asset>.meta.sig`).
pub const METADATA_SUFFIX: &str = ".meta";

/// Contents of a signed `<asset>.meta` sidecar.
///
/// A raw-bytes signature proves only that bytes came from the release key, not
/// WHICH artefact or version they are, so an attacker able to control what the
/// release host serves could replay an older, genuinely signed build. This
/// sidecar binds the bytes to a version and a filename, and is signed with the
/// same key; `scripts/sign-release.sh` emits it for every asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseMetadata {
    /// Release tag the asset belongs to, e.g. `v1.2.3`.
    pub version: String,
    /// Basename of the asset the digest covers, e.g. `colony-linux`.
    pub asset: String,
    /// Lowercase hex SHA-256 of the asset bytes.
    pub sha256: String,
}

impl ReleaseMetadata {
    /// Parse the `key=value` sidecar format. Unknown keys are ignored so future
    /// fields do not break older launchers; the three known ones are required.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| anyhow::anyhow!("release metadata is not valid UTF-8"))?;
        let mut version = None;
        let mut asset = None;
        let mut sha256 = None;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                anyhow::bail!("malformed release metadata line: {line:?}");
            };
            match key.trim() {
                "version" => version = Some(value.trim().to_string()),
                "asset" => asset = Some(value.trim().to_string()),
                "sha256" => sha256 = Some(value.trim().to_ascii_lowercase()),
                _ => {}
            }
        }
        Ok(Self {
            version: version.ok_or_else(|| anyhow::anyhow!("release metadata has no version"))?,
            asset: asset.ok_or_else(|| anyhow::anyhow!("release metadata has no asset"))?,
            sha256: sha256.ok_or_else(|| anyhow::anyhow!("release metadata has no sha256"))?,
        })
    }
}

/// Verify a detached ed25519 signature over `data` against the embedded Colony
/// release keys. Returns Ok if ANY trusted key validates it.
///
/// Every key is tried even after one succeeds is NOT the case here - the loop
/// short-circuits, which is fine: the key list is public and its length leaks
/// nothing. What matters is that the signature itself is checked with
/// `verify_strict`, which is constant-time in the secret-bearing parts.
pub fn verify_release_signature(data: &[u8], signature_bytes: &[u8]) -> Result<()> {
    // Parse once: a malformed signature is a malformed signature whatever key
    // we would have tried it against, and the error should say so.
    let sig = parse_signature(signature_bytes)?;
    for key in RELEASE_PUBLIC_KEYS {
        if verify_parsed(key, data, &sig).is_ok() {
            return Ok(());
        }
    }
    anyhow::bail!("signature verification failed (untrusted or corrupt update)")
}

fn verify_parsed(pubkey: &[u8; 32], data: &[u8], sig: &Signature) -> Result<()> {
    let vk = VerifyingKey::from_bytes(pubkey)
        .map_err(|e| anyhow::anyhow!("invalid release public key: {e}"))?;
    // verify_strict: rejects malleable/non-canonical signatures and small-
    // order key components - the recommended verifier for update/security
    // contexts (plain verify() accepts signatures strict verification would
    // refuse).
    vk.verify_strict(data, sig)
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

    /// Verify against ONE named key. Production always goes through
    /// `verify_release_signature`, which parses once and loops over the trusted
    /// list; tests need to name a key so they can pin down which one accepted.
    fn verify_with_key(pubkey: &[u8; 32], data: &[u8], signature_bytes: &[u8]) -> Result<()> {
        let sig = parse_signature(signature_bytes)?;
        verify_parsed(pubkey, data, &sig)
    }

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
        // No embedded release key may validate the unrelated test vector.
        assert!(verify_release_signature(TEST_MSG, &TEST_SIG).is_err());
        for key in RELEASE_PUBLIC_KEYS {
            assert!(verify_with_key(key, TEST_MSG, &TEST_SIG).is_err());
        }
    }

    /// Rotation depends on a signature from EITHER listed key being accepted -
    /// that is the whole overlap window. Without it, the documented procedure
    /// strands one half of the install base whichever key you sign with.
    #[test]
    fn any_trusted_key_verifies_and_an_untrusted_one_does_not() {
        // Stand in for a rotation list: the real release key plus the throwaway
        // test key, in the shape RELEASE_PUBLIC_KEYS takes during an overlap.
        let rotating: &[[u8; 32]] = &[RELEASE_PUBLIC_KEYS[0], TEST_PUBKEY];

        let verified = rotating
            .iter()
            .any(|k| verify_with_key(k, TEST_MSG, &TEST_SIG).is_ok());
        assert!(
            verified,
            "a signature from the incoming key must be accepted during the overlap"
        );

        // A key that is NOT on the list still fails: rotation widens trust to a
        // named set, it does not weaken verification.
        let mut stranger = TEST_PUBKEY;
        stranger[0] ^= 0xff;
        assert!(
            !std::iter::once(stranger).any(|k| verify_with_key(&k, TEST_MSG, &TEST_SIG).is_ok()),
            "an unlisted key must never validate"
        );

        // And the shipped list is a single key today, so nothing is widened yet.
        assert_eq!(
            RELEASE_PUBLIC_KEYS.len(),
            1,
            "no rotation is in progress; bump this when one starts"
        );
    }

    #[test]
    fn malformed_signature_rejected() {
        assert!(verify_with_key(&TEST_PUBKEY, TEST_MSG, &[0u8; 10]).is_err());
        assert!(verify_with_key(&TEST_PUBKEY, TEST_MSG, b"not-base64-!!!").is_err());
    }

    #[test]
    fn metadata_parses_sign_release_output() {
        // Byte-for-byte what scripts/sign-release.sh writes.
        let raw = b"version=v0.9.1\nasset=colony-linux\nsha256=B1A5AF3D\n";
        let meta = ReleaseMetadata::parse(raw).unwrap();
        assert_eq!(meta.version, "v0.9.1");
        assert_eq!(meta.asset, "colony-linux");
        assert_eq!(meta.sha256, "b1a5af3d", "digest is normalized to lowercase");
    }

    #[test]
    fn metadata_ignores_unknown_keys() {
        let raw = b"version=v1.0.0\nasset=a\nsha256=ff\nfuture=whatever\n";
        assert!(ReleaseMetadata::parse(raw).is_ok());
    }

    #[test]
    fn metadata_requires_every_known_field() {
        assert!(ReleaseMetadata::parse(b"asset=a\nsha256=ff\n").is_err());
        assert!(ReleaseMetadata::parse(b"version=v1\nsha256=ff\n").is_err());
        assert!(ReleaseMetadata::parse(b"version=v1\nasset=a\n").is_err());
        assert!(ReleaseMetadata::parse(b"garbage\n").is_err());
        assert!(ReleaseMetadata::parse(&[0xff, 0xfe]).is_err());
    }
}
