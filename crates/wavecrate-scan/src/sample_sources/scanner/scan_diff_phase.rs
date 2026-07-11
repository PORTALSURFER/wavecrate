use std::path::Path;
use std::sync::atomic::AtomicBool;

use super::scan::{ScanContext, ScanError};
use super::scan_diff::{PreparedFile, should_compute_full_hash};
use super::scan_fs::{compute_content_hash, read_facts};

pub(super) fn prepare_diff(
    root: &Path,
    path: &Path,
    context: &ScanContext,
    cancel: Option<&AtomicBool>,
) -> Result<PreparedFile, ScanError> {
    let facts = read_facts(root, path)?;
    let needs_hash = should_compute_full_hash(context.mode, facts.size)
        && context.existing.get(&facts.relative).is_none_or(|entry| {
            entry.file_size != facts.size
                || entry.modified_ns != facts.modified_ns
                || entry.content_hash.is_none()
        });
    let content_hash = if needs_hash {
        Some(compute_content_hash(path, cancel)?)
    } else {
        None
    };
    Ok(PreparedFile {
        facts,
        content_hash,
    })
}
