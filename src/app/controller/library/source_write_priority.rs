//! Source-local write-priority windows for browser-owned file operations.

use crate::sample_sources::SourceId;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static ACTIVE_FILE_OPS: OnceLock<Mutex<HashMap<SourceId, usize>>> = OnceLock::new();

fn active_file_ops() -> &'static Mutex<HashMap<SourceId, usize>> {
    ACTIVE_FILE_OPS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Mark `source_id` as owning a short file-op write-priority window.
pub(crate) fn begin_file_op_write_priority(source_id: &SourceId) {
    let mut active = active_file_ops()
        .lock()
        .expect("source file-op write-priority mutex poisoned");
    *active.entry(source_id.clone()).or_insert(0) += 1;
}

/// Clear one active file-op write-priority window for `source_id`.
pub(crate) fn finish_file_op_write_priority(source_id: &SourceId) {
    let mut active = active_file_ops()
        .lock()
        .expect("source file-op write-priority mutex poisoned");
    let Some(count) = active.get_mut(source_id) else {
        return;
    };
    *count = count.saturating_sub(1);
    if *count == 0 {
        active.remove(source_id);
    }
}

/// Return true when same-source maintenance writes should defer.
pub(crate) fn file_op_write_priority_active(source_id: &SourceId) -> bool {
    active_file_ops()
        .lock()
        .expect("source file-op write-priority mutex poisoned")
        .contains_key(source_id)
}

#[cfg(test)]
pub(crate) struct FileOpWritePriorityGuard {
    source_id: SourceId,
}

#[cfg(test)]
impl FileOpWritePriorityGuard {
    pub(crate) fn new(source_id: &SourceId) -> Self {
        begin_file_op_write_priority(source_id);
        Self {
            source_id: source_id.clone(),
        }
    }
}

#[cfg(test)]
impl Drop for FileOpWritePriorityGuard {
    fn drop(&mut self) {
        finish_file_op_write_priority(&self.source_id);
    }
}
