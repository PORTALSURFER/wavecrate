use super::*;
use std::path::Path;

pub(super) fn remap_missing_file_state(
    controller: &mut AppController,
    source_id: &SourceId,
    old_path: &Path,
    new_entry: &WavEntry,
) {
    if let Some(missing) = controller.library.missing.wavs.get_mut(source_id) {
        let removed = missing.remove(old_path);
        if removed && new_entry.missing {
            missing.insert(new_entry.relative_path.clone());
        }
    }
}
