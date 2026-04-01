use super::*;
use std::path::Path;

/// Update all cached structures after a file path or metadata change.
pub(crate) fn update_cached_entry(
    controller: &mut AppController,
    source: &SampleSource,
    old_path: &Path,
    new_entry: WavEntry,
) {
    update_selection_paths(controller, source, old_path, &new_entry.relative_path);
    controller.invalidate_cached_audio(&source.id, old_path);
    if let Some(missing) = controller.library.missing.wavs.get_mut(&source.id) {
        let removed = missing.remove(old_path);
        if removed && new_entry.missing {
            missing.insert(new_entry.relative_path.clone());
        }
    }
    if old_path == new_entry.relative_path {
        let mut updated = false;
        if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
            updated |= controller
                .wav_entries
                .update_entry(old_path, new_entry.clone());
        }
        if let Some(cache) = controller.cache.wav.entries.get_mut(&source.id) {
            updated |= cache.update_entry(old_path, new_entry.clone());
        }
        if updated && controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
            controller.rebuild_browser_lists();
        }
        return;
    }
    if let Ok(db) = controller.database_for(source)
        && matches!(db.index_for_path(old_path), Ok(Some(_)))
    {
        let _ = controller.rewrite_db_entry_for_source(
            source,
            old_path,
            &new_entry.relative_path,
            new_entry.file_size,
            new_entry.modified_ns,
            new_entry.tag,
        );
    }
    let mut updated = false;
    if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id)
        && let Some(index) = controller.wav_entries.lookup.get(old_path).copied()
        && let Some(slot) = controller.wav_entries.entry_mut(index)
    {
        *slot = new_entry.clone();
        controller.wav_entries.lookup.remove(old_path);
        controller
            .wav_entries
            .insert_lookup(new_entry.relative_path.clone(), index);
        updated = true;
        if controller.ui.browser.selection.last_focused_index == Some(index)
            || controller.ui.browser.selection.last_focused_path.as_deref() == Some(old_path)
        {
            controller.ui.browser.selection.last_focused_index = Some(index);
            controller.ui.browser.selection.last_focused_path =
                Some(new_entry.relative_path.clone());
        }
    }
    if let Some(cache) = controller.cache.wav.entries.get_mut(&source.id)
        && let Some(index) = cache.lookup.get(old_path).copied()
        && let Some(slot) = cache.entry_mut(index)
    {
        *slot = new_entry.clone();
        cache.lookup.remove(old_path);
        cache.insert_lookup(new_entry.relative_path.clone(), index);
        updated = true;
    }
    if updated {
        if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
            controller.ui_cache.browser.search.invalidate();
            controller.ui_cache.browser.pipeline.invalidate();
            controller.rebuild_browser_lists();
        }
        if old_path != new_entry.relative_path {
            controller.ui_cache.browser.labels.remove(&source.id);
        }
    } else {
        controller.invalidate_wav_entries_for_source_preserve_folders(source);
    }
    controller.invalidate_cached_audio(&source.id, &new_entry.relative_path);
}

/// Invalidate caches after inserting a new entry for a source.
pub(crate) fn insert_cached_entry(
    controller: &mut AppController,
    source: &SampleSource,
    entry: WavEntry,
) {
    let selected_source_active =
        controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id);
    let selected_entries_loaded =
        selected_source_active && controller.wav_entries.source_id.as_ref() == Some(&source.id);
    let entry_index = controller
        .database_for(source)
        .ok()
        .and_then(|db| db.index_for_path(&entry.relative_path).ok().flatten());
    let mut selected_inserted = false;
    if let Some(index) = entry_index {
        if selected_entries_loaded {
            selected_inserted = controller.wav_entries.insert_entry_at(index, entry.clone());
        }
        if let Some(cache) = controller.cache.wav.entries.get_mut(&source.id) {
            let _ = cache.insert_entry_at(index, entry.clone());
        }
    }
    if selected_source_active {
        if selected_entries_loaded && selected_inserted {
            controller.ui_cache.browser.labels.remove(&source.id);
            controller.ui_cache.browser.bpm_values.remove(&source.id);
            controller.ui_cache.browser.search.invalidate();
            controller.ui_cache.browser.pipeline.invalidate();
            controller.rebuild_browser_lists();
        } else if selected_entries_loaded {
            controller.invalidate_wav_entries_for_source(source);
        } else {
            controller.ui_cache.browser.labels.remove(&source.id);
            controller.ui_cache.browser.bpm_values.remove(&source.id);
            controller.ui_cache.browser.search.invalidate();
            controller.ui_cache.browser.pipeline.invalidate();
        }
    } else {
        controller.ui_cache.browser.labels.remove(&source.id);
        controller.ui_cache.browser.bpm_values.remove(&source.id);
    }
    controller.invalidate_cached_audio(&source.id, &entry.relative_path);
}

/// Rewrite selection paths when a file is renamed or moved.
pub(crate) fn update_selection_paths(
    controller: &mut AppController,
    source: &SampleSource,
    old_path: &Path,
    new_path: &Path,
) {
    controller.update_compare_anchor_path(&source.id, old_path, new_path);
    controller.remap_browser_marked_path(&source.id, old_path, new_path);
    if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
        if !controller.ui.browser.selection.selected_paths.is_empty() {
            let mut updated =
                Vec::with_capacity(controller.ui.browser.selection.selected_paths.len());
            let mut replaced = false;
            for path in controller.ui.browser.selection.selected_paths.iter() {
                if path == old_path {
                    replaced = true;
                    if !updated.iter().any(|candidate| candidate == new_path) {
                        updated.push(new_path.to_path_buf());
                    }
                } else {
                    updated.push(path.clone());
                }
            }
            if replaced {
                controller.set_browser_selected_paths(updated);
            }
        }
        if controller.sample_view.wav.selected_wav.as_deref() == Some(old_path) {
            controller.sample_view.wav.selected_wav = Some(new_path.to_path_buf());
        }
        if controller.sample_view.wav.loaded_wav.as_deref() == Some(old_path) {
            controller.sample_view.wav.loaded_wav = Some(new_path.to_path_buf());
            controller.set_ui_loaded_wav(Some(new_path.to_path_buf()));
        } else if controller.ui.loaded_wav.as_deref() == Some(old_path) {
            controller.set_ui_loaded_wav(Some(new_path.to_path_buf()));
        }
    }
    if let Some(audio) = controller.sample_view.wav.loaded_audio.as_mut()
        && audio.source_id == source.id
        && audio.relative_path == old_path
    {
        audio.relative_path = new_path.to_path_buf();
    }
}
