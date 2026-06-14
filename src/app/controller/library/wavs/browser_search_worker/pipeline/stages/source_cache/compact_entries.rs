use super::super::super::*;
use super::cache_invalidation::{
    clear_derived_search_caches, clear_path_dependent_scores_if_changed,
};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub(super) fn reload_compact_entries(
    cache: &mut SearchWorkerCache,
    rows: &[crate::sample_sources::db::read::SearchEntryRow],
    queue: &SearchJobQueue,
    generation: u64,
    revision: u64,
    paths_revision: u64,
) -> bool {
    let Some((compact_entries, entry_lookup, path_fingerprint)) =
        build_compact_entries(rows, queue, generation)
    else {
        return false;
    };
    cache.entries = Some(compact_entries);
    cache.entry_lookup = entry_lookup;
    cache.revision = revision;
    cache.paths_revision = paths_revision;
    clear_path_dependent_scores_if_changed(cache, path_fingerprint);
    clear_derived_search_caches(cache);
    true
}

fn build_compact_entries(
    loaded_entries: &[crate::sample_sources::db::read::SearchEntryRow],
    queue: &SearchJobQueue,
    generation: u64,
) -> Option<(
    Vec<CompactSearchEntry>,
    std::collections::HashMap<Arc<str>, usize>,
    u64,
)> {
    let mut compact_entries = Vec::with_capacity(loaded_entries.len());
    let mut entry_lookup = std::collections::HashMap::with_capacity(loaded_entries.len());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (index, entry) in loaded_entries.iter().enumerate() {
        if super::super::super::search_job_canceled_for_index(queue, generation, index) {
            return None;
        }
        let compact_entry = compact_search_entry_for(entry);
        compact_entry.relative_path.as_ref().hash(&mut hasher);
        entry_lookup.insert(Arc::clone(&compact_entry.relative_path), index);
        compact_entries.push(compact_entry);
    }
    if super::super::super::search_job_canceled(queue, generation) {
        return None;
    }
    Some((compact_entries, entry_lookup, hasher.finish()))
}

fn compact_search_entry_for(
    entry: &crate::sample_sources::db::read::SearchEntryRow,
) -> CompactSearchEntry {
    let relative_path: Arc<str> = Arc::from(entry.relative_path.to_string_lossy().into_owned());
    let display_label = compact_search_display_label(&entry.relative_path, &entry.metadata);
    CompactSearchEntry {
        display_label: display_label.into_boxed_str(),
        relative_path,
        tag: entry.metadata.tag,
        locked: entry.metadata.locked,
        last_played_at: entry.metadata.last_played_at,
        tag_named: entry.metadata.tag_named,
    }
}

pub(super) fn compact_search_display_label(
    relative_path: &std::path::Path,
    metadata: &crate::sample_sources::db::read::SearchEntryMetadata,
) -> String {
    let mut label = crate::app::view_model::sample_display_label(relative_path);
    for tag in &metadata.normal_tags {
        label.push(' ');
        label.push_str(tag);
    }
    label
}
