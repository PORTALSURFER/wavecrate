use super::*;
use crate::app::controller::jobs::SourceMetadataMutationOp;
use crate::app::controller::state::runtime::MetadataRollback;
use crate::app_core::actions::NativeBrowserTagState;
use crate::sample_sources::db::SourceTag;
use std::collections::BTreeSet;

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
    set_sample_looped_for_source_batch(
        controller,
        source,
        std::slice::from_ref(&path.to_path_buf()),
        looped,
        false,
    )?;
    Ok(())
}

/// Persist and propagate a loop-marker update for a batch of paths in one source.
pub(crate) fn set_sample_looped_for_source_batch(
    controller: &mut AppController,
    source: &SampleSource,
    paths: &[PathBuf],
    looped: bool,
    _require_present: bool,
) -> Result<usize, String> {
    let paths = unique_paths(paths);
    if paths.is_empty() {
        return Ok(0);
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

/// Return the selected target-set assignment state for one normal library tag.
pub(crate) fn normal_tag_state_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    paths: &[PathBuf],
    label: &str,
) -> Result<NativeBrowserTagState, String> {
    let Some(identity) = normalize_normal_tag_label(label) else {
        return Ok(NativeBrowserTagState::Off);
    };
    if paths.is_empty() {
        return Ok(NativeBrowserTagState::Off);
    }
    let mut assigned = 0usize;
    for path in paths {
        if normal_tag_assigned_for_source(controller, source, path, &identity.normalized_text)? {
            assigned += 1;
        }
    }
    Ok(match assigned {
        0 => NativeBrowserTagState::Off,
        count if count == paths.len() => NativeBrowserTagState::On,
        _ => NativeBrowserTagState::Mixed,
    })
}

/// Persist and propagate one normal library tag assignment.
pub(crate) fn apply_normal_tag_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    label: &str,
) -> Result<(), String> {
    set_normal_tag_for_source(controller, source, path, label, true)
}

/// Persist and propagate removal of one normal library tag assignment.
pub(crate) fn remove_normal_tag_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    label: &str,
) -> Result<(), String> {
    set_normal_tag_for_source(controller, source, path, label, false)
}

fn set_normal_tag_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    label: &str,
    assigned: bool,
) -> Result<(), String> {
    set_normal_tag_for_source_batch(
        controller,
        source,
        std::slice::from_ref(&path.to_path_buf()),
        label,
        assigned,
    )?;
    Ok(())
}

/// Persist and propagate one normal library tag assignment state for a source batch.
pub(crate) fn set_normal_tag_for_source_batch(
    controller: &mut AppController,
    source: &SampleSource,
    paths: &[PathBuf],
    label: &str,
    assigned: bool,
) -> Result<usize, String> {
    let identity =
        normalize_normal_tag_label(label).ok_or_else(|| "Tag label cannot be empty".to_string())?;
    let paths = unique_paths(paths);
    if paths.is_empty() {
        return Ok(0);
    }
    #[cfg(test)]
    let optimistic_started_at = std::time::Instant::now();
    let mut source_ops = Vec::with_capacity(paths.len());
    let mut rollback = Vec::with_capacity(paths.len());
    for path in &paths {
        let before_present =
            normal_tag_assigned_for_source(controller, source, path, &identity.normalized_text)?;
        update_normal_tag_cache(
            controller,
            &source.id,
            path,
            &identity.display_label,
            &identity.normalized_text,
            assigned,
        );
        source_ops.push(if assigned {
            SourceMetadataMutationOp::AssignNormalTag {
                relative_path: path.clone(),
                label: identity.display_label.clone(),
            }
        } else {
            SourceMetadataMutationOp::RemoveNormalTag {
                relative_path: path.clone(),
                label: identity.display_label.clone(),
            }
        });
        rollback.push(MetadataRollback::NormalTag {
            relative_path: path.clone(),
            normalized_text: identity.normalized_text.clone(),
            display_label: identity.display_label.clone(),
            before_present,
            expected_present: assigned,
        });
    }
    controller.ui_cache.browser.pipeline.invalidate();
    controller.mark_browser_row_metadata_projection_revision_dirty();
    controller.mark_browser_search_projection_revision_dirty();
    #[cfg(test)]
    crate::app::controller::batch_latency::record(
        crate::app::controller::batch_latency::BatchLatencySample::new(
            crate::app::controller::batch_latency::BatchLatencyPhase::TagSidebarOptimisticTag,
            paths.len(),
            optimistic_started_at.elapsed(),
        ),
    );
    controller.queue_metadata_mutation(source, source_ops, Vec::new(), rollback, true);
    Ok(paths.len())
}

pub(crate) fn normal_tags_for_path(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
) -> Result<Vec<SourceTag>, String> {
    if let Some(tags) = controller
        .ui_cache
        .browser
        .normal_tags
        .get(&source.id)
        .and_then(|source_tags| source_tags.get(path))
    {
        return Ok(tags.clone());
    }
    let tags = controller
        .database_for(source)
        .map_err(|err| err.to_string())?
        .tags_for_path(path)
        .map_err(|err| err.to_string())?;
    controller
        .ui_cache
        .browser
        .normal_tags
        .entry(source.id.clone())
        .or_default()
        .insert(path.to_path_buf(), tags.clone());
    Ok(tags)
}

fn normal_tag_assigned_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    normalized_text: &str,
) -> Result<bool, String> {
    Ok(normal_tags_for_path(controller, source, path)?
        .iter()
        .any(|tag| tag.normalized_text == normalized_text))
}

fn unique_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
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

fn update_normal_tag_cache(
    controller: &mut AppController,
    source_id: &SourceId,
    path: &Path,
    display_label: &str,
    normalized_text: &str,
    assigned: bool,
) {
    if let Some(index) = controller.wav_index_for_path(path) {
        let _ = controller.ensure_wav_page_loaded(index);
        if let Some(entry) = controller.wav_entries.entry_mut(index) {
            update_normal_tag_labels(
                &mut entry.normal_tags,
                display_label,
                normalized_text,
                assigned,
            );
        }
    }
    if let Some(cache) = controller.cache.wav.entries.get_mut(source_id)
        && let Some(index) = cache.lookup.get(path).copied()
        && let Some(entry) = cache.entry_mut(index)
    {
        update_normal_tag_labels(
            &mut entry.normal_tags,
            display_label,
            normalized_text,
            assigned,
        );
    }
    let tags = controller
        .ui_cache
        .browser
        .normal_tags
        .entry(source_id.clone())
        .or_default()
        .entry(path.to_path_buf())
        .or_default();
    if assigned {
        if !tags
            .iter()
            .any(|tag| tag.normalized_text == normalized_text)
        {
            tags.push(SourceTag {
                id: 0,
                display_label: display_label.to_string(),
                normalized_text: normalized_text.to_string(),
            });
        }
        tags.sort_by(|left, right| {
            left.display_label
                .to_ascii_lowercase()
                .cmp(&right.display_label.to_ascii_lowercase())
                .then_with(|| left.normalized_text.cmp(&right.normalized_text))
        });
    } else {
        tags.retain(|tag| tag.normalized_text != normalized_text);
    }
}

fn update_normal_tag_labels(
    labels: &mut Vec<String>,
    display_label: &str,
    normalized_text: &str,
    assigned: bool,
) {
    if assigned {
        if !labels
            .iter()
            .any(|label| label.to_ascii_lowercase() == normalized_text)
        {
            labels.push(display_label.to_string());
        }
        labels.sort_by_key(|label| label.to_ascii_lowercase());
    } else {
        labels.retain(|label| label.to_ascii_lowercase() != normalized_text);
    }
}

struct NormalTagIdentity {
    display_label: String,
    normalized_text: String,
}

fn normalize_normal_tag_label(label: &str) -> Option<NormalTagIdentity> {
    let display_label = label.split_whitespace().collect::<Vec<_>>().join(" ");
    if display_label.is_empty() {
        return None;
    }
    Some(NormalTagIdentity {
        normalized_text: display_label.to_ascii_lowercase(),
        display_label,
    })
}
