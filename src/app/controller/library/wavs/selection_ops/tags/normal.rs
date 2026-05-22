use super::*;
use crate::app::controller::jobs::SourceMetadataMutationOp;
use crate::app::controller::state::runtime::MetadataRollback;
use crate::app_core::actions::NativeBrowserTagState;
use crate::sample_sources::db::SourceTag;

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

pub(crate) fn apply_normal_tag_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    label: &str,
) -> Result<(), String> {
    set_normal_tag_for_source_batch(
        controller,
        source,
        std::slice::from_ref(&path.to_path_buf()),
        label,
        true,
    )?;
    Ok(())
}

pub(crate) fn remove_normal_tag_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    path: &Path,
    label: &str,
) -> Result<(), String> {
    set_normal_tag_for_source_batch(
        controller,
        source,
        std::slice::from_ref(&path.to_path_buf()),
        label,
        false,
    )?;
    Ok(())
}

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
