use super::*;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct EntryCacheMutation {
    pub(super) updated: bool,
    pub(super) selected_index: Option<usize>,
}

impl EntryCacheMutation {
    fn record(&mut self, updated: bool, selected_index: Option<usize>) {
        self.updated |= updated;
        self.selected_index = self.selected_index.or(selected_index);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EntryInsertion {
    pub(super) selected_entries_loaded: bool,
    pub(super) selected_inserted: bool,
    pub(super) selected_insert_index: Option<usize>,
}

pub(super) fn update_same_path_entry(
    controller: &mut AppController,
    source_id: &SourceId,
    path: &Path,
    new_entry: &WavEntry,
) -> EntryCacheMutation {
    let mut mutation = EntryCacheMutation::default();
    if controller.selection_state.ctx.selected_source.as_ref() == Some(source_id) {
        let updated = controller.wav_entries.update_entry(path, new_entry.clone());
        let selected_index = controller.wav_entries.lookup.get(path).copied();
        mutation.record(updated, selected_index);
    }
    if let Some(cache) = controller.cache.wav.entries.get_mut(source_id) {
        let updated = cache.update_entry(path, new_entry.clone());
        let selected_index = cache.lookup.get(path).copied();
        mutation.record(updated, selected_index);
    }
    mutation
}

pub(super) fn update_path_changed_entry(
    controller: &mut AppController,
    source_id: &SourceId,
    old_path: &Path,
    new_entry: &WavEntry,
) -> EntryCacheMutation {
    let mut mutation = EntryCacheMutation::default();
    if controller.selection_state.ctx.selected_source.as_ref() == Some(source_id)
        && let Some(index) = controller.wav_entries.lookup.get(old_path).copied()
        && let Some(slot) = controller.wav_entries.entry_mut(index)
    {
        *slot = new_entry.clone();
        controller.wav_entries.lookup.remove(old_path);
        controller
            .wav_entries
            .insert_lookup(new_entry.relative_path.clone(), index);
        update_browser_focus_path(controller, old_path, &new_entry.relative_path, index);
        mutation.record(true, Some(index));
    }
    if let Some(cache) = controller.cache.wav.entries.get_mut(source_id)
        && let Some(index) = cache.lookup.get(old_path).copied()
        && let Some(slot) = cache.entry_mut(index)
    {
        *slot = new_entry.clone();
        cache.lookup.remove(old_path);
        cache.insert_lookup(new_entry.relative_path.clone(), index);
        mutation.record(true, None);
    }
    mutation
}

pub(super) fn insert_entry_at_database_index(
    controller: &mut AppController,
    source: &SampleSource,
    entry: &WavEntry,
    entry_index: Option<usize>,
) -> EntryInsertion {
    let selected_source_active =
        controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id);
    let selected_entries_loaded =
        selected_source_active && controller.wav_entries.source_id.as_ref() == Some(&source.id);
    let mut selected_inserted = false;
    let mut selected_insert_index = None;
    if let Some(index) = entry_index {
        if selected_entries_loaded {
            selected_inserted = controller.wav_entries.insert_entry_at(index, entry.clone());
            if selected_inserted {
                selected_insert_index = Some(index);
            }
        }
        if let Some(cache) = controller.cache.wav.entries.get_mut(&source.id) {
            let _ = cache.insert_entry_at(index, entry.clone());
        }
    }
    EntryInsertion {
        selected_entries_loaded,
        selected_inserted,
        selected_insert_index,
    }
}

fn update_browser_focus_path(
    controller: &mut AppController,
    old_path: &Path,
    new_path: &Path,
    index: usize,
) {
    if controller.ui.browser.selection.last_focused_index == Some(index)
        || controller.ui.browser.selection.last_focused_path.as_deref() == Some(old_path)
    {
        controller.ui.browser.selection.last_focused_index = Some(index);
        controller.ui.browser.selection.last_focused_path = Some(new_path.to_path_buf());
    }
}
