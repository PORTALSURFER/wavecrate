use super::*;
use crate::app::controller::jobs::SourceMetadataMutationOp;
use crate::app::controller::state::runtime::MetadataRollback;

/// Persist and propagate a loop-marker update for a specific source/path.
pub(crate) fn set_sample_looped_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    looped: bool,
    require_present: bool,
) -> Result<(), String> {
    set_sample_looped_for_source_batch(
        controller,
        source,
        std::slice::from_ref(&path.to_path_buf()),
        looped,
        require_present,
    )?;
    Ok(())
}

/// Persist and propagate a loop-marker update for a batch of paths in one source.
pub(crate) fn set_sample_looped_for_source_batch(
    controller: &mut AppController,
    source: &SampleSource,
    paths: &[PathBuf],
    looped: bool,
    require_present: bool,
) -> Result<usize, String> {
    let paths = unique_paths(paths);
    if paths.is_empty() {
        return Ok(0);
    }
    if require_present {
        for path in &paths {
            require_sample_present_for_source(controller, source, path)?;
        }
    }
    let mut source_ops = Vec::with_capacity(paths.len());
    let mut rollback = Vec::with_capacity(paths.len());
    for path in &paths {
        let before_looped = looped_for_source_from_controller(controller, source, path);
        let intent_id = controller
            .runtime
            .source_lane
            .mutations
            .begin_looped_metadata_intent(&source.id, path);
        update_looped_caches(controller, &source.id, path, looped);
        source_ops.push(SourceMetadataMutationOp::SetLooped {
            relative_path: path.clone(),
            looped,
        });
        rollback.push(MetadataRollback::Looped {
            relative_path: path.clone(),
            intent_id,
            before_looped,
            expected_looped: looped,
        });
    }
    controller.ui_cache.browser.pipeline.invalidate();
    controller.mark_browser_row_metadata_projection_revision_dirty();
    controller.queue_metadata_mutation(source, source_ops, Vec::new(), rollback, false);
    Ok(paths.len())
}

/// Persist and propagate a sound-type metadata update for a specific source/path.
pub(crate) fn set_sample_sound_type_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    sound_type: Option<crate::sample_sources::SampleSoundType>,
) -> Result<(), String> {
    let before_sound_type = controller
        .wav_index_for_path(path)
        .and_then(|index| {
            controller
                .wav_entry(index)
                .and_then(|entry| entry.sound_type)
        })
        .or_else(|| {
            controller
                .cache
                .wav
                .entries
                .get(&source.id)
                .and_then(|cache| cache.lookup.get(path).copied())
                .and_then(|index| controller.cache.wav.entries.get(&source.id)?.entry(index))
                .and_then(|entry| entry.sound_type)
        });
    if let Some(index) = controller.wav_index_for_path(path) {
        let _ = controller.ensure_wav_page_loaded(index);
        if let Some(entry) = controller.wav_entries.entry_mut(index) {
            entry.sound_type = sound_type;
        }
    }
    if let Some(cache) = controller.cache.wav.entries.get_mut(&source.id)
        && let Some(index) = cache.lookup.get(path).copied()
        && let Some(entry) = cache.entry_mut(index)
    {
        entry.sound_type = sound_type;
    }
    controller.ui_cache.browser.pipeline.invalidate();
    controller.mark_browser_row_metadata_projection_revision_dirty();
    controller.mark_browser_search_projection_revision_dirty();
    controller.queue_metadata_mutation(
        source,
        vec![SourceMetadataMutationOp::SetSoundType {
            relative_path: path.to_path_buf(),
            sound_type,
        }],
        Vec::new(),
        vec![MetadataRollback::SoundType {
            relative_path: path.to_path_buf(),
            before_sound_type,
            expected_sound_type: sound_type,
        }],
        false,
    );
    Ok(())
}

/// Persist and propagate a single custom user-tag update for a specific source/path.
pub(crate) fn set_sample_user_tag_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    user_tag: Option<String>,
) -> Result<(), String> {
    let normalized = user_tag.and_then(|tag| {
        let trimmed = tag.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });
    let before_user_tag = controller
        .wav_index_for_path(path)
        .and_then(|index| {
            controller
                .wav_entry(index)
                .and_then(|entry| entry.user_tag.clone())
        })
        .or_else(|| {
            controller
                .cache
                .wav
                .entries
                .get(&source.id)
                .and_then(|cache| cache.lookup.get(path).copied())
                .and_then(|index| controller.cache.wav.entries.get(&source.id)?.entry(index))
                .and_then(|entry| entry.user_tag.clone())
        });
    if let Some(index) = controller.wav_index_for_path(path) {
        let _ = controller.ensure_wav_page_loaded(index);
        if let Some(entry) = controller.wav_entries.entry_mut(index) {
            entry.user_tag = normalized.clone();
        }
    }
    if let Some(cache) = controller.cache.wav.entries.get_mut(&source.id)
        && let Some(index) = cache.lookup.get(path).copied()
        && let Some(entry) = cache.entry_mut(index)
    {
        entry.user_tag = normalized.clone();
    }
    controller.ui_cache.browser.pipeline.invalidate();
    controller.mark_browser_row_metadata_projection_revision_dirty();
    controller.mark_browser_search_projection_revision_dirty();
    controller.queue_metadata_mutation(
        source,
        vec![SourceMetadataMutationOp::SetUserTag {
            relative_path: path.to_path_buf(),
            user_tag: normalized.clone(),
        }],
        Vec::new(),
        vec![MetadataRollback::UserTag {
            relative_path: path.to_path_buf(),
            before_user_tag,
            expected_user_tag: normalized,
        }],
        false,
    );
    Ok(())
}

fn looped_for_source_from_controller(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
) -> bool {
    controller
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
        .unwrap_or(false)
}

fn update_looped_caches(
    controller: &mut AppController,
    source_id: &SourceId,
    path: &Path,
    looped: bool,
) {
    if let Some(index) = controller.wav_index_for_path(path) {
        let _ = controller.ensure_wav_page_loaded(index);
        if let Some(entry) = controller.wav_entries.entry_mut(index) {
            entry.looped = looped;
        }
    }
    if let Some(cache) = controller.cache.wav.entries.get_mut(source_id)
        && let Some(index) = cache.lookup.get(path).copied()
        && let Some(entry) = cache.entry_mut(index)
    {
        entry.looped = looped;
    }
}
