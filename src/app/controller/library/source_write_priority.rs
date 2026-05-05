//! Source-local write-priority windows for browser-owned file operations.

use crate::sample_sources::SourceId;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static ACTIVE_FILE_OPS: OnceLock<Mutex<HashMap<SourceId, usize>>> = OnceLock::new();
static COMPLETED_RENAME_REMAPS: OnceLock<Mutex<CompletedRenameRemaps>> = OnceLock::new();
const MAX_COMPLETED_RENAME_REMAPS: usize = 512;

#[derive(Debug, Default)]
struct CompletedRenameRemaps {
    entries: HashMap<(SourceId, PathBuf), PathBuf>,
    order: VecDeque<(SourceId, PathBuf)>,
}

fn active_file_ops() -> &'static Mutex<HashMap<SourceId, usize>> {
    ACTIVE_FILE_OPS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn completed_rename_remaps() -> &'static Mutex<CompletedRenameRemaps> {
    COMPLETED_RENAME_REMAPS.get_or_init(|| Mutex::new(CompletedRenameRemaps::default()))
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

/// Return the source ids currently owning file-op write priority.
pub(crate) fn active_file_op_write_priority_sources() -> HashSet<SourceId> {
    active_file_ops()
        .lock()
        .expect("source file-op write-priority mutex poisoned")
        .keys()
        .cloned()
        .collect()
}

/// Remember one successful browser rename so stale queued metadata can follow
/// the path-derived sample identity remap instead of reading the old path.
pub(crate) fn record_completed_browser_rename(
    source_id: &SourceId,
    old_relative: &Path,
    new_relative: &Path,
) {
    if old_relative == new_relative {
        return;
    }
    let key = (source_id.clone(), old_relative.to_path_buf());
    let mut remaps = completed_rename_remaps()
        .lock()
        .expect("completed browser rename remap mutex poisoned");
    if !remaps.entries.contains_key(&key) {
        remaps.order.push_back(key.clone());
    }
    remaps.entries.insert(key, new_relative.to_path_buf());
    while remaps.order.len() > MAX_COMPLETED_RENAME_REMAPS {
        if let Some(expired) = remaps.order.pop_front() {
            remaps.entries.remove(&expired);
        }
    }
}

/// Return the new path for a recently completed browser rename.
pub(crate) fn completed_browser_rename_target(
    source_id: &SourceId,
    old_relative: &Path,
) -> Option<PathBuf> {
    completed_rename_remaps()
        .lock()
        .expect("completed browser rename remap mutex poisoned")
        .entries
        .get(&(source_id.clone(), old_relative.to_path_buf()))
        .cloned()
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
