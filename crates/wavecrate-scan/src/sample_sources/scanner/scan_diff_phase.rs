use super::scan::{ScanContext, ScanError};
use super::scan_diff::{PreparedFile, should_compute_full_hash};
use super::scan_fs::read_facts;
use std::path::Path;

pub(super) fn prepare_diff(
    root: &Path,
    path: &Path,
    context: &ScanContext,
) -> Result<PreparedFile, ScanError> {
    let facts = read_facts(root, path)?;
    let hash_required = should_compute_full_hash(context.mode, facts.size);
    let needs_hash = hash_required
        && context.existing.get(&facts.relative).is_none_or(|entry| {
            entry.file_size != facts.size
                || entry.modified_ns != facts.modified_ns
                || entry.content_hash.is_none()
        });
    let requires_apply = context.existing.get(&facts.relative).is_none_or(|entry| {
        entry.file_size != facts.size
            || entry.modified_ns != facts.modified_ns
            || entry.missing
            || (entry.content_hash.is_none() && needs_hash)
    });
    Ok(PreparedFile {
        facts,
        hash_required,
        needs_hash,
        requires_apply,
        content_hash: None,
    })
}
