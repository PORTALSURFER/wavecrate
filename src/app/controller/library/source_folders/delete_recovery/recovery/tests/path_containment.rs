use super::*;
#[cfg(unix)]
use crate::app::controller::library::source_folders::delete_recovery::DeleteStagingInfo;
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;

#[test]
fn recover_rejects_parent_dir_original_without_restoring_outside_source() {
    let (temp, source) = sample_source();
    let outside = temp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    fs::create_dir_all(staging_root.join("gone")).unwrap();
    write_journal(
        &staging_root,
        "evil-original",
        "../outside",
        "gone",
        "staged",
    );

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert_eq!(report.entries.len(), 1);
    assert_recovery(
        &report.entries[0],
        DeleteRecoveryAction::Restore,
        DeleteRecoveryStatus::Failed,
    );
    assert!(
        report.entries[0]
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("Invalid delete journal entry"))
    );
    assert!(outside.is_dir());
    assert!(staging_root.join("gone").is_dir());
}

#[test]
fn recover_rejects_windows_prefixed_journal_path_without_restoring() {
    let (_temp, source) = sample_source();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    fs::create_dir_all(staging_root.join("gone")).unwrap();
    write_journal(
        &staging_root,
        "evil-windows",
        "C:/outside",
        "gone",
        "staged",
    );

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert_eq!(report.entries.len(), 1);
    assert_recovery(
        &report.entries[0],
        DeleteRecoveryAction::Restore,
        DeleteRecoveryStatus::Failed,
    );
    assert!(staging_root.join("gone").is_dir());
}

#[test]
fn recover_rejects_parent_dir_staged_path_without_touching_outside_staging() {
    let (temp, source) = sample_source();
    let outside_staged = temp.path().join("outside-staged");
    fs::create_dir_all(&outside_staged).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    fs::create_dir_all(&staging_root).unwrap();
    write_journal(
        &staging_root,
        "evil-staged",
        "gone",
        "../outside-staged",
        "staged",
    );

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert_eq!(report.entries.len(), 1);
    assert_recovery(
        &report.entries[0],
        DeleteRecoveryAction::Restore,
        DeleteRecoveryStatus::Failed,
    );
    assert!(outside_staged.is_dir());
    assert!(!source.root.join("gone").exists());
}

#[cfg(unix)]
#[test]
fn recover_rejects_symlinked_staged_folder_without_moving_target() {
    let (temp, source) = sample_source();
    let outside = temp.path().join("outside-target");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("keep.txt"), b"outside").unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    fs::create_dir_all(&staging_root).unwrap();
    std::os::unix::fs::symlink(&outside, staging_root.join("gone")).unwrap();
    write_journal(&staging_root, "evil-link", "gone", "gone", "staged");

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert_eq!(report.entries.len(), 1);
    assert_recovery(
        &report.entries[0],
        DeleteRecoveryAction::Restore,
        DeleteRecoveryStatus::Failed,
    );
    assert_eq!(fs::read(outside.join("keep.txt")).unwrap(), b"outside");
    assert!(!source.root.join("gone").exists());
}

#[cfg(unix)]
#[test]
fn purge_deleted_folder_rejects_symlinked_staged_folder_without_deleting_target() {
    let (temp, source) = sample_source();
    let outside = temp.path().join("outside-purge");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("keep.txt"), b"outside").unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    fs::create_dir_all(&staging_root).unwrap();
    let staged_link = staging_root.join("gone");
    std::os::unix::fs::symlink(&outside, &staged_link).unwrap();
    let info = DeleteStagingInfo {
        id: "evil-purge".into(),
        original_relative: PathBuf::from("gone"),
        staged_relative: PathBuf::from("gone"),
        staged_absolute: staged_link,
    };

    let err = purge_deleted_folder(&info, &staging_root).unwrap_err();

    assert!(err.contains("symlink"));
    assert_eq!(fs::read(outside.join("keep.txt")).unwrap(), b"outside");
}

#[cfg(unix)]
#[test]
fn restore_deleted_folder_rejects_symlinked_source_parent_without_moving_staged_folder() {
    let (temp, source) = sample_source();
    let outside = temp.path().join("outside-restore");
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, source.root.join("link-parent")).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged_absolute = staging_root.join("gone");
    fs::create_dir_all(&staged_absolute).unwrap();
    let info = DeleteStagingInfo {
        id: "evil-restore".into(),
        original_relative: PathBuf::from("link-parent/gone"),
        staged_relative: PathBuf::from("gone"),
        staged_absolute: staged_absolute.clone(),
    };

    let err = restore_deleted_folder(&info, &source.root.join("link-parent/gone"), &staging_root)
        .unwrap_err();

    assert!(err.contains("symlink") || err.contains("escapes"));
    assert!(staged_absolute.is_dir());
    assert!(!outside.join("gone").exists());
}

#[cfg(unix)]
#[test]
fn rollback_staged_folder_rejects_symlinked_source_parent_without_moving_staged_folder() {
    let (temp, source) = sample_source();
    let outside = temp.path().join("outside-rollback");
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, source.root.join("link-parent")).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged_absolute = staging_root.join("gone");
    fs::create_dir_all(&staged_absolute).unwrap();
    let info = DeleteStagingInfo {
        id: "evil-rollback".into(),
        original_relative: PathBuf::from("link-parent/gone"),
        staged_relative: PathBuf::from("gone"),
        staged_absolute: staged_absolute.clone(),
    };

    let err = rollback_staged_folder(
        &info,
        &source.root.join("link-parent/gone"),
        &staging_root,
        "rollback",
    )
    .unwrap_err();

    assert!(err.contains("symlink") || err.contains("escapes"));
    assert!(staged_absolute.is_dir());
    assert!(!outside.join("gone").exists());
}

fn write_journal(
    staging_root: &Path,
    id: &str,
    original_relative: &str,
    staged_relative: &str,
    stage: &str,
) {
    fs::create_dir_all(staging_root).unwrap();
    let json = format!(
        r#"{{
  "entries": [
    {{
      "id": "{id}",
      "original_relative": "{original_relative}",
      "staged_relative": "{staged_relative}",
      "deleted_entries": [],
      "stage": "{stage}",
      "created_at": 1
    }}
  ]
}}"#
    );
    fs::write(staging_root.join("delete_journal.json"), json).unwrap();
}
