//! Crash recovery support for staged folder deletes.
//!
//! This module keeps three responsibilities separate:
//! - `journal`: stage/journal persistence and rollback helpers used by delete flows
//! - `recovery`: startup reconciliation that decides whether to restore or retain deletes
//! - `controller_apply`: UI/cache application of recovery reports once work finishes
//! - `retained_restore`: explicit restore/purge flows for retained deletes
//! - `restore_merge`: conflict-aware merge policy for explicit retained restores

mod controller_apply;
mod journal;
mod recovery;
mod restore_merge;
mod retained_resolution;
mod retained_restore;

/// Folder name used to stage pending deletes inside a source root.
pub(crate) const DELETE_STAGING_DIR: &str = ".sempal_delete_staging";

pub(crate) use journal::{
    DeleteStagingInfo, cleanup_staging_root, mark_delete_retained, purge_deleted_folder,
    restage_deleted_folder, restore_deleted_folder, rollback_staged_folder,
    stage_folder_for_delete,
};
pub(crate) use recovery::{
    DeleteRecoveryAction, DeleteRecoveryEntry, DeleteRecoveryReport, DeleteRecoveryStatus,
    RetainedDeleteEntry, recover_staged_deletes,
};
pub(crate) use retained_resolution::run_retained_delete_resolution_job;
