//! Asset download, archive extraction, and launcher self-update.
//!
//! Downloads stream to a temporary file with length/integrity checks and are
//! atomically promoted into place; launcher self-updates are verified against a
//! signed detached signature (see [`crate::signing`]) before being applied.

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Duration;

use crate::github::{APP_VERSION, CONNECT_TIMEOUT, GITHUB_ACCOUNT, LAUNCHER_OWNER, LAUNCHER_REPO};
use crate::persistence::{colony_apps_dir, colony_data_dir};

/// Build the HTTP client used for large asset downloads (longer read timeout
/// than the API client).
fn download_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(format!("Colony-Launcher/{APP_VERSION}"))
        .timeout(Duration::from_secs(300))
        .connect_timeout(CONNECT_TIMEOUT)
        .build()?)
}

/// Stream an HTTP GET to `dest_path`, sending throttled progress (0.0..1.0)
/// over `progress_tx`. Verifies the received length against Content-Length when
/// present and rejects empty/truncated downloads. Removes `dest_path` on any
/// failure. Shared by app-asset install and launcher self-update.
async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
    dest_path: &std::path::Path,
    progress_tx: Option<futures::channel::mpsc::UnboundedSender<(u64, Option<u64>)>>,
) -> Result<()> {
    let mut request = client.get(url);
    if let Some(t) = token {
        request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {t}"));
    }

    let resp = request.send().await.map_err(|e| {
        if e.is_timeout() {
            anyhow::anyhow!("Download timed out for {url}")
        } else {
            anyhow::anyhow!("Download failed for {url}: {e}")
        }
    })?;

    if !resp.status().is_success() {
        anyhow::bail!("Download failed: HTTP {} for {url}", resp.status());
    }

    let total = resp.content_length();
    let mut downloaded: u64 = 0;

    use futures::StreamExt;
    use std::io::Write;
    let mut file = std::fs::File::create(dest_path)?;
    let mut stream = resp.bytes_stream();
    let mut last_pct: u32 = 0;

    let stream_result: Result<()> = async {
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;

            if let Some(ref tx) = progress_tx {
                // Throttle: send on whole-percent changes when the total is
                // known, else every 256 KiB - not per network chunk.
                let should_send = match total {
                    Some(total) if total > 0 => {
                        let pct = ((downloaded as f64 / total as f64) * 100.0) as u32;
                        if pct != last_pct {
                            last_pct = pct;
                            true
                        } else {
                            false
                        }
                    }
                    _ => {
                        let bucket = (downloaded / (256 * 1024)) as u32;
                        if bucket != last_pct {
                            last_pct = bucket;
                            true
                        } else {
                            false
                        }
                    }
                };
                if should_send {
                    let _ = tx.unbounded_send((downloaded, total));
                }
            }
        }
        file.flush()?;
        Ok(())
    }
    .await;

    if let Err(e) = stream_result {
        let _ = std::fs::remove_file(dest_path);
        return Err(e);
    }

    // Guard against a silently-truncated or empty transfer.
    if let Some(total) = total {
        if downloaded != total {
            let _ = std::fs::remove_file(dest_path);
            anyhow::bail!("Incomplete download: got {downloaded} of {total} bytes for {url}");
        }
    }
    if downloaded == 0 {
        let _ = std::fs::remove_file(dest_path);
        anyhow::bail!("Empty download (0 bytes) for {url}");
    }

    Ok(())
}

/// Fetch a small OPTIONAL resource: `Ok(None)` on HTTP 404 (the resource
/// genuinely is not published), `Err` on any other failure - so a transient
/// network error can never be mistaken for "not published" (an attacker able
/// to induce errors must not be able to make a signed app look unsigned).
async fn fetch_optional_bytes(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
) -> Result<Option<Vec<u8>>> {
    let mut request = client.get(url);
    if let Some(t) = token {
        request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let resp = request.send().await?;
    if resp.status().as_u16() == 404 {
        return Ok(None);
    }
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {} for {url}", resp.status());
    }
    Ok(Some(resp.bytes().await?.to_vec()))
}

/// Fetch a small resource (e.g. a detached signature) fully into memory.
async fn fetch_bytes(client: &reqwest::Client, url: &str, token: Option<&str>) -> Result<Vec<u8>> {
    let mut request = client.get(url);
    if let Some(t) = token {
        request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let resp = request.send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {} for {url}", resp.status());
    }
    Ok(resp.bytes().await?.to_vec())
}

/// Verify SHA256 checksum of a file against an expected hex digest.
fn verify_sha256(path: &std::path::Path, expected_hex: &str) -> Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let computed = format!("{:x}", hasher.finalize());
    if computed != expected_hex.to_lowercase() {
        anyhow::bail!(
            "SHA256 mismatch: expected {}, got {}",
            expected_hex.to_lowercase(),
            computed
        );
    }
    Ok(())
}

/// True when the running executable lives in a system location owned by a
/// package manager (AUR/pacman installs land in /usr/bin, manual system
/// installs in /usr/local or /opt). Self-update can NEVER apply there - the
/// backup rename of the running exe fails with EACCES after downloading the
/// whole asset - so the UI offers package-manager guidance instead of a
/// download button that is guaranteed to die.
pub fn launcher_is_system_managed() -> bool {
    #[cfg(unix)]
    {
        std::env::current_exe()
            .map(|exe| exe.starts_with("/usr") || exe.starts_with("/opt"))
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Ensure a filename is a single normal path component (no `..`, no path
/// separators, not absolute) before it is joined into a destination directory.
/// Shared by archive extraction and raw-asset download to block path traversal.
pub(crate) fn ensure_safe_component(name: &str) -> Result<()> {
    let p = std::path::Path::new(name);
    anyhow::ensure!(
        p.components().count() == 1
            && matches!(p.components().next(), Some(std::path::Component::Normal(_))),
        "Invalid file name (path traversal attempt?): {name}"
    );
    Ok(())
}

/// Extract a single file from a .zip archive.
fn extract_from_zip(
    archive_path: &std::path::Path,
    binary_name: &str,
    dest_dir: &std::path::Path,
) -> Result<PathBuf> {
    ensure_safe_component(binary_name)?;
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_name = entry.name().to_string();
        // Match by exact filename (last component), handles entries like "dir/binary"
        let matches = std::path::Path::new(&entry_name)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == binary_name)
            .unwrap_or(false);
        if matches {
            anyhow::ensure!(
                entry.is_file(),
                "Refusing to extract non-regular zip entry '{binary_name}'"
            );
            // Extract to a temp file, then atomically rename over any previous
            // install so a failed extraction never leaves a truncated binary.
            let final_dest = dest_dir.join(binary_name);
            let tmp_dest = dest_dir.join(format!("{binary_name}.new"));
            let mut out = std::fs::File::create(&tmp_dest)?;
            std::io::copy(&mut entry, &mut out)?;
            drop(out);
            std::fs::rename(&tmp_dest, &final_dest)?;
            return Ok(final_dest);
        }
    }
    anyhow::bail!("Binary '{binary_name}' not found in zip archive")
}

/// Extract a single file from a .tar.gz archive.
fn extract_from_tar_gz(
    archive_path: &std::path::Path,
    binary_name: &str,
    dest_dir: &std::path::Path,
) -> Result<PathBuf> {
    ensure_safe_component(binary_name)?;
    let file = std::fs::File::open(archive_path)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let path = entry.path()?;
        let matches = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == binary_name)
            .unwrap_or(false);
        if matches {
            // Reject symlink / hardlink / device entries: only a regular file
            // may be unpacked, so a crafted archive cannot make us follow a
            // link to an arbitrary path when we later chmod the result.
            anyhow::ensure!(
                entry.header().entry_type().is_file(),
                "Refusing to extract non-regular tar entry '{binary_name}'"
            );
            let final_dest = dest_dir.join(binary_name);
            let tmp_dest = dest_dir.join(format!("{binary_name}.new"));
            entry.unpack(&tmp_dest)?;
            std::fs::rename(&tmp_dest, &final_dest)?;
            return Ok(final_dest);
        }
    }
    anyhow::bail!("Binary '{binary_name}' not found in tar.gz archive")
}

/// Extract a binary from an archive based on its extension.
///
/// `asset_name` is the original release asset filename used only to detect the
/// archive type — `archive_path` may be a staging file (e.g. `foo.zip.part`)
/// whose own extension must not be used for detection.
fn extract_binary_from_archive(
    archive_path: &std::path::Path,
    asset_name: &str,
    binary_name: &str,
    dest_dir: &std::path::Path,
) -> Result<PathBuf> {
    if asset_name.ends_with(".zip") {
        let result = extract_from_zip(archive_path, binary_name, dest_dir);
        let _ = std::fs::remove_file(archive_path);
        result
    } else if asset_name.ends_with(".tar.gz") || asset_name.ends_with(".tgz") {
        let result = extract_from_tar_gz(archive_path, binary_name, dest_dir);
        let _ = std::fs::remove_file(archive_path);
        result
    } else {
        // Raw binary (e.g. .exe, no archive extension) — rename to binary_name in dest_dir
        let dest = dest_dir.join(binary_name);
        std::fs::rename(archive_path, &dest)?;
        Ok(dest)
    }
}

/// Everything needed to install one resolved release asset.
pub struct AssetInstall {
    pub repo_name: String,
    /// The resolved (never "latest") release tag being installed.
    pub tag: String,
    /// The resolved asset name to download.
    pub filename: String,
    /// When set, the download is an archive and this named binary is extracted.
    pub binary_name: Option<String>,
    /// When set, the download is integrity-checked against this hex digest.
    pub expected_sha256: Option<String>,
    /// True when `filename` was resolved from a filePattern: the name is then
    /// persisted next to the binary so `installed_app_path` can find the
    /// install again.
    pub record_asset: bool,
    /// True when the manifest declares `"signed": true`: a missing `.sig`
    /// then ABORTS the install instead of being treated as a legacy unsigned
    /// app - closing the "compromised repo simply omits signatures" hole.
    pub require_signature: bool,
}

/// Download a release asset to `<colony_apps_dir>/<repo_name>/<filename>`,
/// verify/extract it, atomically promote it into place, and record the
/// installed version. Returns the final path on success.
pub async fn download_release_asset(
    token: Option<String>,
    install: AssetInstall,
    progress_tx: Option<futures::channel::mpsc::UnboundedSender<(u64, Option<u64>)>>,
) -> Result<PathBuf> {
    let AssetInstall {
        repo_name,
        tag,
        filename,
        binary_name,
        expected_sha256,
        record_asset,
        require_signature,
    } = install;
    // The manifest-supplied filename becomes a local path — guard it against
    // traversal (`../`, absolute paths) before joining, mirroring the archive
    // `binary` guard.
    ensure_safe_component(&filename)?;

    let dest_dir = colony_apps_dir()?.join(&repo_name);
    std::fs::create_dir_all(&dest_dir)?;
    let dest_path = dest_dir.join(&filename);
    // Download to a temporary sibling so an interrupted or failed transfer
    // never truncates the currently-installed binary.
    let temp_path = dest_dir.join(format!("{filename}.part"));

    let url = format!(
        "https://github.com/{GITHUB_ACCOUNT}/{repo_name}/releases/download/{tag}/{filename}"
    );

    let client = download_client()?;
    download_to_file(&client, &url, token.as_deref(), &temp_path, progress_tx).await?;

    // Opportunistic app-signature verification: when the release publishes
    // `<asset>.sig`, it MUST verify against the org release key (the same
    // ed25519 key that signs the launcher, embedded in src/signing.rs). A
    // missing signature is a legacy unsigned app and stays allowed; any
    // OTHER failure fetching it aborts - a transient error must never make a
    // signed app look unsigned.
    let signature =
        match fetch_optional_bytes(&client, &format!("{url}.sig"), token.as_deref()).await {
            Ok(sig) => sig,
            Err(e) => {
                let _ = std::fs::remove_file(&temp_path);
                anyhow::bail!("Could not check for a release signature of {filename}: {e}");
            }
        };
    if require_signature && signature.is_none() {
        let _ = std::fs::remove_file(&temp_path);
        anyhow::bail!(
            "The manifest requires signed releases, but no {filename}.sig was published - refusing to install"
        );
    }

    // Integrity check, archive extraction and the atomic promotion are
    // CPU/IO-bound — run them on a blocking thread. Any failure removes the
    // temp file and leaves the previous install untouched.
    let final_path = {
        let temp_path = temp_path.clone();
        let dest_path = dest_path.clone();
        let dest_dir = dest_dir.clone();
        let expected_sha256 = expected_sha256.clone();
        let binary_name = binary_name.clone();
        let filename = filename.clone();

        tokio::task::spawn_blocking(move || -> Result<PathBuf> {
            if let Some(sig) = signature {
                let bytes = std::fs::read(&temp_path)?;
                if let Err(e) = crate::signing::verify_release_signature(&bytes, &sig) {
                    let _ = std::fs::remove_file(&temp_path);
                    anyhow::bail!(
                        "Signature verification FAILED for {filename} - refusing to install: {e}"
                    );
                }
                tracing::info!("ed25519 signature verified for {filename}");
            }
            if let Some(ref expected) = expected_sha256 {
                if let Err(e) = verify_sha256(&temp_path, expected) {
                    let _ = std::fs::remove_file(&temp_path);
                    return Err(e);
                }
                tracing::info!("SHA256 verified for {filename}");
            }

            let final_path = if let Some(ref bin) = binary_name {
                // Archive install: extract the named binary (atomically renamed
                // into place by the extractor), then drop the archive. Detect the
                // archive type from `filename`, not the `.part` staging path.
                tracing::info!("Extracting '{bin}' from archive '{filename}'");
                let extracted = extract_binary_from_archive(&temp_path, &filename, bin, &dest_dir)?;
                let _ = std::fs::remove_file(&temp_path);
                extracted
            } else {
                // Raw binary: atomically promote the verified temp file over any
                // previous install.
                std::fs::rename(&temp_path, &dest_path)?;
                dest_path
            };

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&final_path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&final_path, perms)?;
            }

            // Record WHAT is installed here, atomically with the install
            // itself - not in the UI message handler. A cancel mid-install
            // drops the awaiting future but this blocking task runs to
            // completion: writing the version from the handler meant an
            // installed binary with no version file, silently excluded from
            // every future update check (or, in the filePattern case, an
            // orphaned binary the app no longer even sees as installed).
            crate::persistence::save_installed_version(&repo_name, &tag)?;
            if record_asset {
                crate::persistence::save_installed_asset(&repo_name, &filename)?;
            }
            // Desktop integration (Linux): index the installed app in the
            // desktop environment. Best-effort - a failure here must not fail
            // an otherwise complete install.
            if let Err(e) = crate::persistence::write_desktop_entry(&repo_name, &final_path) {
                tracing::warn!("could not write desktop entry for {repo_name}: {e}");
            }

            Ok(final_path)
        })
        .await?
    };

    match final_path {
        Ok(p) => Ok(p),
        Err(e) => {
            let _ = std::fs::remove_file(&temp_path);
            Err(e)
        }
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
    let temp_dir = colony_data_dir()?.join("update-staging");
    std::fs::create_dir_all(&temp_dir)?;
    let dest_path = temp_dir.join(&filename);

    let url = format!(
        "https://github.com/{LAUNCHER_OWNER}/{LAUNCHER_REPO}/releases/download/{tag}/{filename}"
    );

    let client = download_client()?;
    // download_to_file validates the length and rejects an empty/truncated body.
    download_to_file(&client, &url, token.as_deref(), &dest_path, progress_tx).await?;

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
    // Persist the signature next to the staged binary so apply_launcher_update
    // can re-verify at install time — closing any window in which the staged
    // file could be swapped between download and apply.
    let sig_path = staged_signature_path(&dest_path);
    if let Err(e) = std::fs::write(&sig_path, &signature) {
        let _ = std::fs::remove_file(&dest_path);
        anyhow::bail!("Could not stage update signature: {e}");
    }
    tracing::info!("Launcher update signature verified for {filename}");

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
    // (a swap between read and copy would slip through).
    std::fs::write(&staged_next, &staged_bytes)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_from_zip_works() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("colony_test_zip_extract");
        let _ = std::fs::create_dir_all(&dir);

        // Create a zip archive with a binary inside
        let zip_path = dir.join("test.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        zip_writer
            .start_file("subdir/my-binary", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip_writer.write_all(b"binary-content").unwrap();
        zip_writer.finish().unwrap();

        // Extract
        let result = extract_from_zip(&zip_path, "my-binary", &dir);
        assert!(result.is_ok());
        let extracted = result.unwrap();
        assert_eq!(
            extracted.file_name().unwrap().to_str().unwrap(),
            "my-binary"
        );
        assert_eq!(
            std::fs::read_to_string(&extracted).unwrap(),
            "binary-content"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_binary_from_archive_detects_type_from_asset_name_not_staging_path() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("colony_test_part_extract");
        let _ = std::fs::create_dir_all(&dir);

        // Build a real zip, but stage it under a `.part` name (as the download
        // path does) so its own extension is NOT `.zip`.
        let staged = dir.join("my-app-linux.zip.part");
        let file = std::fs::File::create(&staged).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        zip_writer
            .start_file("my-app", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip_writer.write_all(b"real-elf-bytes").unwrap();
        zip_writer.finish().unwrap();

        // Type detection must use the asset name, so the archive is extracted
        // rather than the compressed bytes being renamed to the binary.
        let result = extract_binary_from_archive(&staged, "my-app-linux.zip", "my-app", &dir);
        assert!(result.is_ok(), "extract failed: {:?}", result.err());
        let extracted = result.unwrap();
        assert_eq!(extracted.file_name().unwrap().to_str().unwrap(), "my-app");
        assert_eq!(
            std::fs::read_to_string(&extracted).unwrap(),
            "real-elf-bytes"
        );
        // The staging archive is consumed.
        assert!(!staged.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_from_tar_gz_works() {
        let dir = std::env::temp_dir().join("colony_test_targz_extract");
        let _ = std::fs::create_dir_all(&dir);

        // Create a tar.gz archive with a binary inside
        let tar_gz_path = dir.join("test.tar.gz");
        let file = std::fs::File::create(&tar_gz_path).unwrap();
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar_builder = tar::Builder::new(gz);

        let content = b"binary-content-tar";
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar_builder
            .append_data(&mut header, "subdir/my-cli", &content[..])
            .unwrap();
        // Finish tar, then finish gzip encoder to write the gzip footer
        let gz = tar_builder.into_inner().unwrap();
        gz.finish().unwrap();

        // Extract
        let result = extract_from_tar_gz(&tar_gz_path, "my-cli", &dir);
        assert!(result.is_ok(), "extract failed: {:?}", result.err());
        let extracted = result.unwrap();
        assert_eq!(extracted.file_name().unwrap().to_str().unwrap(), "my-cli");
        assert_eq!(
            std::fs::read_to_string(&extracted).unwrap(),
            "binary-content-tar"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_from_zip_missing_binary() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("colony_test_zip_missing");
        let _ = std::fs::create_dir_all(&dir);

        let zip_path = dir.join("empty.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        zip_writer
            .start_file("other-file", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip_writer.write_all(b"data").unwrap();
        zip_writer.finish().unwrap();

        let result = extract_from_zip(&zip_path, "nonexistent", &dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        let _ = std::fs::remove_dir_all(&dir);
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
        assert!(verify_sha256(&file_path, expected).is_ok());

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

        assert!(verify_sha256(&file_path, "0000000000000000").is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
