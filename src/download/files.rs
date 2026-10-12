//! Writing into install and staging directories: single-component names,
//! creates that never follow a planted link, replacing a binary that is
//! running, and extracting one file from a zip or tar.gz archive.

use anyhow::Result;
use std::path::PathBuf;

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
pub(super) fn replace_file(new: &std::path::Path, dest: &std::path::Path) -> std::io::Result<()> {
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

/// Create `path` fresh, never following an existing file or symlink.
///
/// `File::create` truncates whatever it finds, and follows a symlink to write at
/// its target: a symlink pre-planted at a predictable staging name (`<binary>.new`,
/// `<asset>.part`) would redirect the write outside the install directory, and the
/// caller's `set_permissions` then chmods that target. Unlinking first and opening
/// with `create_new` makes the create fail rather than follow anything that races
/// in between.
pub(super) fn create_new_file(path: &std::path::Path) -> Result<std::fs::File> {
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
pub(super) fn write_new_file(path: &std::path::Path, contents: &[u8]) -> Result<()> {
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
        let entry_name = entry.name()?.into_owned();
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
pub(super) fn extract_binary_from_archive(
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
