use super::*;
use std::path::Path;

pub(super) fn invalidate_old_entry_audio(
    controller: &mut AppController,
    source_id: &SourceId,
    old_path: &Path,
) {
    controller.invalidate_cached_audio(source_id, old_path);
}

pub(super) fn invalidate_new_entry_audio(
    controller: &mut AppController,
    source_id: &SourceId,
    new_path: &Path,
) {
    controller.invalidate_cached_audio(source_id, new_path);
}
