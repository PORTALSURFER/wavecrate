//! Zip extraction helpers for updater payloads.

use std::{fs::File, path::Path};

use super::super::UpdateError;

const MAX_ZIP_ENTRIES: usize = 10_000;
const MAX_ZIP_ENTRY_UNCOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ZIP_TOTAL_UNCOMPRESSED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ZIP_COMPRESSION_RATIO: u64 = 200;

#[derive(Clone, Copy)]
struct ZipExtractionLimits {
    max_entries: usize,
    max_entry_uncompressed_bytes: u64,
    max_total_uncompressed_bytes: u64,
    max_compression_ratio: u64,
}

impl ZipExtractionLimits {
    fn standard() -> Self {
        Self {
            max_entries: MAX_ZIP_ENTRIES,
            max_entry_uncompressed_bytes: MAX_ZIP_ENTRY_UNCOMPRESSED_BYTES,
            max_total_uncompressed_bytes: MAX_ZIP_TOTAL_UNCOMPRESSED_BYTES,
            max_compression_ratio: MAX_ZIP_COMPRESSION_RATIO,
        }
    }
}

/// Extract a zip archive into a directory while enforcing safety limits.
pub(crate) fn unzip_to_dir(zip_path: &Path, dest_dir: &Path) -> Result<(), UpdateError> {
    unzip_to_dir_with_limits(zip_path, dest_dir, ZipExtractionLimits::standard())
}

#[cfg(unix)]
fn safe_unix_file_mode(archive_mode: u32) -> u32 {
    if archive_mode & 0o111 != 0 {
        0o755
    } else {
        0o644
    }
}

fn unzip_to_dir_with_limits(
    zip_path: &Path,
    dest_dir: &Path,
    limits: ZipExtractionLimits,
) -> Result<(), UpdateError> {
    let file = File::open(zip_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|err| UpdateError::Zip(err.to_string()))?;
    let entry_count = archive.len();
    if entry_count > limits.max_entries {
        return Err(UpdateError::Invalid(format!(
            "Archive has {entry_count} entries, limit is {}",
            limits.max_entries
        )));
    }
    let mut total_uncompressed: u64 = 0;
    for i in 0..entry_count {
        let mut entry = archive
            .by_index(i)
            .map_err(|err| UpdateError::Zip(err.to_string()))?;
        let uncompressed_size = entry.size();
        if uncompressed_size > limits.max_entry_uncompressed_bytes {
            return Err(UpdateError::Invalid(format!(
                "Archive entry '{}' is too large ({} bytes, limit {})",
                entry.name(),
                uncompressed_size,
                limits.max_entry_uncompressed_bytes
            )));
        }
        if uncompressed_size > 0 {
            let compressed_size = entry.compressed_size();
            if compressed_size == 0 {
                return Err(UpdateError::Invalid(format!(
                    "Archive entry '{}' has zero compressed size",
                    entry.name()
                )));
            }
            let max_uncompressed = compressed_size.saturating_mul(limits.max_compression_ratio);
            if uncompressed_size > max_uncompressed {
                return Err(UpdateError::Invalid(format!(
                    "Archive entry '{}' exceeds compression ratio limit",
                    entry.name()
                )));
            }
        }
        total_uncompressed = total_uncompressed
            .checked_add(uncompressed_size)
            .ok_or_else(|| UpdateError::Invalid("Archive size overflow".into()))?;
        if total_uncompressed > limits.max_total_uncompressed_bytes {
            return Err(UpdateError::Invalid(format!(
                "Archive extracted size {} exceeds limit {}",
                total_uncompressed, limits.max_total_uncompressed_bytes
            )));
        }
        let outpath = match entry.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };
        if entry.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
            continue;
        }
        if let Some(parent) = outpath.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut outfile = File::create(&outpath)?;
        std::io::copy(&mut entry, &mut outfile)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                std::fs::set_permissions(
                    &outpath,
                    std::fs::Permissions::from_mode(safe_unix_file_mode(mode)),
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) -> Result<(), UpdateError> {
        let file = File::create(path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            zip.start_file(name, options)
                .map_err(|err| UpdateError::Zip(err.to_string()))?;
            zip.write_all(data)?;
        }
        zip.finish()
            .map_err(|err| UpdateError::Zip(err.to_string()))?;
        Ok(())
    }

    #[cfg(unix)]
    fn write_zip_with_modes(
        path: &Path,
        entries: &[(&str, &[u8], u32)],
    ) -> Result<(), UpdateError> {
        let file = File::create(path)?;
        let mut zip = zip::ZipWriter::new(file);
        for (name, data, mode) in entries {
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(*mode);
            zip.start_file(name, options)
                .map_err(|err| UpdateError::Zip(err.to_string()))?;
            zip.write_all(data)?;
        }
        zip.finish()
            .map_err(|err| UpdateError::Zip(err.to_string()))?;
        Ok(())
    }

    #[test]
    fn unzip_rejects_entry_over_size_limit() {
        let temp = tempdir().expect("tempdir");
        let zip_path = temp.path().join("oversize.zip");
        write_zip(&zip_path, &[("big.bin", &[1u8; 8])]).expect("zip write");
        let limits = ZipExtractionLimits {
            max_entries: 10,
            max_entry_uncompressed_bytes: 4,
            max_total_uncompressed_bytes: 100,
            max_compression_ratio: 100,
        };
        let err = unzip_to_dir_with_limits(&zip_path, temp.path().join("out").as_path(), limits)
            .expect_err("oversized entry must fail");
        assert!(err.to_string().contains("too large"));
    }

    #[test]
    fn unzip_rejects_total_uncompressed_over_limit() {
        let temp = tempdir().expect("tempdir");
        let zip_path = temp.path().join("total.zip");
        write_zip(&zip_path, &[("a.txt", &[1u8; 6]), ("b.txt", &[2u8; 6])]).expect("zip write");
        let limits = ZipExtractionLimits {
            max_entries: 10,
            max_entry_uncompressed_bytes: 10,
            max_total_uncompressed_bytes: 10,
            max_compression_ratio: 100,
        };
        let err = unzip_to_dir_with_limits(&zip_path, temp.path().join("out").as_path(), limits)
            .expect_err("total uncompressed limit must fail");
        assert!(err.to_string().contains("exceeds limit"));
    }

    #[test]
    fn unzip_rejects_suspicious_compression_ratio() {
        let temp = tempdir().expect("tempdir");
        let zip_path = temp.path().join("ratio.zip");
        let data = vec![b'a'; 2048];
        write_zip(&zip_path, &[("dense.txt", data.as_slice())]).expect("zip write");
        let limits = ZipExtractionLimits {
            max_entries: 10,
            max_entry_uncompressed_bytes: 10_000,
            max_total_uncompressed_bytes: 10_000,
            max_compression_ratio: 2,
        };
        let err = unzip_to_dir_with_limits(&zip_path, temp.path().join("out").as_path(), limits)
            .expect_err("compression ratio must fail");
        assert!(err.to_string().contains("compression ratio"));
    }

    #[cfg(unix)]
    #[test]
    fn unzip_strips_unsafe_permissions() {
        let temp = tempdir().expect("tempdir");
        let zip_path = temp.path().join("perms.zip");
        write_zip_with_modes(
            &zip_path,
            &[
                ("bin/tool", b"run", 0o7777),
                ("data/config.toml", b"cfg", 0o6666),
            ],
        )
        .expect("zip write");
        unzip_to_dir(&zip_path, temp.path().join("out").as_path()).expect("unzip");
        let exec_mode = std::fs::metadata(temp.path().join("out/bin/tool"))
            .expect("exec metadata")
            .permissions()
            .mode()
            & 0o7777;
        let data_mode = std::fs::metadata(temp.path().join("out/data/config.toml"))
            .expect("data metadata")
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(exec_mode, 0o755);
        assert_eq!(data_mode, 0o644);
    }
}
