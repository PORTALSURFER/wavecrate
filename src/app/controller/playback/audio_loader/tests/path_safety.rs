use super::*;

#[test]
fn ensure_safe_relative_path_rejects_parent_dir() {
    let err =
        super::super::stages::ensure_safe_relative_path(Path::new("../escape.wav")).unwrap_err();
    assert!(matches!(err, super::super::AudioLoadError::Failed(_)));
}

#[test]
fn ensure_safe_relative_path_rejects_rooted_path() {
    let err =
        super::super::stages::ensure_safe_relative_path(Path::new("/escape.wav")).unwrap_err();
    assert!(matches!(err, super::super::AudioLoadError::Failed(_)));
}

#[cfg(windows)]
#[test]
fn ensure_safe_relative_path_rejects_windows_drive_prefix() {
    let err =
        super::super::stages::ensure_safe_relative_path(Path::new(r"C:\escape.wav")).unwrap_err();
    assert!(matches!(err, super::super::AudioLoadError::Failed(_)));
}

#[cfg(windows)]
#[test]
fn ensure_safe_relative_path_rejects_windows_rooted_path() {
    let err =
        super::super::stages::ensure_safe_relative_path(Path::new(r"\escape.wav")).unwrap_err();
    assert!(matches!(err, super::super::AudioLoadError::Failed(_)));
}

#[test]
fn ensure_safe_relative_path_accepts_normal_relative_paths() {
    super::super::stages::ensure_safe_relative_path(Path::new("folder/./file.wav")).unwrap();
}
