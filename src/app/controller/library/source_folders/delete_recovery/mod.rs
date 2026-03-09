//! Crash recovery support for staged folder deletes.
//!
//! This module keeps three responsibilities separate:
//! - `journal`: stage/journal persistence and rollback helpers used by delete flows
//! - `recovery`: startup reconciliation that decides whether to restore or finalize
//! - `controller_apply`: UI/cache application of recovery reports once work finishes

mod controller_apply;
mod journal;
mod recovery;

/// Folder name used to stage pending deletes inside a source root.
pub(crate) const DELETE_STAGING_DIR: &str = ".sempal_delete_staging";

pub(crate) use journal::{
    cleanup_staging_root, mark_delete_db_committed, remove_delete_entry, rollback_staged_folder,
    stage_folder_for_delete,
};
pub(crate) use recovery::{
    DeleteRecoveryAction, DeleteRecoveryEntry, DeleteRecoveryReport, DeleteRecoveryStatus,
    recover_staged_deletes,
};
