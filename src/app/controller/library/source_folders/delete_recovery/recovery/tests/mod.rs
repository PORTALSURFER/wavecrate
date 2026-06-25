mod journal_handling;
mod metadata_replay;
mod restore_paths;
mod retained_cleanup;
mod staged_state;

use super::*;
use crate::app::controller::library::source_folders::delete_recovery::restore_merge::restore_retained_folder_with_merge_with_stamp;
use crate::app::controller::library::source_folders::delete_recovery::{
    DeleteRecoveryAction, DeleteRecoveryStatus, mark_delete_restore_pending_db,
    mark_delete_retained, stage_folder_for_delete,
};
use crate::sample_sources::SampleSource;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

use super::super::journal::{DeleteJournalStage, update_entry_stage};

fn sample_source() -> (tempfile::TempDir, SampleSource) {
    let dir = tempdir().unwrap();
    let root = dir.path().join("source");
    fs::create_dir_all(&root).unwrap();
    (dir, SampleSource::new(root))
}

fn sample_entry(relative_path: &str) -> crate::sample_sources::WavEntry {
    crate::sample_sources::WavEntry {
        relative_path: relative_path.into(),
        file_size: 1024,
        modified_ns: 123,
        content_hash: Some("abc123".into()),
        tag: crate::sample_sources::Rating::KEEP_1,
        looped: true,
        sound_type: None,
        locked: true,
        missing: false,
        last_played_at: Some(456),
        last_curated_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }
}

fn assert_recovery(
    entry: &DeleteRecoveryEntry,
    action: DeleteRecoveryAction,
    status: DeleteRecoveryStatus,
) {
    assert_eq!(entry.action, action);
    assert_eq!(entry.status, status);
}
