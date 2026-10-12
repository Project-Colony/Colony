//! Installing an app: resolve, download, verify, extract, and promote one
//! release asset into its directory together with its install records.

use anyhow::Result;
use std::path::PathBuf;

use super::files::{ensure_safe_component, extract_binary_from_archive, replace_file};
use super::http::{
    build_url, discard_partial, download_client, download_with_resume, fetch_optional_bytes,
};
use super::verify::{check_metadata_bindings, ensure_not_a_downgrade, verify_sha256_bytes};
use crate::github::GITHUB_ACCOUNT;

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
