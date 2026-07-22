mod dialog;
mod filesystem_refresh;
mod filesystem_refresh_worker;
mod maintenance;
mod progress;
mod rating_decay_worker;
mod source_commands;
mod task_ids;
mod worker;

#[cfg(test)]
pub(in crate::native_app) use filesystem_refresh::{
    FILESYSTEM_SYNC_PREP_INTENTS, FILESYSTEM_SYNC_PREP_REASON,
};
pub(in crate::native_app) use filesystem_refresh_worker::sync_source_database_paths;
pub(in crate::native_app) use maintenance::FolderScanMaintenanceResult;
#[cfg(test)]
pub(in crate::native_app) use source_commands::{
    PROCESS_SOURCE_PREP_INTENTS, PROCESS_SOURCE_PREP_REASON,
};
#[cfg(test)]
pub(in crate::native_app) use worker::{
    SOURCE_SCAN_COMPLETION_PREP_INTENTS, SOURCE_SCAN_COMPLETION_PREP_REASON,
};
