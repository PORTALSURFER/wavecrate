use super::{IssueTokenStoreError, crypto};
use std::path::Path;

/// Write a file with restricted permissions using an atomic swap on supported platforms.
pub(super) fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), IssueTokenStoreError> {
    use std::io::Write;
    let dir = path.parent().ok_or_else(|| {
        IssueTokenStoreError::Io(std::io::Error::other("token path has no parent directory"))
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        IssueTokenStoreError::Io(std::io::Error::other("token path has no file name"))
    })?;

    let mut last_err = None;
    for _ in 0..5 {
        let suffix = random_hex(6)?;
        let tmp_path = dir.join(format!("{}.tmp-{}", file_name.to_string_lossy(), suffix));
        let mut open_options = std::fs::OpenOptions::new();
        open_options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            open_options.mode(0o600);
        }
        match open_options.open(&tmp_path) {
            Ok(mut file) => {
                file.write_all(bytes)?;
                file.sync_all()?;
                drop(file);
                replace_file(&tmp_path, path)?;
                #[cfg(target_os = "windows")]
                {
                    harden_windows_permissions(path);
                }
                sync_parent_dir(dir)?;
                return Ok(());
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                last_err = Some(err);
                continue;
            }
            Err(err) => return Err(err.into()),
        }
    }

    Err(IssueTokenStoreError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        format!(
            "failed to create temporary file for {}: {}",
            path.display(),
            last_err
                .as_ref()
                .map(|err| err.to_string())
                .unwrap_or_else(|| "unknown error".into())
        ),
    )))
}

fn replace_file(temp_path: &Path, path: &Path) -> Result<(), IssueTokenStoreError> {
    match std::fs::rename(temp_path, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            #[cfg(target_os = "windows")]
            if err.kind() == std::io::ErrorKind::AlreadyExists
                || err.kind() == std::io::ErrorKind::PermissionDenied
            {
                clear_windows_readonly(path);
                if let Err(e) = std::fs::remove_file(path)
                    && e.kind() != std::io::ErrorKind::NotFound
                {
                    return Err(e.into());
                }
                std::fs::rename(temp_path, path)?;
                return Ok(());
            }
            Err(err.into())
        }
    }
}

fn sync_parent_dir(dir: &Path) -> Result<(), IssueTokenStoreError> {
    #[cfg(unix)]
    {
        let dir_handle = std::fs::File::open(dir)?;
        dir_handle.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
    }
    Ok(())
}

fn random_hex(len: usize) -> Result<String, IssueTokenStoreError> {
    let bytes = crypto::random_bytes(len)?;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut out, "{:02x}", byte).expect("writing to String should not fail");
    }
    Ok(out)
}

#[cfg(target_os = "windows")]
/// Apply best-effort hiding/readonly attributes for the fallback token file.
/// This is not equivalent to ACLs but avoids a visible plaintext file.
fn harden_windows_permissions(path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY, SetFileAttributesW,
        },
        core::PCWSTR,
    };
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let _ = unsafe {
        SetFileAttributesW(
            PCWSTR(wide.as_ptr()),
            FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_READONLY,
        )
    };
}

#[cfg(target_os = "windows")]
/// Clear readonly attributes so the fallback token file can be replaced.
pub(super) fn clear_windows_readonly(path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::Storage::FileSystem::{FILE_ATTRIBUTE_NORMAL, SetFileAttributesW},
        core::PCWSTR,
    };
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let _ = unsafe { SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_ATTRIBUTE_NORMAL) };
}
