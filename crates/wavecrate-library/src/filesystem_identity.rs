//! Stable filesystem-object identity helpers.

use std::{fs, path::Path};

/// Return a stable platform identity for the filesystem object described by `metadata`.
///
/// Callers should obtain `metadata` with either [`fs::metadata`] or
/// [`fs::symlink_metadata`] according to whether symlinks should be followed. The helper
/// preserves that choice when opening the object on Windows. Identity lookup is best-effort;
/// unsupported platforms and filesystem or permission failures return `None`.
pub fn stable_filesystem_identity(path: &Path, metadata: &fs::Metadata) -> Option<String> {
    stable_filesystem_identity_impl(path, metadata)
}

#[cfg(unix)]
fn stable_filesystem_identity_impl(_path: &Path, metadata: &fs::Metadata) -> Option<String> {
    use std::os::unix::fs::MetadataExt;

    Some(format!("unix:{}:{}", metadata.dev(), metadata.ino()))
}

#[cfg(windows)]
fn stable_filesystem_identity_impl(path: &Path, metadata: &fs::Metadata) -> Option<String> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle,
        },
    };

    let mut options = fs::OpenOptions::new();
    options
        .access_mode(FILE_READ_ATTRIBUTES.0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0);
    if metadata.file_type().is_symlink() {
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0);
    }

    let file = options.open(path).ok()?;
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut information) }.ok()?;
    let file_index =
        (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow);
    Some(format!(
        "windows:{}:{}",
        information.dwVolumeSerialNumber, file_index
    ))
}

#[cfg(not(any(unix, windows)))]
fn stable_filesystem_identity_impl(_path: &Path, _metadata: &fs::Metadata) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_links_share_identity_but_distinct_files_do_not() {
        let temp = tempfile::tempdir().expect("create identity fixture");
        let original = temp.path().join("original.wav");
        let linked = temp.path().join("linked.wav");
        let distinct = temp.path().join("distinct.wav");
        fs::write(&original, b"original").expect("write original fixture");
        fs::hard_link(&original, &linked).expect("create hard-link fixture");
        fs::write(&distinct, b"distinct").expect("write distinct fixture");

        let identity = |path: &Path| {
            let metadata = fs::symlink_metadata(path).expect("read fixture metadata");
            stable_filesystem_identity(path, &metadata).expect("read fixture identity")
        };

        assert_eq!(identity(&original), identity(&linked));
        assert_ne!(identity(&original), identity(&distinct));
    }
}
