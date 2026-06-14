use super::*;

use super::audio::invalidate_new_entry_audio;
use super::projection::apply_insert_projection;
use super::source_entries::insert_entry_at_database_index;

/// Invalidate caches after inserting a new entry for a source.
pub(crate) fn insert_cached_entry(
    controller: &mut AppController,
    source: &SampleSource,
    entry: WavEntry,
) {
    let entry_index = controller
        .database_for(source)
        .ok()
        .and_then(|db| db.index_for_path(&entry.relative_path).ok().flatten());
    let insertion = insert_entry_at_database_index(controller, source, &entry, entry_index);
    apply_insert_projection(controller, source, insertion);
    invalidate_new_entry_audio(controller, &source.id, &entry.relative_path);
}
