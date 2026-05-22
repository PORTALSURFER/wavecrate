use super::*;

#[cfg(test)]
use std::sync::{LazyLock, Mutex};

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::app::controller::library::browser_controller::helpers) struct RenameLoopedProvenanceLog
{
    pub(in crate::app::controller::library::browser_controller::helpers) old_relative: PathBuf,
    pub(in crate::app::controller::library::browser_controller::helpers) new_relative: PathBuf,
    pub(in crate::app::controller::library::browser_controller::helpers) request_looped: bool,
    pub(in crate::app::controller::library::browser_controller::helpers) db_looped: Option<bool>,
    pub(in crate::app::controller::library::browser_controller::helpers) final_looped: bool,
}

#[cfg(test)]
static RENAME_LOOPED_PROVENANCE_LOGS: LazyLock<Mutex<Vec<RenameLoopedProvenanceLog>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[cfg(test)]
pub(in crate::app::controller::library::browser_controller::helpers) fn take_rename_looped_provenance_logs_for_tests()
-> Vec<RenameLoopedProvenanceLog> {
    std::mem::take(&mut *RENAME_LOOPED_PROVENANCE_LOGS.lock().unwrap())
}

#[cfg(test)]
pub(super) fn record_rename_looped_provenance(
    old_relative: &Path,
    new_relative: &Path,
    request_looped: bool,
    db_looped: Option<bool>,
    final_looped: bool,
) {
    RENAME_LOOPED_PROVENANCE_LOGS
        .lock()
        .unwrap()
        .push(RenameLoopedProvenanceLog {
            old_relative: old_relative.to_path_buf(),
            new_relative: new_relative.to_path_buf(),
            request_looped,
            db_looped,
            final_looped,
        });
}

#[cfg(not(test))]
pub(super) fn record_rename_looped_provenance(
    _old_relative: &Path,
    _new_relative: &Path,
    _request_looped: bool,
    _db_looped: Option<bool>,
    _final_looped: bool,
) {
}
