use super::*;
use crate::app::controller::jobs::SourceMetadataMutationOp;
use crate::app::controller::state::runtime::MetadataRollback;

fn sample_locked_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
) -> Result<bool, String> {
    if let Some(index) = controller.wav_index_for_path(path) {
        let _ = controller.ensure_wav_page_loaded(index);
        if let Some(entry) = controller.wav_entries.entry(index) {
            return Ok(entry.locked);
        }
    }
    if let Some(cache) = controller.cache.wav.entries.get(&source.id)
        && let Some(index) = cache.lookup.get(path).copied()
        && let Some(entry) = cache.entry(index)
    {
        return Ok(entry.locked);
    }
    controller
        .database_for(source)
        .map_err(|err| err.to_string())?
        .locked_for_path(path)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Sample not found".to_string())
}

/// Set a triage-column tag for the target sample path.
pub(crate) fn set_sample_tag(
    controller: &mut AppController,
    path: &Path,
    column: TriageFlagColumn,
) -> Result<(), String> {
    let target_tag = match column {
        TriageFlagColumn::Trash => Rating::TRASH_3,
        TriageFlagColumn::Neutral => Rating::NEUTRAL,
        TriageFlagColumn::Keep => Rating::KEEP_1,
    };
    set_sample_tag_value(controller, path, target_tag)
}

/// Set an explicit rating tag for the target sample path.
pub(crate) fn set_sample_tag_value(
    controller: &mut AppController,
    path: &Path,
    target_tag: Rating,
) -> Result<(), String> {
    let Some(source) = controller.current_source() else {
        return Err("Select a source first".into());
    };
    set_sample_tag_for_source(controller, &source, path, target_tag, true)
}

/// Persist and propagate a rating tag update for a specific source/path.
///
/// The top keep state (`KEEP_3`) also promotes the sample into the persistent
/// locked state so direct tagging and incremental rating use the same keep-lock rule.
pub(crate) fn set_sample_tag_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    target_tag: Rating,
    require_present: bool,
) -> Result<(), String> {
    let target_locked = target_tag == Rating::KEEP_3;
    set_sample_tag_and_locked_for_source(
        controller,
        source,
        path,
        target_tag,
        target_locked,
        require_present,
    )
}

/// Persist and propagate a rating tag plus keep-lock update for a specific source/path.
pub(crate) fn set_sample_tag_and_locked_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    target_tag: Rating,
    locked: bool,
    _require_present: bool,
) -> Result<(), String> {
    let before_tag = controller
        .wav_index_for_path(path)
        .and_then(|index| controller.wav_entry(index).map(|entry| entry.tag))
        .or_else(|| {
            controller
                .cache
                .wav
                .entries
                .get(&source.id)
                .and_then(|cache| cache.lookup.get(path).copied())
                .and_then(|index| controller.cache.wav.entries.get(&source.id)?.entry(index))
                .map(|entry| entry.tag)
        })
        .unwrap_or(Rating::NEUTRAL);
    let before_locked = controller
        .wav_index_for_path(path)
        .and_then(|index| controller.wav_entry(index).map(|entry| entry.locked))
        .or_else(|| {
            controller
                .cache
                .wav
                .entries
                .get(&source.id)
                .and_then(|cache| cache.lookup.get(path).copied())
                .and_then(|index| controller.cache.wav.entries.get(&source.id)?.entry(index))
                .map(|entry| entry.locked)
        })
        .unwrap_or(false);
    let mut updated_active = false;
    if let Some(index) = controller.wav_index_for_path(path) {
        let _ = controller.ensure_wav_page_loaded(index);
        if let Some(entry) = controller.wav_entries.entry_mut(index) {
            entry.tag = target_tag;
            entry.locked = locked;
            updated_active = true;
        }
    }
    if let Some(cache) = controller.cache.wav.entries.get_mut(&source.id)
        && let Some(index) = cache.lookup.get(path).copied()
        && let Some(entry) = cache.entry_mut(index)
    {
        entry.tag = target_tag;
        entry.locked = locked;
    }
    controller.ui_cache.browser.pipeline.invalidate();
    controller.mark_browser_row_metadata_projection_revision_dirty();
    controller.mark_browser_search_projection_revision_dirty();
    controller.queue_metadata_mutation(
        source,
        vec![SourceMetadataMutationOp::SetTagAndLocked {
            relative_path: path.to_path_buf(),
            tag: target_tag,
            locked,
        }],
        Vec::new(),
        vec![MetadataRollback::TagAndLocked {
            relative_path: path.to_path_buf(),
            before_tag,
            before_locked,
            expected_tag: target_tag,
            expected_locked: locked,
        }],
        updated_active,
    );
    Ok(())
}

/// Persist and propagate a keep-lock update for a specific source/path.
pub(crate) fn set_sample_locked_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    locked: bool,
    require_present: bool,
) -> Result<(), String> {
    let tag = controller
        .wav_index_for_path(path)
        .and_then(|index| controller.wav_entry(index).map(|entry| entry.tag))
        .or_else(|| {
            controller
                .cache
                .wav
                .entries
                .get(&source.id)
                .and_then(|cache| cache.lookup.get(path).copied())
                .and_then(|index| controller.cache.wav.entries.get(&source.id)?.entry(index))
                .map(|entry| entry.tag)
        })
        .or_else(|| {
            controller
                .database_for(source)
                .ok()?
                .tag_for_path(path)
                .ok()
                .flatten()
        })
        .ok_or_else(|| "Sample not found".to_string())?;
    set_sample_tag_and_locked_for_source(controller, source, path, tag, locked, require_present)
}

/// Persist and propagate a loop-marker update for a specific source/path.
pub(crate) fn set_sample_looped_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    looped: bool,
    _require_present: bool,
) -> Result<(), String> {
    let before_looped = controller
        .wav_index_for_path(path)
        .and_then(|index| controller.wav_entry(index).map(|entry| entry.looped))
        .or_else(|| {
            controller
                .cache
                .wav
                .entries
                .get(&source.id)
                .and_then(|cache| cache.lookup.get(path).copied())
                .and_then(|index| controller.cache.wav.entries.get(&source.id)?.entry(index))
                .map(|entry| entry.looped)
        })
        .unwrap_or(false);
    if let Some(index) = controller.wav_index_for_path(path) {
        let _ = controller.ensure_wav_page_loaded(index);
        if let Some(entry) = controller.wav_entries.entry_mut(index) {
            entry.looped = looped;
        }
    }
    if let Some(cache) = controller.cache.wav.entries.get_mut(&source.id)
        && let Some(index) = cache.lookup.get(path).copied()
        && let Some(entry) = cache.entry_mut(index)
    {
        entry.looped = looped;
    }
    controller.ui_cache.browser.pipeline.invalidate();
    controller.mark_browser_row_metadata_projection_revision_dirty();
    controller.queue_metadata_mutation(
        source,
        vec![SourceMetadataMutationOp::SetLooped {
            relative_path: path.to_path_buf(),
            looped,
        }],
        Vec::new(),
        vec![MetadataRollback::Looped {
            relative_path: path.to_path_buf(),
            before_looped,
            expected_looped: looped,
        }],
        false,
    );
    Ok(())
}
