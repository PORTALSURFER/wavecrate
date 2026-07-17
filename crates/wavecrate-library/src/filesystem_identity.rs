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

/// Return whether two persisted identities describe the same filesystem object.
///
/// Version 2 added creation time so reused inode/file-index values no longer alias a
/// replacement. During migration, a legacy identity can still be matched to its v2 form
/// by the platform object fields that were available in v1.
pub fn same_filesystem_object_identity(previous: &str, current: &str) -> bool {
    if previous == current {
        return true;
    }
    legacy_identity_parts(previous)
        .zip(version_2_identity_parts(current))
        .is_some_and(|(previous, current)| previous == current)
        || version_2_identity_parts(previous)
            .zip(legacy_identity_parts(current))
            .is_some_and(|(previous, current)| previous == current)
}

fn legacy_identity_parts(identity: &str) -> Option<(&str, &str, &str)> {
    let mut parts = identity.split(':');
    let platform = parts.next()?;
    if !matches!(platform, "unix" | "windows") {
        return None;
    }
    let first = parts.next()?;
    let second = parts.next()?;
    parts.next().is_none().then_some((platform, first, second))
}

fn version_2_identity_parts(identity: &str) -> Option<(&str, &str, &str)> {
    let mut parts = identity.split(':');
    let platform = match parts.next()? {
        "unix-v2" => "unix",
        "windows-v2" => "windows",
        _ => return None,
    };
    let first = parts.next()?;
    let second = parts.next()?;
    let _creation_time = parts.next()?;
    parts.next().is_none().then_some((platform, first, second))
}

#[cfg(unix)]
fn stable_filesystem_identity_impl(_path: &Path, metadata: &fs::Metadata) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    use std::time::UNIX_EPOCH;

    // Device and inode alone are not sufficient: filesystems may immediately reuse an
    // inode after deletion. The creation timestamp remains stable across a rename and
    // distinguishes a replacement that inherits the same device/inode pair.
    let created = metadata.created().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(format!(
        "unix-v2:{}:{}:{}",
        metadata.dev(),
        metadata.ino(),
        created.as_nanos()
    ))
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
    let creation_time = (u64::from(information.ftCreationTime.dwHighDateTime) << 32)
        | u64::from(information.ftCreationTime.dwLowDateTime);
    Some(format!(
        "windows-v2:{}:{}:{}",
        information.dwVolumeSerialNumber, file_index, creation_time
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

    #[test]
    fn rename_preserves_identity() {
        let temp = tempfile::tempdir().expect("create identity fixture");
        let original = temp.path().join("original.wav");
        let renamed = temp.path().join("renamed.wav");
        fs::write(&original, b"original").expect("write original fixture");

        let original_metadata = fs::symlink_metadata(&original).expect("read original metadata");
        let original_identity = stable_filesystem_identity(&original, &original_metadata)
            .expect("read original identity");
        fs::rename(&original, &renamed).expect("rename fixture");
        let renamed_metadata = fs::symlink_metadata(&renamed).expect("read renamed metadata");
        let renamed_identity =
            stable_filesystem_identity(&renamed, &renamed_metadata).expect("read renamed identity");

        assert_eq!(original_identity, renamed_identity);
    }

    #[test]
    fn legacy_identity_matches_its_version_2_upgrade_only() {
        assert!(same_filesystem_object_identity(
            "unix:10:20",
            "unix-v2:10:20:30"
        ));
        assert!(same_filesystem_object_identity(
            "windows-v2:10:20:30",
            "windows:10:20"
        ));
        assert!(!same_filesystem_object_identity(
            "unix-v2:10:20:30",
            "unix-v2:10:20:31"
        ));
        assert!(!same_filesystem_object_identity(
            "unix:10:20",
            "windows-v2:10:20:30"
        ));
    }
}
