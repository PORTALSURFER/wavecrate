//! Journal-backed staging helpers for folder-delete crash recovery.
//!
//! The on-disk journal records the delete lifecycle as
//! `Intent -> Staged -> Deleted -> RestorePendingDb`.
//! Recovery uses that contract to decide whether a folder should be restored back into the
//! source tree after a crash or retained inside the app-owned trash area.

mod atomic_save;
mod model;
mod staging;
mod store;

pub(crate) use model::{DeleteJournal, DeleteJournalEntry};
pub(crate) use model::{DeleteJournalStage, DeleteStagingInfo};
pub(crate) use staging::{
    cleanup_staging_root, mark_delete_restore_pending_db, mark_delete_retained,
    purge_deleted_folder, remove_delete_entry, restage_deleted_folder, restore_deleted_folder,
    rollback_staged_folder, stage_folder_for_delete,
};
pub(crate) use store::{load_journal, remove_entry};

#[cfg(test)]
pub(crate) use atomic_save::fail_next_save_before_replace_for_tests;
#[cfg(test)]
pub(crate) use store::update_entry_stage;

#[cfg(test)]
mod tests;
