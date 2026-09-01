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
use crate::persistence::colony_cache_dir;

/// How long a download may stall before we give up. This is an *inactivity*
/// budget, not a total one: a slow-but-alive link keeps its transfer, a dead
/// socket still dies promptly.
const DOWNLOAD_READ_TIMEOUT: Duration = Duration::from_secs(60);

/// Build the HTTP client used for large asset downloads.
///
/// Deliberately no `.timeout()`: that is a *total* deadline covering connect,
/// TLS, redirects and the whole body stream, so a 300 s cap made any asset
/// larger than the line could carry in five minutes impossible to fetch at all
/// (a 40 MB binary needed a sustained ~1.2 Mbit/s or it could never finish, on
/// every attempt, with no partial progress kept). A read timeout bounds the
/// only thing worth bounding - a connection that has stopped delivering bytes.
fn download_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(format!("Colony-Launcher/{APP_VERSION}"))
        .read_timeout(DOWNLOAD_READ_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()?)
}

/// Sanity ceiling on an advertised body. The whole asset is written to disk
/// before any signature or digest check can run - that is unavoidable for a
/// detached signature - so a misconfigured or hostile release could otherwise
/// fill the user's home partition on its own say-so. Generous: the largest
/// asset the org ships is two orders of magnitude below this.
const MAX_ASSET_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Identity of the transfer a `.part` file belongs to, written beside it.
///
/// Resuming means stitching two responses into one file, so it is only sound
/// while both halves describe the same artifact. A re-tagged release, or an
/// asset rebuilt under the same name, must restart from zero rather than
/// produce a file that never existed anywhere.
#[derive(PartialEq, Eq)]
struct PartIdentity {
    etag: String,
    total: u64,
}

impl PartIdentity {
    fn path(dest: &std::path::Path) -> PathBuf {
        let mut name = dest.as_os_str().to_os_string();
        name.push(".id");
        PathBuf::from(name)
    }

    fn read(dest: &std::path::Path) -> Option<Self> {
        let raw = std::fs::read_to_string(Self::path(dest)).ok()?;
        let (etag, total) = raw.split_once('\n')?;
        Some(Self {
            etag: etag.to_string(),
            total: total.trim().parse().ok()?,
        })
    }

    fn write(&self, dest: &std::path::Path) {
        let _ = std::fs::write(Self::path(dest), format!("{}\n{}\n", self.etag, self.total));
    }

    fn forget(dest: &std::path::Path) {
        let _ = std::fs::remove_file(Self::path(dest));
    }
}

/// Whether a 206 response still describes the artifact a partial file belongs
/// to. Both the validator and the total length must agree.
fn response_matches_identity(resp: &reqwest::Response, id: &PartIdentity) -> bool {
    let etag = resp
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok());
    if etag != Some(id.etag.as_str()) {
        return false;
    }
    // "bytes <start>-<end>/<total>" - the total is the part we can compare.
    resp.headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit('/').next())
        .and_then(|total| total.trim().parse::<u64>().ok())
        == Some(id.total)
}

/// Move `new` onto `dest`, even when `dest` is a binary that is currently
/// running.
///
/// `std::fs::rename` over a live executable fails on Windows: the image is held
/// with FILE_SHARE_READ|FILE_SHARE_DELETE, so MoveFileExW cannot delete the
/// destination and returns ERROR_ACCESS_DENIED. Colony already knew the parade
/// and applied it to its OWN self-update - rename the running file aside first,
/// then move the new one in - but the app installer never learned it, so
/// updating an app the user had left open failed AFTER downloading and
/// verifying the whole asset, with a raw "Access is denied" and no hint that
/// closing the app would fix it.
///
/// The aside-rename is harmless on Unix (where the plain rename would have
/// worked anyway), so this takes the same path on every platform rather than
/// keeping two behaviours that only one of them exercises.
fn replace_file(new: &std::path::Path, dest: &std::path::Path) -> std::io::Result<()> {
    if !dest.exists() {
        return std::fs::rename(new, dest);
    }
    let aside = dest.with_extension("old");
    let _ = std::fs::remove_file(&aside);
    // A running image can always be RENAMED, on every platform - that is the
    // whole trick. If even this fails, the destination is genuinely locked and
    // the caller gets the real error instead of a mystery.
    std::fs::rename(dest, &aside)?;
    match std::fs::rename(new, dest) {
        Ok(()) => {
            // Best effort: on Windows the old image stays until the process
            // holding it exits, and prune_staging() sweeps it at next boot.
            let _ = std::fs::remove_file(&aside);
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::rename(&aside, dest);
            Err(e)
        }
    }
}

/// Discard a partial transfer and the identity that described it.
fn discard_partial(dest_path: &std::path::Path) {
    let _ = std::fs::remove_file(dest_path);
    PartIdentity::forget(dest_path);
}

/// Stream an HTTP GET to `dest_path`, sending throttled progress over
/// `progress_tx`, resuming a previous partial transfer when one is present and
/// provably describes the same bytes.
///
/// Verifies the received length against Content-Length when present and rejects
/// empty/truncated downloads. A partial file is KEPT on a transport failure so
/// the next attempt can continue it, and removed on anything that makes it
/// meaningless. Shared by app-asset install and launcher self-update.
async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
    dest_path: &std::path::Path,
    progress_tx: Option<futures::channel::mpsc::UnboundedSender<(u64, Option<u64>)>>,
) -> Result<()> {
    // What is already on disk, and may we continue it? Both the server's ETag
    // and the total length must match what the previous attempt recorded.
    let resume_from = match (
        std::fs::metadata(dest_path).map(|m| m.len()).ok(),
        PartIdentity::read(dest_path),
    ) {
        (Some(have), Some(id)) if have > 0 && have < id.total => Some((have, id)),
        // Anything else - no identity file, a complete or over-long file, an
        // empty one - is not resumable. Start clean.
        _ => {
            discard_partial(dest_path);
            None
        }
    };

    let mut request = client.get(url);
    if let Some(t) = token {
        request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if let Some((have, ref id)) = resume_from {
        // If-Range is sent, but is NOT trusted: GitHub redirects release assets
        // to Azure blob storage, which ignores the header and answers 206 to a
        // stale validator just the same (verified against a real release
        // asset). So the 206 branch below re-checks the ETag and the total
        // itself rather than taking the status code as proof.
        request = request
            .header(reqwest::header::RANGE, format!("bytes={have}-"))
            .header(reqwest::header::IF_RANGE, id.etag.clone());
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

    // A 206 is a claim, not a proof - the storage backend ignores If-Range.
    // Accept it only when the response still describes the artifact the partial
    // file belongs to: same ETag, and the same total in Content-Range. On any
    // disagreement, throw the partial away and let the retry start clean rather
    // than stitching two different bodies into a file that never existed.
    let resuming = if resp.status().as_u16() == 206 {
        match resume_from {
            Some((_, ref id)) if response_matches_identity(&resp, id) => true,
            Some(_) => {
                discard_partial(dest_path);
                anyhow::bail!("The release changed while downloading {url}; restarting");
            }
            None => false,
        }
    } else {
        false
    };
    let already: u64 = if resuming {
        resume_from.as_ref().map(|(have, _)| *have).unwrap_or(0)
    } else {
        0
    };

    // Total size of the WHOLE asset, not of this response.
    let total = if resuming {
        resume_from.as_ref().map(|(_, id)| id.total)
    } else {
        resp.content_length()
    };

    if let Some(total) = total {
        anyhow::ensure!(
            total <= MAX_ASSET_BYTES,
            "Refusing {url}: the release advertises {total} bytes, over Colony's {MAX_ASSET_BYTES}-byte ceiling"
        );
    }

    use futures::StreamExt;
    use std::io::Write;
    let mut file = if resuming {
        tracing::info!("resuming {url} at {already} bytes");
        std::fs::OpenOptions::new().append(true).open(dest_path)?
    } else {
        // Fresh transfer. Record what we are about to fetch so a later attempt
        // can tell whether continuing this file is sound, and never follow
        // whatever sits at the (predictable) staging name.
        discard_partial(dest_path);
        if let (Some(etag), Some(total)) = (
            resp.headers()
                .get(reqwest::header::ETAG)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string),
            resp.content_length(),
        ) {
            PartIdentity { etag, total }.write(dest_path);
        }
        create_new_file(dest_path)?
    };
    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = already;
    let mut last_pct: u32 = 0;

    let stream_result: Result<()> = async {
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            anyhow::ensure!(
                downloaded <= MAX_ASSET_BYTES,
                "Refusing {url}: body exceeded Colony's {MAX_ASSET_BYTES}-byte ceiling"
            );

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
        // KEEP the partial file. A drop at 95% used to throw away everything
        // and charge the user the full asset again on the next click; with the
        // identity sidecar beside it, the next attempt continues instead. Only
        // a transfer we can no longer describe is discarded.
        if PartIdentity::read(dest_path).is_none() {
            discard_partial(dest_path);
        }
        return Err(e);
    }

    // Guard against a silently-truncated or empty transfer. Both are terminal
    // for THIS file: a short body means the server disagrees with the length it
    // advertised, so continuing from it would be guesswork.
    if let Some(total) = total {
        if downloaded != total {
            discard_partial(dest_path);
            anyhow::bail!("Incomplete download: got {downloaded} of {total} bytes for {url}");
        }
    }
    if downloaded == 0 {
        discard_partial(dest_path);
        anyhow::bail!("Empty download (0 bytes) for {url}");
    }

    // Complete: the identity file has done its job.
    PartIdentity::forget(dest_path);
    Ok(())
}

/// `download_to_file` with a bounded retry, so a dropped connection continues
/// from where it stopped without the user having to notice and click again.
///
/// Only transport failures are retried, and only because the partial file now
/// survives them: each attempt resumes from what the previous one wrote, so
/// three tries cost three connections, not three assets. A verification failure
/// never reaches here - it happens after this returns.
async fn download_with_resume(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
    dest_path: &std::path::Path,
    progress_tx: Option<futures::channel::mpsc::UnboundedSender<(u64, Option<u64>)>>,
) -> Result<()> {
    const ATTEMPTS: u32 = 3;
    let mut last_err = None;
    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            // Linear backoff; the point is to ride out a blip, not to hammer.
            tokio::time::sleep(Duration::from_secs(2 * attempt as u64)).await;
            tracing::info!("retrying {url} (attempt {} of {ATTEMPTS})", attempt + 1);
        }
        let had_partial = PartIdentity::read(dest_path).is_some();
        match download_to_file(client, url, token, dest_path, progress_tx.clone()).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                // Retry when the next attempt would do something DIFFERENT:
                // either a partial survived and will be continued, or one was
                // just discarded (the release changed under us) and the next
                // attempt starts clean. Otherwise - a 404, a refused URL - the
                // next attempt would only repeat this one.
                let resumable = PartIdentity::read(dest_path).is_some();
                if !resumable && !had_partial {
                    break;
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Download failed for {url}")))
}

/// Ceiling for the small sidecars Colony buffers whole (`.sig` is 64 bytes,
/// `.meta` three short lines). Whoever controls a release can publish a
/// multi-gigabyte file named `foo-linux.sig`; without a cap that is an OOM the
/// moment a user clicks Install. Generous by four orders of magnitude.
const MAX_SIDECAR_BYTES: u64 = 64 * 1024;

/// Read a response body with an upper bound, so a body with no Content-Length -
/// or one that lies about it - cannot be unbounded.
async fn bounded_body(resp: reqwest::Response, url: &str, max: u64) -> Result<Vec<u8>> {
    if let Some(len) = resp.content_length() {
        anyhow::ensure!(
            len <= max,
            "Refusing {url}: declares {len} bytes, over the {max}-byte limit"
        );
    }
    use futures::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut out: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        anyhow::ensure!(
            out.len() as u64 + chunk.len() as u64 <= max,
            "Refusing {url}: body exceeds the {max}-byte limit"
        );
        out.extend_from_slice(&chunk);
    }
    Ok(out)
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
    Ok(Some(bounded_body(resp, url, MAX_SIDECAR_BYTES).await?))
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
    bounded_body(resp, url, MAX_SIDECAR_BYTES).await
}

/// Verify a SHA256 digest over bytes already in memory.
///
/// Takes bytes rather than a path so the caller checks exactly what it is about
/// to install: re-opening the staged file to hash it means the digest describes
/// one read and the install uses another.
fn verify_sha256_bytes(bytes: &[u8], expected_hex: &str) -> Result<()> {
    let computed = format!("{:x}", Sha256::digest(bytes));
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

/// Ensure a filename is a single normal path component (no `..`, no path
/// separators, not absolute) before it is joined into a destination directory.
/// Shared by archive extraction and raw-asset download to block path traversal.
/// Names Win32 resolves to devices no matter which directory contains them,
/// and regardless of any extension (`CON.txt` is still the console).
const RESERVED_DEVICE_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub(crate) fn ensure_safe_component(name: &str) -> Result<()> {
    let p = std::path::Path::new(name);
    anyhow::ensure!(
        p.components().count() == 1
            && matches!(p.components().next(), Some(std::path::Component::Normal(_))),
        "Invalid file name (path traversal attempt?): {name}"
    );

    // The check above is the shape of a POSIX path, which is only half the
    // question on the three platforms Colony supports. Rust's Windows parser
    // recognises a drive prefix only when exactly one letter precedes the
    // colon, so "payload:stream" is a single Normal component and joining it
    // writes an NTFS alternate data stream on a file named "payload"; a
    // reserved device name resolves to a device from any directory; and a
    // trailing dot or space is stripped by the filesystem, so the name written
    // differs from the name checked.
    //
    // Enforced on every platform, not behind cfg(windows): the catalog is
    // shared, so a manifest that would be refused on Windows must be refused
    // everywhere rather than installing differently per user.
    anyhow::ensure!(
        !name.contains(
            |c: char| matches!(c, ':' | '\\' | '/' | '<' | '>' | '"' | '|' | '?' | '*')
                || c.is_control()
        ),
        "Invalid file name (reserved character): {name}"
    );
    anyhow::ensure!(
        !name.ends_with('.') && !name.ends_with(' '),
        "Invalid file name (trailing dot or space is silently stripped): {name}"
    );
    let stem = name.split('.').next().unwrap_or(name);
    anyhow::ensure!(
        !RESERVED_DEVICE_NAMES
            .iter()
            .any(|d| stem.eq_ignore_ascii_case(d)),
        "Invalid file name (reserved device name): {name}"
    );
    Ok(())
}

/// Build a URL from a trusted base and remote-controlled path segments,
/// percent-encoding each segment so it can never be structural.
///
/// `format!`-ing a remote string into a URL is not safe even when the string
/// looks harmless in a JSON diff. `reqwest::Client::get` parses through the
/// WHATWG URL parser, which collapses `..` segments (and `%2e%2e`) *before* the
/// request is made: a `colony.json` whose `tag` reads
/// `v1/../../../../../EvilOrg/EvilRepo/releases/download/v1` shortens the path
/// into a different account entirely, and Colony would install and trust bytes
/// that were never published under the org. That is the one containment claim
/// the whole trust model rests on, and it was reachable from write access to a
/// single line of a catalog repo - no release-publishing rights needed.
///
/// Segments are pushed through `path_segments_mut`, which percent-encodes them,
/// so a `/` or a `..` inside a segment stays data. Legitimate git tags may
/// contain `/` (`release/1.0`), so encoding is the honest fix here; rejecting
/// the value outright would break them.
pub(crate) fn build_url(base: &str, segments: &[&str]) -> Result<String> {
    let mut url = reqwest::Url::parse(base)?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow::anyhow!("URL base cannot have path segments: {base}"))?;
        for segment in segments {
            anyhow::ensure!(
                !segment.is_empty(),
                "empty URL path segment for base {base}"
            );
            path.push(segment);
        }
    }
    Ok(url.into())
}

/// Normalized form of `raw` when it is an absolute `http`/`https` URL with a
/// host, else `None`.
///
/// Gate for everything handed to the desktop's URI opener: link destinations in
/// remote Markdown (READMEs, release notes) reach it verbatim, and `open::that`
/// is not a browser call — it execs `xdg-open`/`gio open` on Linux and
/// `Start-Process` on Windows, both of which dispatch `file://`, UNC paths and
/// any registered `x-scheme-handler/*`. A `file://` or custom-scheme link in a
/// hostile README would otherwise be a one-click execution primitive that
/// bypasses every signature and digest check in this module.
///
/// Returns the REPARSED string rather than a bool on purpose: the WHATWG parser
/// strips tabs, newlines and leading control characters anywhere in the input, so
/// validating `raw` and then opening `raw` would judge one string and execute a
/// different one (`"ht\ntps://x"` parses as `https://x`). Callers open what was
/// actually validated.
pub(crate) fn web_url(raw: &str) -> Option<String> {
    // Relative and scheme-less inputs (including protocol-relative
    // `//host/share/x.exe`) fail to parse, which is the desired answer.
    let parsed = reqwest::Url::parse(raw).ok()?;
    // A host is required: `http:evil` and `http:/x` parse but address nothing,
    // and an opener may fall back to treating them as a local path.
    if !matches!(parsed.scheme(), "http" | "https") || !parsed.has_host() {
        return None;
    }
    Some(parsed.into())
}

/// Create `path` fresh, never following an existing file or symlink.
///
/// `File::create` truncates whatever it finds, and follows a symlink to write at
/// its target: a symlink pre-planted at a predictable staging name (`<binary>.new`,
/// `<asset>.part`) would redirect the write outside the install directory, and the
/// caller's `set_permissions` then chmods that target. Unlinking first and opening
/// with `create_new` makes the create fail rather than follow anything that races
/// in between.
fn create_new_file(path: &std::path::Path) -> Result<std::fs::File> {
    // Removing our own leftover staging file is expected; a failure here is not
    // fatal because create_new below is the actual guard.
    let _ = std::fs::remove_file(path);
    Ok(std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?)
}

/// `std::fs::write` for the trust path: same result, but through
/// [`create_new_file`] so a symlink planted at the (entirely predictable)
/// staging name is never followed.
fn write_new_file(path: &std::path::Path, contents: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut file = create_new_file(path)?;
    file.write_all(contents)?;
    file.flush()?;
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
            let mut out = create_new_file(&tmp_dest)?;
            std::io::copy(&mut entry, &mut out)?;
            drop(out);
            replace_file(&tmp_dest, &final_dest)?;
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
            replace_file(&tmp_dest, &final_dest)?;
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
    // `binary_name` is a manifest-supplied string that becomes a path component
    // in EVERY branch below, so the traversal guard is hoisted here, before the
    // dispatch. It used to live only inside the zip and tar extractors, which
    // left the raw-binary branch able to write outside `dest_dir` — and, after
    // the caller's chmod 0755 and desktop entry, to gain execution at next login
    // — from a hostile manifest. The extractors keep their own call so they stay
    // safe in isolation.
    ensure_safe_component(binary_name)?;
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
        replace_file(archive_path, &dest)?;
        Ok(dest)
    }
}

/// Everything needed to install one resolved release asset.
pub struct AssetInstall {
    pub repo_name: String,
    /// The manifest's category, so the desktop menu files the app where it
    /// belongs instead of dumping everything under Utility.
    pub category: crate::scan::AppCategory,
    /// What the desktop menu should call this app: the manifest's declared
    /// name, which may differ from the repo slug. The slug stays the identity
    /// key everywhere else (install dir, caches, entry filename).
    pub display_name: String,
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
        display_name,
        category,
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

    let dest_dir = crate::persistence::colony_app_dir(&repo_name)?;
    std::fs::create_dir_all(&dest_dir)?;
    let dest_path = dest_dir.join(&filename);
    // Download to a temporary sibling so an interrupted or failed transfer
    // never truncates the currently-installed binary.
    let temp_path = dest_dir.join(format!("{filename}.part"));

    // Every segment after the host is remote-controlled (`repo_name` from the
    // API listing, `tag` and `filename` from colony.json), so build the URL
    // from encoded segments instead of interpolating them.
    let url = build_url(
        "https://github.com",
        &[
            GITHUB_ACCOUNT,
            &repo_name,
            "releases",
            "download",
            &tag,
            &filename,
        ],
    )?;

    let client = download_client()?;
    download_with_resume(&client, &url, token.as_deref(), &temp_path, progress_tx).await?;

    // `manifest.signed` lives in the very repo the signature protects, so a
    // compromised repo could flip it to false and drop the `.sig` to install
    // unsigned code silently. Pin it: once an install of this app has been
    // signature-verified, later updates must stay signed whatever the manifest
    // now claims. The pin only ever raises the bar.
    let signature_pinned = crate::persistence::load_installed_signed(&repo_name);
    let require_signature = require_signature || signature_pinned;

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
    // Roll the signed metadata sidecar down to apps. A bare `.sig` proves only
    // that the bytes came from the org key - not WHICH artefact or version they
    // are - so a compromised maintainer could take an old, genuinely signed,
    // known-vulnerable build, re-upload it under a new tag, and Colony would
    // install it with every trust indicator green. The sidecar binds the bytes
    // to an asset name, a digest and a version; the machinery already existed
    // and was reachable only from the launcher's own self-update.
    //
    // Opportunistic and pinned, exactly like `.sig`: an app that publishes no
    // sidecar today still installs, but once one has been verified, a later
    // release cannot silently stop publishing it.
    let metadata_pinned = crate::persistence::load_installed_metadata(&repo_name);
    let metadata = match fetch_optional_bytes(
        &client,
        &format!("{url}{}", crate::signing::METADATA_SUFFIX),
        token.as_deref(),
    )
    .await
    {
        Ok(Some(meta_bytes)) => {
            // The sidecar is only worth anything signed. A missing signature
            // for a PRESENT sidecar is not "legacy", it is a broken release.
            let meta_sig = fetch_optional_bytes(
                &client,
                &format!(
                    "{url}{}{}",
                    crate::signing::METADATA_SUFFIX,
                    crate::signing::SIGNATURE_SUFFIX
                ),
                token.as_deref(),
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!("Could not fetch the metadata signature for {filename}: {e}")
            })
            .and_then(|sig| {
                sig.ok_or_else(|| {
                    anyhow::anyhow!(
                        "{filename}{} was published without {filename}{}{} - refusing to install",
                        crate::signing::METADATA_SUFFIX,
                        crate::signing::METADATA_SUFFIX,
                        crate::signing::SIGNATURE_SUFFIX
                    )
                })
            });
            match meta_sig {
                Ok(meta_sig) => Some((meta_bytes, meta_sig)),
                Err(e) => {
                    discard_partial(&temp_path);
                    return Err(e);
                }
            }
        }
        Ok(None) => None,
        Err(e) => {
            discard_partial(&temp_path);
            anyhow::bail!("Could not check for release metadata of {filename}: {e}");
        }
    };
    if metadata_pinned && metadata.is_none() {
        discard_partial(&temp_path);
        anyhow::bail!(
            "{repo_name} was previously installed with verified release metadata, but this release publishes no {filename}{} - refusing to install (uninstall it first to opt back out)",
            crate::signing::METADATA_SUFFIX
        );
    }

    if require_signature && signature.is_none() {
        let _ = std::fs::remove_file(&temp_path);
        // Say WHICH rule refused, because the two have different remedies: the
        // manifest is the repo's own declaration, whereas the pin is our memory
        // of a previously verified install and can only be cleared by
        // uninstalling the app.
        if signature_pinned {
            anyhow::bail!(
                "{repo_name} was previously installed with a verified signature, but this release publishes no {filename}.sig - refusing to install an unsigned downgrade of a signed app (uninstall it first to opt back out)"
            );
        }
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

        let installed_version = crate::persistence::load_installed_version(&repo_name);
        let resolved_tag = tag.clone();
        tokio::task::spawn_blocking(move || -> Result<PathBuf> {
            let was_signed = signature.is_some();
            let had_metadata = metadata.is_some();
            // Read the staged file ONCE and check everything against that one
            // buffer. Each separate read of `<app dir>/<filename>.part` - a
            // predictable path - is another chance to check one set of bytes
            // and install a different set; the launcher path already avoids
            // this by installing the buffer it verified.
            let staged_bytes =
                if signature.is_some() || metadata.is_some() || expected_sha256.is_some() {
                    Some(std::fs::read(&temp_path)?)
                } else {
                    None
                };
            if let Some(sig) = signature {
                let bytes = staged_bytes.as_deref().unwrap_or_default();
                if let Err(e) = crate::signing::verify_release_signature(bytes, &sig) {
                    let _ = std::fs::remove_file(&temp_path);
                    anyhow::bail!(
                        "Signature verification FAILED for {filename} - refusing to install: {e}"
                    );
                }
                tracing::info!("ed25519 signature verified for {filename}");
            }
            // The sidecar, verified against the same trusted keys and then
            // bound to THIS asset, THIS digest and THIS tag - which is what a
            // bare signature cannot say.
            if let Some((meta_bytes, meta_sig)) = metadata {
                let bytes = staged_bytes.as_deref().unwrap_or_default();
                let checked = crate::signing::verify_release_signature(&meta_bytes, &meta_sig)
                    .and_then(|()| crate::signing::ReleaseMetadata::parse(&meta_bytes))
                    .and_then(|meta| {
                        check_metadata_bindings(
                            &meta,
                            bytes,
                            &filename,
                            Some(resolved_tag.as_str()),
                        )?;
                        // ">=", not ">": an app pinned to a fixed tag
                        // legitimately reinstalls the same version, so the
                        // launcher's strictly-newer rule would lock it out.
                        ensure_not_a_downgrade(&meta, installed_version.as_deref())?;
                        Ok(())
                    });
                if let Err(e) = checked {
                    let _ = std::fs::remove_file(&temp_path);
                    anyhow::bail!(
                        "Release metadata check FAILED for {filename} - refusing to install: {e}"
                    );
                }
                tracing::info!("signed release metadata verified for {filename}");
            }
            if let Some(ref expected) = expected_sha256 {
                let bytes = staged_bytes.as_deref().unwrap_or_default();
                if let Err(e) = verify_sha256_bytes(bytes, expected) {
                    let _ = std::fs::remove_file(&temp_path);
                    return Err(e);
                }
                tracing::info!("SHA256 verified for {filename}");
            }
            drop(staged_bytes);

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
                replace_file(&temp_path, &dest_path)?;
                dest_path
            };

            // An app whose asset name carries the version - which is the case
            // `filePattern` exists to serve - installs the new release under a
            // NEW filename, so the previous binary is simply orphaned. Nothing
            // ever showed or reclaimed it: SphereCord's AppImage is 166 MB, so
            // three updates left half a gigabyte of invisible junk that only an
            // uninstall would clear. Drop the superseded file now that the new
            // one is committed.
            if let Some(stale) = crate::persistence::load_installed_asset(&repo_name) {
                if stale != filename
                    && Some(stale.as_str()) != binary_name.as_deref()
                    && ensure_safe_component(&stale).is_ok()
                {
                    let stale_path = dest_dir.join(&stale);
                    if stale_path != final_path && stale_path.is_file() {
                        match std::fs::remove_file(&stale_path) {
                            Ok(()) => tracing::info!("removed superseded binary {stale}"),
                            Err(e) => tracing::warn!("could not remove superseded {stale}: {e}"),
                        }
                    }
                }
            }

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
            //
            // Order matters: the ASSET marker lands first and the VERSION
            // marker last. The two writes are individually non-atomic, so a
            // kill between them leaves a torn state - and only this order makes
            // that state honest. With the version written first, a
            // filePattern app whose asset name carries the version would claim
            // the new version while `.colony_asset` still named the old file:
            // installed_app_path resolves through the asset marker, so Colony
            // would report the new version and Launch would run the old binary,
            // with no update offered to correct it. This way round, a torn
            // install simply re-offers the update.
            if record_asset {
                crate::persistence::save_installed_asset(&repo_name, &filename)?;
            }
            crate::persistence::save_installed_version(&repo_name, &tag)?;
            // Pin the signature requirement for future updates: a repo that
            // ships signatures today must not be able to stop tomorrow. Only
            // ever raises the bar - the marker is written, never cleared, while
            // the app stays installed.
            if was_signed {
                crate::persistence::save_installed_signed(&repo_name)?;
            }
            // Same one-way ratchet for the sidecar: a repo that binds its
            // releases today must not be able to stop tomorrow.
            if had_metadata {
                crate::persistence::save_installed_metadata(&repo_name)?;
            }
            // Desktop integration (Linux): index the installed app in the
            // desktop environment. Best-effort - a failure here must not fail
            // an otherwise complete install.
            if let Err(e) = crate::persistence::write_desktop_entry(
                &repo_name,
                &display_name,
                category,
                &final_path,
            ) {
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

/// Verify a signed metadata sidecar and enforce what it binds.
///
/// Checks, in order: the sidecar is signed by the release key; it describes the
/// asset we asked for; its digest matches the bytes in hand; it names the tag the
/// update check selected (`expected_tag`, unknown at apply time hence optional);
/// and its version is strictly newer than the running build. That last check is
/// the anti-rollback - without it, any older org-signed binary is a valid
/// "update".
fn verify_launcher_metadata(
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
fn check_metadata_bindings(
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

    let digest = format!("{:x}", Sha256::digest(binary_bytes));
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
fn ensure_not_a_downgrade(
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-shot HTTP server that TRUNCATES the first response and honours
    /// Range on the next one - the shape of a real dropped connection.
    ///
    /// Hand-rolled on `std::net` rather than pulled in as a dependency: the
    /// point is to prove the resume path end to end without adding a test-only
    /// crate to a project that counts its dependencies.
    fn spawn_truncating_server(
        body: Vec<u8>,
        cut: usize,
        etag: &str,
    ) -> (String, std::thread::JoinHandle<()>) {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}/asset", listener.local_addr().unwrap());
        let etag = etag.to_string();
        let handle = std::thread::spawn(move || {
            // Two connections: the truncated one, then the ranged one.
            for _ in 0..2 {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                let mut reader = BufReader::new(&stream);
                let mut range_start: Option<usize> = None;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                        range_start = v.split('-').next().and_then(|n| n.trim().parse().ok());
                    }
                    if line == "\r\n" || line == "\n" {
                        break;
                    }
                }
                let mut stream = &stream;
                match range_start {
                    Some(start) => {
                        let chunk = &body[start..];
                        let head = format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nETag: {}\r\nConnection: close\r\n\r\n",
                            start,
                            body.len() - 1,
                            body.len(),
                            chunk.len(),
                            etag
                        );
                        let _ = stream.write_all(head.as_bytes());
                        let _ = stream.write_all(chunk);
                    }
                    None => {
                        let head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: {}\r\nConnection: close\r\n\r\n",
                            body.len(),
                            etag
                        );
                        let _ = stream.write_all(head.as_bytes());
                        // Promise the whole body, deliver part of it, hang up.
                        let _ = stream.write_all(&body[..cut]);
                    }
                }
                let _ = stream.flush();
            }
        });
        (url, handle)
    }

    /// The behaviour this whole batch exists for: a connection that died at 40%
    /// used to throw away every byte and charge the user the full asset again.
    #[test]
    fn a_dropped_connection_is_resumed_instead_of_restarted() {
        let body: Vec<u8> = (0..300_000usize)
            .map(|i| ((i * 7 + 3) % 256) as u8)
            .collect();
        let (url, server) = spawn_truncating_server(body.clone(), 120_000, "\"v1\"");

        let dir = std::env::temp_dir().join("colony_test_resume");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("asset.part");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(async {
            let client = download_client().unwrap();
            download_with_resume(&client, &url, None, &dest, None).await
        });

        assert!(
            result.is_ok(),
            "the retry must finish the transfer: {result:?}"
        );
        assert_eq!(
            std::fs::read(&dest).unwrap(),
            body,
            "the stitched file must be byte-identical to the asset"
        );
        assert!(
            !PartIdentity::path(&dest).exists(),
            "a completed transfer leaves no identity file behind"
        );

        let _ = std::fs::remove_dir_all(&dir);
        let _ = server.join();
    }

    /// Resuming stitches two responses into one file, so it is only sound
    /// while both describe the same artifact. The identity sidecar is what
    /// makes that decidable; without it, a re-tagged release could be silently
    /// assembled from two different bodies.
    #[test]
    fn a_partial_transfer_is_only_resumable_against_its_own_identity() {
        let dir = std::env::temp_dir().join("colony_test_part_identity");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let part = dir.join("grape-linux.part");

        // A partial transfer plus the identity that describes it.
        std::fs::write(&part, vec![0u8; 512]).unwrap();
        PartIdentity {
            etag: "\"abc\"".into(),
            total: 4096,
        }
        .write(&part);

        let id = PartIdentity::read(&part).expect("identity round-trips");
        assert_eq!(id.etag, "\"abc\"");
        assert_eq!(id.total, 4096);
        assert!(
            std::fs::metadata(&part).unwrap().len() < id.total,
            "a short file against a known total is what makes a resume possible"
        );

        // Discarding takes the sidecar with it, so the next attempt cannot
        // resume from bytes nothing describes.
        discard_partial(&part);
        assert!(!part.exists());
        assert!(PartIdentity::read(&part).is_none());

        // A truncated or garbage sidecar is not an identity.
        std::fs::write(PartIdentity::path(&part), "no-newline-here").unwrap();
        assert!(PartIdentity::read(&part).is_none());
        std::fs::write(PartIdentity::path(&part), "\"abc\"\nnot-a-number\n").unwrap();
        assert!(PartIdentity::read(&part).is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A `tag` from colony.json used to be interpolated straight into the
    /// release URL. reqwest parses with the WHATWG parser, which collapses
    /// `..` BEFORE the request is issued, so one line of a catalog repo could
    /// redirect the install to an account outside the org entirely - defeating
    /// the only containment claim the trust model has.
    #[test]
    fn a_hostile_tag_cannot_walk_the_release_url_out_of_the_org() {
        let hostile = "v1/../../../../../EvilOrg/EvilRepo/releases/download/v1";
        let url = build_url(
            "https://github.com",
            &[
                "Project-Colony",
                "Grape",
                "releases",
                "download",
                hostile,
                "grape-linux",
            ],
        )
        .expect("segments are encoded, not rejected");

        assert!(
            url.starts_with("https://github.com/Project-Colony/Grape/releases/download/"),
            "the request must stay inside the org, got {url}"
        );
        assert!(
            !url.contains("EvilOrg/EvilRepo/releases"),
            "the traversal must not survive as structure, got {url}"
        );

        // Reparsing is what reqwest itself does; the collapse must not happen
        // there either, which is the whole point of encoding the segment.
        let reparsed = reqwest::Url::parse(&url).expect("valid URL");
        assert_eq!(
            reparsed.path_segments().map(|s| s.count()),
            Some(6),
            "no segment may be structural: {url}"
        );

        // A legitimate tag containing a slash (`release/1.0`) still works -
        // rejecting `/` outright would have broken real repos.
        let ok = build_url(
            "https://github.com",
            &[
                "Project-Colony",
                "Grape",
                "releases",
                "download",
                "release/1.0",
                "grape-linux",
            ],
        )
        .expect("slashes in a tag are encoded, not an error");
        assert!(ok.contains("release%2F1.0"), "got {ok}");
    }

    /// The raw-binary branch of `extract_binary_from_archive` used to join the
    /// manifest's `binary` field straight into the install dir, so a hostile
    /// manifest could write anywhere (the caller then chmods 0755 and writes a
    /// desktop entry pointing at it). The guard is now hoisted above the branch.
    #[test]
    fn raw_binary_branch_rejects_traversal_in_binary_name() {
        // Every escape target below must ALREADY EXIST as a directory, so that
        // without the guard the rename would genuinely succeed. A target whose
        // parent is missing would make this test pass on ENOENT alone, and it
        // would stay green with the fix reverted.
        let root = std::env::temp_dir().join("colony_test_raw_traversal");
        let _ = std::fs::remove_dir_all(&root);
        let dest_dir = root.join("apps").join("SomeApp");
        std::fs::create_dir_all(&dest_dir).unwrap();
        std::fs::create_dir_all(dest_dir.join("sub")).unwrap();
        let outside = root.join("apps").join("escaped");
        let staged = dest_dir.join("app.bin.part");

        for hostile in ["../escaped", "../../apps/escaped", "sub/nested", ".."] {
            std::fs::write(&staged, b"payload").unwrap();
            // `app.bin` has no archive extension, so this hits the raw branch.
            let result = extract_binary_from_archive(&staged, "app.bin", hostile, &dest_dir);
            assert!(
                result.is_err(),
                "traversal must be refused for binary name {hostile:?}"
            );
            assert!(
                staged.exists(),
                "a refused extraction must leave staging intact ({hostile:?})"
            );
            assert!(
                !outside.exists(),
                "nothing may be written outside the install dir ({hostile:?})"
            );
            assert!(
                !dest_dir.join("sub").join("nested").exists(),
                "nothing may be written below the install dir ({hostile:?})"
            );
            std::fs::remove_file(&staged).unwrap();
        }

        // Control: the same call with a plain name DOES install. Without it, the
        // assertions above could be passing because of a broken fixture.
        std::fs::write(&staged, b"payload").unwrap();
        let installed = extract_binary_from_archive(&staged, "app.bin", "app", &dest_dir).unwrap();
        assert_eq!(installed, dest_dir.join("app"));
        assert!(installed.exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn only_http_and_https_urls_may_reach_the_uri_opener() {
        for ok in [
            "https://github.com/Project-Colony/Colony",
            "http://example.com/a?b=c#d",
            "HTTPS://GitHub.com/x",
        ] {
            assert!(web_url(ok).is_some(), "{ok} should be allowed");
        }
        for bad in [
            // The one-click execution vectors from a hostile README.
            "file:///home/user/.local/share/applications/colony-app.desktop",
            "//198.51.100.7/share/setup.exe",
            "\\\\198.51.100.7\\share\\setup.exe",
            "javascript:alert(1)",
            "smb://host/share",
            "steam://run/1",
            "data:text/html,<script>1</script>",
            "mailto:a@b.c",
            "",
            "   ",
            "not a url",
            "/etc/passwd",
        ] {
            assert!(web_url(bad).is_none(), "{bad:?} must be refused");
        }
    }

    /// The WHATWG parser strips tabs, newlines and leading control characters
    /// anywhere in the input, so a bool-returning check would validate one string
    /// while `open::that` received a different one. CommonMark decodes character
    /// references in link destinations, so `[x](ht&#10;tps://evil)` produces
    /// exactly these shapes from a hostile README.
    #[test]
    fn opened_url_is_the_one_that_was_validated() {
        for sneaky in [
            "ht\ntps://evil.example/x",
            "ht\ttps://evil.example/x",
            "\thttps://evil.example/x",
            "  https://evil.example/x",
            "https://evil.example/\rx",
        ] {
            // These DO parse (that is the whole problem), so the contract is not
            // "refused" but "what comes back is clean": a real http(s) URL with no
            // control characters, never the raw string.
            let normalized = web_url(sneaky).expect("parses as https");
            assert_ne!(normalized, sneaky, "the raw form must not be handed on");
            assert!(normalized.starts_with("https://"), "{normalized}");
            assert!(
                !normalized.chars().any(|c| c.is_control()),
                "{normalized:?} still carries control characters"
            );
        }
        // Scheme-only forms are normalized by the parser rather than rejected:
        // for http(s) it fills in the host, which stays a plain web URL.
        assert_eq!(
            web_url("http:evil.example/x").as_deref(),
            Some("http://evil.example/x")
        );
    }

    fn meta_for(bytes: &[u8], version: &str, asset: &str) -> crate::signing::ReleaseMetadata {
        crate::signing::ReleaseMetadata {
            version: version.into(),
            asset: asset.into(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
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
