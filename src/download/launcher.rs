//! Launcher self-update: whether this install may update itself, staging a
//! signed update, and replacing the running binary with rollback.

use anyhow::Result;
use std::path::PathBuf;

use super::files::write_new_file;
use super::http::{build_url, download_client, download_with_resume, fetch_bytes};
use super::verify::verify_launcher_metadata;
use crate::github::{LAUNCHER_OWNER, LAUNCHER_REPO};
use crate::persistence::colony_cache_dir;

/// True when the running executable lives in a system location owned by a
/// package manager (AUR/pacman installs land in /usr/bin, manual system
/// installs in /usr/local or /opt). Self-update can NEVER apply there - the
/// backup rename of the running exe fails with EACCES after downloading the
/// whole asset - so the UI offers package-manager guidance instead of a
/// download button that is guaranteed to die.
pub fn launcher_is_system_managed() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(dir) = exe.parent() else {
        return false;
    };
    // Behavioural, not a list of paths: probe whether we can actually create a
    // file next to the executable. That answers /usr, /opt, Program Files,
    // a read-only mount and a macOS /Applications install with one test and no
    // per-platform table to keep in sync. The path check was #[cfg(unix)] only,
    // so a Windows install under Program Files downloaded and verified the
    // entire asset before dying on the rename - exactly the failure the guard
    // was written to prevent, just never extended to the other platforms.
    let probe = dir.join(format!(".colony-write-probe-{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            false
        }
        Err(_) => true,
    }
}

/// Download a release asset from the Colony launcher repo.
/// Returns the path to the downloaded file in a staging directory.
pub async fn download_launcher_asset(
    token: Option<String>,
    tag: String,
    filename: String,
    progress_tx: Option<futures::channel::mpsc::UnboundedSender<(u64, Option<u64>)>>,
) -> Result<PathBuf> {
    // Transient by definition: staging lives in the cache.
    let temp_dir = colony_cache_dir()?.join("update-staging");
    std::fs::create_dir_all(&temp_dir)?;
    let dest_path = temp_dir.join(&filename);
    // Stream to `<asset>.part` and only rename onto the apply path once the
    // whole file is here. Writing straight to the final name meant a download
    // the user never applied - or a cancelled one - left a partial file at
    // exactly the path apply_launcher_update consumes. Apply re-verifies, so it
    // was refused fail-closed rather than being a hole, but the file sat there.
    let part_path = temp_dir.join(format!("{filename}.part"));

    let url = build_url(
        "https://github.com",
        &[
            LAUNCHER_OWNER,
            LAUNCHER_REPO,
            "releases",
            "download",
            &tag,
            &filename,
        ],
    )?;

    let client = download_client()?;
    // Validates the length, rejects an empty/truncated body, and resumes a
    // previous partial transfer when one provably describes the same asset.
    download_with_resume(&client, &url, token.as_deref(), &part_path, progress_tx).await?;
    let _ = std::fs::remove_file(&dest_path);
    std::fs::rename(&part_path, &dest_path)?;

    // Fail-closed signature check: fetch the detached signature and verify the
    // downloaded binary against the embedded release key BEFORE it can be
    // applied. A missing, malformed, or invalid signature aborts the update.
    let sig_url = format!("{url}{}", crate::signing::SIGNATURE_SUFFIX);
    let signature = match fetch_bytes(&client, &sig_url, token.as_deref()).await {
        Ok(bytes) => bytes,
        Err(e) => {
            let _ = std::fs::remove_file(&dest_path);
            anyhow::bail!(
                "Refusing to self-update: could not fetch the update signature ({sig_url}): {e}"
            );
        }
    };
    let binary_bytes = std::fs::read(&dest_path)?;
    if let Err(e) = crate::signing::verify_release_signature(&binary_bytes, &signature) {
        let _ = std::fs::remove_file(&dest_path);
        anyhow::bail!("Refusing to self-update: {e}");
    }
    // The signature above proves the bytes came from the release key, but not
    // WHICH artefact or version they are: an attacker controlling what the
    // release host serves could replay an older, genuinely signed build. The
    // signed metadata sidecar closes that by binding these bytes to a version
    // and a filename. Fail-closed, exactly like the signature itself.
    let meta_url = format!("{url}{}", crate::signing::METADATA_SUFFIX);
    let meta_bytes = match fetch_bytes(&client, &meta_url, token.as_deref()).await {
        Ok(bytes) => bytes,
        Err(e) => {
            let _ = std::fs::remove_file(&dest_path);
            anyhow::bail!(
                "Refusing to self-update: could not fetch the update metadata ({meta_url}): {e}"
            );
        }
    };
    let meta_sig = match fetch_bytes(
        &client,
        &format!("{meta_url}{}", crate::signing::SIGNATURE_SUFFIX),
        token.as_deref(),
    )
    .await
    {
        Ok(bytes) => bytes,
        Err(e) => {
            let _ = std::fs::remove_file(&dest_path);
            anyhow::bail!("Refusing to self-update: could not fetch the metadata signature: {e}");
        }
    };
    if let Err(e) =
        verify_launcher_metadata(&meta_bytes, &meta_sig, &binary_bytes, &filename, Some(&tag))
    {
        let _ = std::fs::remove_file(&dest_path);
        anyhow::bail!("Refusing to self-update: {e}");
    }

    // Persist the signature next to the staged binary so apply_launcher_update
    // can re-verify at install time — closing any window in which the staged
    // file could be swapped between download and apply. The metadata sidecar is
    // staged for the same reason.
    let sig_path = staged_signature_path(&dest_path);
    if let Err(e) = write_new_file(&sig_path, &signature) {
        let _ = std::fs::remove_file(&dest_path);
        anyhow::bail!("Could not stage update signature: {e}");
    }
    let (meta_path, meta_sig_path) = staged_metadata_paths(&dest_path);
    if let Err(e) = write_new_file(&meta_path, &meta_bytes)
        .and_then(|()| write_new_file(&meta_sig_path, &meta_sig))
    {
        let _ = std::fs::remove_file(&dest_path);
        anyhow::bail!("Could not stage update metadata: {e}");
    }
    tracing::info!("Launcher update signature and metadata verified for {filename} ({tag})");

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest_path, perms)?;
    }

    Ok(dest_path)
}

/// Path of the detached signature staged next to a downloaded binary.
fn staged_signature_path(binary: &std::path::Path) -> PathBuf {
    PathBuf::from(format!(
        "{}{}",
        binary.display(),
        crate::signing::SIGNATURE_SUFFIX
    ))
}

/// Paths of the metadata sidecar and its signature, staged next to a binary.
fn staged_metadata_paths(binary: &std::path::Path) -> (PathBuf, PathBuf) {
    let meta = format!("{}{}", binary.display(), crate::signing::METADATA_SUFFIX);
    let sig = format!("{meta}{}", crate::signing::SIGNATURE_SUFFIX);
    (PathBuf::from(meta), PathBuf::from(sig))
}

/// Replace the running Colony binary with the downloaded update.
/// Returns the exe path for relaunch on success. Restores backup on failure.
pub fn apply_launcher_update(new_binary: &std::path::Path) -> Result<PathBuf> {
    let current_exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Cannot determine current exe path: {e}"))?;

    // Re-verify the staged binary against its staged signature at the moment of
    // installation, closing any local window in which the staged file (in the
    // update-staging dir) could have been swapped after the download-time check.
    let sig_path = staged_signature_path(new_binary);
    let signature = std::fs::read(&sig_path).map_err(|e| {
        anyhow::anyhow!(
            "Missing staged update signature ({}): {e}",
            sig_path.display()
        )
    })?;
    let staged_bytes = std::fs::read(new_binary)
        .map_err(|e| anyhow::anyhow!("Cannot read staged update binary: {e}"))?;
    crate::signing::verify_release_signature(&staged_bytes, &signature)
        .map_err(|e| anyhow::anyhow!("Refusing to apply update: {e}"))?;

    // Re-check the sidecar at install time too, for the same reason the signature
    // is re-checked: the staging directory is writable, so the file could have
    // been swapped after the download-time check. The requested tag is not in
    // scope here, so this enforces the asset name, the digest and "strictly newer
    // than the running build" - enough to stop a rollback, though a swap to
    // another org-signed release that is ALSO newer than the running build would
    // still pass. Closing that needs the tag threaded through the staged state.
    let (meta_path, meta_sig_path) = staged_metadata_paths(new_binary);
    let staged_meta = std::fs::read(&meta_path).map_err(|e| {
        anyhow::anyhow!(
            "Missing staged update metadata ({}): {e}",
            meta_path.display()
        )
    })?;
    let staged_meta_sig = std::fs::read(&meta_sig_path).map_err(|e| {
        anyhow::anyhow!(
            "Missing staged update metadata signature ({}): {e}",
            meta_sig_path.display()
        )
    })?;
    let expected_asset = new_binary
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("staged update has no usable file name"))?;
    verify_launcher_metadata(
        &staged_meta,
        &staged_meta_sig,
        &staged_bytes,
        expected_asset,
        None,
    )
    .map_err(|e| anyhow::anyhow!("Refusing to apply update: {e}"))?;

    // Refuse to touch the running binary if the staged update is empty/missing.
    anyhow::ensure!(
        !staged_bytes.is_empty(),
        "Staged update binary is empty; refusing to apply"
    );

    let backup_path = current_exe.with_extension("old");
    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    // Stage the new binary next to the current exe (same filesystem) so the
    // final swap is an atomic rename rather than a non-atomic, interruptible
    // copy directly over the running binary.
    let staged_next = current_exe.with_extension("new");
    if staged_next.exists() {
        let _ = std::fs::remove_file(&staged_next);
    }
    // Write the byte buffer that was just VERIFIED - copying the file again
    // would re-read from disk and install bytes the signature check never saw
    // (a swap between read and copy would slip through). Through
    // `write_new_file`, because `<exe>.new` is a predictable name in a
    // user-writable directory: `fs::write` follows a symlink planted there, so
    // the verified bytes would land at the attacker's path and the rename below
    // would then move the SYMLINK over the running binary - turning Colony's
    // own executable into a link to a file rewritable afterwards, cleanly past
    // every signature check.
    write_new_file(&staged_next, &staged_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to stage new binary: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&staged_next)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&staged_next, perms)?;
    }

    // Back up the running binary (renaming a running exe works on all
    // platforms), then atomically move the staged copy into its place.
    std::fs::rename(&current_exe, &backup_path)
        .map_err(|e| anyhow::anyhow!("Failed to backup current binary: {e}"))?;

    match std::fs::rename(&staged_next, &current_exe) {
        Ok(()) => {
            let _ = std::fs::remove_file(new_binary);
            let _ = std::fs::remove_file(&sig_path);
            // The sidecars are staged next to the binary, so they must be swept
            // too or the staging directory never becomes empty.
            let _ = std::fs::remove_file(&meta_path);
            let _ = std::fs::remove_file(&meta_sig_path);
            let _ = std::fs::remove_dir(new_binary.parent().unwrap_or(new_binary));
            Ok(current_exe)
        }
        Err(e) => {
            tracing::error!("Failed to install new binary, restoring backup: {e}");
            let _ = std::fs::remove_file(&staged_next);
            if let Err(re) = std::fs::rename(&backup_path, &current_exe) {
                tracing::error!(
                    "CRITICAL: could not restore backup {} -> {}: {re}",
                    backup_path.display(),
                    current_exe.display()
                );
            }
            Err(anyhow::anyhow!("Failed to install new binary: {e}"))
        }
    }
}
