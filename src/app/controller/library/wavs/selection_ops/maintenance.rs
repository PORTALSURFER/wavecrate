use super::super::*;

pub(crate) fn rebuild_wav_lookup(controller: &mut AppController) {
    controller.wav_entries.lookup.clear();
    let mut entries = Vec::new();
    for (page_index, page) in controller.wav_entries.pages.iter() {
        let base = page_index * controller.wav_entries.page_size;
        for (idx, entry) in page.iter().enumerate() {
            entries.push((entry.relative_path.clone(), base + idx));
        }
    }
    for (path, index) in entries {
        controller.wav_entries.insert_lookup(path, index);
    }
}

pub(crate) fn invalidate_cached_audio_for_entry_updates(
    controller: &mut AppController,
    source_id: &SourceId,
    updates: &[(WavEntry, WavEntry)],
) {
    for (old_entry, new_entry) in updates {
        controller.invalidate_cached_audio(source_id, &old_entry.relative_path);
        controller.invalidate_cached_audio(source_id, &new_entry.relative_path);
    }
}
