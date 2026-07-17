use super::scan::{ScanContext, ScanError, ScanMode};
use super::scan_diff::{PreparedFile, should_compute_full_hash};
use super::scan_fs::read_facts;
use std::path::Path;
use wavecrate_library::filesystem_identity::same_filesystem_object_identity;

pub(super) fn prepare_diff(
    root: &Path,
    path: &Path,
    context: &ScanContext,
) -> Result<PreparedFile, ScanError> {
    let facts = read_facts(root, path)?;
    let existing = context.existing.get(&facts.relative);
    let persisted_identity = context
        .committed_file_identity(&facts.relative)
        .map(str::trim)
        .filter(|identity| !identity.is_empty());
    let observed_identity = facts
        .file_identity
        .as_deref()
        .map(str::trim)
        .filter(|identity| !identity.is_empty());
    let identity_requires_update =
        observed_identity.is_some_and(|identity| persisted_identity != Some(identity));
    let identity_replaced = persisted_identity
        .zip(observed_identity)
        .is_some_and(|(previous, current)| !same_filesystem_object_identity(previous, current));
    let force_targeted_verification = context.mode == ScanMode::Targeted;
    let hash_required = should_compute_full_hash(context.mode, facts.size)
        || (force_targeted_verification && existing.is_some());
    let needs_hash = hash_required
        && existing.is_none_or(|entry| {
            force_targeted_verification
                || entry.file_size != facts.size
                || entry.modified_ns != facts.modified_ns
                || entry.content_hash.is_none()
                || identity_replaced
        });
    let requires_apply = existing.is_none_or(|entry| {
        force_targeted_verification
            || entry.file_size != facts.size
            || entry.modified_ns != facts.modified_ns
            || entry.missing
            || (entry.content_hash.is_none() && needs_hash)
            || identity_requires_update
    });
    Ok(PreparedFile {
        facts,
        hash_required,
        needs_hash,
        requires_apply,
        identity_replaced,
        content_hash: None,
    })
}
