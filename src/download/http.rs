//! HTTP streaming: the download client, resumable transfers checked against
//! their identity, bounded reads of small sidecars, and the URL helpers that
//! keep remote strings from becoming structure.

use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

use super::files::create_new_file;
use crate::github::{APP_VERSION, CONNECT_TIMEOUT};

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
pub(super) fn download_client() -> Result<reqwest::Client> {
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

/// Discard a partial transfer and the identity that described it.
pub(super) fn discard_partial(dest_path: &std::path::Path) {
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
pub(super) async fn download_with_resume(
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
pub(super) async fn fetch_optional_bytes(
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
pub(super) async fn fetch_bytes(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
) -> Result<Vec<u8>> {
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
/// is not a browser call - it execs `xdg-open`/`gio open` on Linux and
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
}
