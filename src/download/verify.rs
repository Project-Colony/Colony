//! Integrity and signature rules: SHA-256 digests, and what a signed `.meta`
//! sidecar must bind before an app or a launcher update is accepted.

use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::github::APP_VERSION;

/// Lowercase hex SHA-256 of `bytes`, the form release metadata and colony.json
/// carry. sha2 0.11 returns a `hybrid_array::Array`, which has no `LowerHex`,
/// so the digest is spelled out byte by byte.
fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// Verify a SHA256 digest over bytes already in memory.
///
/// Takes bytes rather than a path so the caller checks exactly what it is about
/// to install: re-opening the staged file to hash it means the digest describes
/// one read and the install uses another.
pub(super) fn verify_sha256_bytes(bytes: &[u8], expected_hex: &str) -> Result<()> {
    let computed = sha256_hex(bytes);
    if computed != expected_hex.to_lowercase() {
        anyhow::bail!(
            "SHA256 mismatch: expected {}, got {}",
            expected_hex.to_lowercase(),
            computed
        );
    }
    Ok(())
}

/// Verify a signed metadata sidecar and enforce what it binds.
///
/// Checks, in order: the sidecar is signed by the release key; it describes the
/// asset we asked for; its digest matches the bytes in hand; it names the tag the
/// update check selected (`expected_tag`, unknown at apply time hence optional);
/// and its version is strictly newer than the running build. That last check is
/// the anti-rollback - without it, any older org-signed binary is a valid
/// "update".
pub(super) fn verify_launcher_metadata(
    meta_bytes: &[u8],
    meta_sig: &[u8],
    binary_bytes: &[u8],
    expected_asset: &str,
    expected_tag: Option<&str>,
) -> Result<()> {
    crate::signing::verify_release_signature(meta_bytes, meta_sig)
        .map_err(|e| anyhow::anyhow!("update metadata signature invalid: {e}"))?;
    let meta = crate::signing::ReleaseMetadata::parse(meta_bytes)?;
    check_metadata_bindings(&meta, binary_bytes, expected_asset, expected_tag)?;
    ensure_strictly_newer(&meta, APP_VERSION)
}

/// The binding rules shared by the launcher and app install paths, split out so
/// they can be tested without the release private key. Assumes the metadata
/// signature has already been verified.
///
/// Deliberately does NOT compare versions: that rule differs between the two
/// callers (see [`ensure_strictly_newer`] and [`ensure_not_a_downgrade`]).
pub(super) fn check_metadata_bindings(
    meta: &crate::signing::ReleaseMetadata,
    binary_bytes: &[u8],
    expected_asset: &str,
    expected_tag: Option<&str>,
) -> Result<()> {
    anyhow::ensure!(
        meta.asset == expected_asset,
        "update metadata is for a different asset (signed '{}', expected '{expected_asset}')",
        meta.asset
    );

    let digest = sha256_hex(binary_bytes);
    anyhow::ensure!(
        digest == meta.sha256,
        "update metadata digest mismatch (signed {}, downloaded {digest})",
        meta.sha256
    );

    if let Some(tag) = expected_tag {
        anyhow::ensure!(
            meta.version == tag,
            "update metadata is for a different release (signed '{}', expected '{tag}')",
            meta.version
        );
    }

    Ok(())
}

/// The launcher's extra rule: a self-update must be STRICTLY newer than the
/// build applying it. Separate from [`check_metadata_bindings`] because apps do
/// not share it - an app pinned to a fixed tag legitimately reinstalls the same
/// version, so "strictly newer" would make reinstalling impossible.
fn ensure_strictly_newer(meta: &crate::signing::ReleaseMetadata, running: &str) -> Result<()> {
    let new = crate::github::parse_version_tag(&meta.version).ok_or_else(|| {
        anyhow::anyhow!(
            "unrecognized version in update metadata: '{}'",
            meta.version
        )
    })?;
    let current = crate::github::parse_version_tag(running)
        .ok_or_else(|| anyhow::anyhow!("unparseable running version"))?;
    anyhow::ensure!(
        new > current,
        "refusing a downgrade: signed update is {new} but the running build is {current}"
    );
    Ok(())
}

/// The app equivalent: never go BACKWARDS from what is installed, but allow the
/// same version (a reinstall, or an app pinned to a fixed tag).
///
/// Non-semver tags are common in the ecosystem and are not orderable, so they
/// are accepted here rather than refused - the asset, digest and tag bindings
/// already did the work that matters, and refusing them would lock those apps
/// out of updates entirely.
pub(super) fn ensure_not_a_downgrade(
    meta: &crate::signing::ReleaseMetadata,
    installed: Option<&str>,
) -> Result<()> {
    let Some(installed) = installed else {
        return Ok(());
    };
    let (Some(new), Some(current)) = (
        crate::github::parse_version_tag(&meta.version),
        crate::github::parse_version_tag(installed),
    ) else {
        return Ok(());
    };
    anyhow::ensure!(
        new >= current,
        "refusing a downgrade: signed release is {new} but {current} is installed"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta_for(bytes: &[u8], version: &str, asset: &str) -> crate::signing::ReleaseMetadata {
        crate::signing::ReleaseMetadata {
            version: version.into(),
            asset: asset.into(),
            sha256: sha256_hex(bytes),
        }
    }

    #[test]
    fn signed_metadata_accepts_a_genuine_newer_release() {
        let bytes = b"new colony build";
        let meta = meta_for(bytes, "v1.2.0", "colony-linux");
        assert!(
            check_metadata_bindings(&meta, bytes, "colony-linux", Some("v1.2.0")).is_ok()
                && ensure_strictly_newer(&meta, "1.1.0").is_ok()
        );
    }

    /// The point of the sidecar: an older but genuinely org-signed binary must
    /// not be installable as an "update".
    #[test]
    fn signed_metadata_refuses_a_downgrade_or_replay() {
        let bytes = b"old colony build";
        for older in ["v1.0.0", "v0.9.0", "v1.1.0"] {
            let meta = meta_for(bytes, older, "colony-linux");
            // The BINDINGS pass - the bytes really are that signed artefact.
            // What refuses the replay is the version rule that sits beside
            // them, which is why it lives in its own function per caller.
            assert!(check_metadata_bindings(&meta, bytes, "colony-linux", Some(older)).is_ok());
            let err = ensure_strictly_newer(&meta, "1.1.0")
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("downgrade"),
                "{older} vs running 1.1.0 should be refused as a downgrade, got: {err}"
            );
        }
    }

    #[test]
    fn signed_metadata_refuses_a_substituted_artefact() {
        let served = b"the macos binary";
        // Genuine, correctly signed metadata - but for a DIFFERENT asset.
        let meta = meta_for(served, "v1.2.0", "colony-macos");
        assert!(
            check_metadata_bindings(&meta, served, "colony-linux", Some("v1.2.0")).is_err(),
            "metadata naming another asset must not validate this download"
        );
    }

    #[test]
    fn signed_metadata_refuses_mismatched_digest_or_tag() {
        let meta = meta_for(b"expected bytes", "v1.2.0", "colony-linux");
        assert!(
            check_metadata_bindings(&meta, b"tampered bytes", "colony-linux", None).is_err(),
            "digest mismatch must fail"
        );
        assert!(
            check_metadata_bindings(&meta, b"expected bytes", "colony-linux", Some("v1.3.0"))
                .is_err(),
            "metadata for another tag than the one resolved must fail"
        );
    }

    /// Apps do not share the launcher's strictly-newer rule - an app pinned to
    /// a fixed tag reinstalls the same version - but they must still refuse a
    /// genuinely-signed OLDER build replayed under a new tag.
    #[test]
    fn an_app_may_reinstall_the_same_version_but_never_an_older_one() {
        let meta = meta_for(b"bytes", "v1.2.0", "grape-linux");

        assert!(
            ensure_not_a_downgrade(&meta, Some("v1.2.0")).is_ok(),
            "reinstalling the pinned version must stay possible"
        );
        assert!(ensure_not_a_downgrade(&meta, Some("v1.1.0")).is_ok());
        assert!(ensure_not_a_downgrade(&meta, None).is_ok(), "first install");
        assert!(
            ensure_not_a_downgrade(&meta, Some("v2.0.0")).is_err(),
            "a signed but older release must not be installable over a newer one"
        );

        // The launcher rule is the stricter one, and stays that way.
        assert!(ensure_strictly_newer(&meta, "1.1.0").is_ok());
        assert!(
            ensure_strictly_newer(&meta, "1.2.0").is_err(),
            "the launcher must never reapply its own version"
        );

        // Non-semver tags are common in the ecosystem and are not orderable:
        // accepted, because the asset/digest/tag bindings already did the work
        // and refusing would lock those apps out of updates entirely.
        let nightly = meta_for(b"bytes", "nightly", "grape-linux");
        assert!(ensure_not_a_downgrade(&nightly, Some("nightly")).is_ok());
    }

    #[test]
    fn sha256_hex_matches_the_fips_180_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_verification_correct() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("colony_test_sha256");
        let _ = std::fs::create_dir_all(&dir);
        let file_path = dir.join("test.bin");
        let content = b"hello world";
        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(content).unwrap();
        f.flush().unwrap();

        // SHA256 of "hello world"
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert!(verify_sha256_bytes(&std::fs::read(&file_path).unwrap(), expected).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sha256_verification_mismatch() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("colony_test_sha256_bad");
        let _ = std::fs::create_dir_all(&dir);
        let file_path = dir.join("test.bin");
        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(b"hello world").unwrap();
        f.flush().unwrap();

        assert!(
            verify_sha256_bytes(&std::fs::read(&file_path).unwrap(), "0000000000000000").is_err()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
