use super::*;
use std::path::{Path, PathBuf};

/// Resolve cached BPM metadata for one path, querying the source DB on cache miss.
pub(super) fn bpm_value_for_path(controller: &mut AppController, path: &Path) -> Option<f32> {
    let source = controller.current_source()?;
    if let Some(cache) = controller.ui_cache.browser.bpm_values.get(&source.id)
        && let Some(cached) = cache.get(path)
    {
        return *cached;
    }
    let db = controller.database_for(&source).ok()?;
    let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), path);
    let bpm = db.bpm_for_sample_id(&sample_id).ok().flatten();
    let cache = controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default();
    cache.insert(path.to_path_buf(), bpm);
    bpm
}

/// Preload BPM metadata for a visible row window to avoid per-row DB lookups.
pub(super) fn preload_bpm_values_for_paths(controller: &mut AppController, paths: &[PathBuf]) {
    if paths.is_empty() {
        return;
    }
    let Some(source) = controller.current_source() else {
        return;
    };
    let source_id = source.id.clone();
    let cache = controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source_id.clone())
        .or_default();
    let mut missing_paths = Vec::new();
    let mut missing_sample_ids = Vec::new();
    for path in paths {
        if cache.contains_key(path) {
            continue;
        }
        missing_paths.push(path.clone());
        missing_sample_ids.push(analysis_jobs::build_sample_id(source_id.as_str(), path));
    }
    if missing_paths.is_empty() {
        return;
    }
    let db = match controller.database_for(&source) {
        Ok(db) => db,
        Err(err) => {
            tracing::debug!("Skipping BPM preload (database unavailable): {err}");
            return;
        }
    };
    let bpm_lookup = match db.bpms_for_sample_ids(&missing_sample_ids) {
        Ok(values) => values,
        Err(err) => {
            tracing::debug!("Skipping BPM preload (batch lookup failed): {err}");
            return;
        }
    };
    let cache = controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source_id)
        .or_default();
    for (path, sample_id) in missing_paths
        .into_iter()
        .zip(missing_sample_ids.into_iter())
    {
        let bpm = bpm_lookup.get(sample_id.as_str()).copied().flatten();
        cache.insert(path, bpm);
    }
}

/// Resolve the tag for a wav entry, falling back to the source database.
pub(super) fn sample_tag_for(
    controller: &mut AppController,
    source: &SampleSource,
    relative_path: &Path,
) -> Result<crate::sample_sources::Rating, String> {
    if let Some(cache) = controller.cache.wav.entries.get(&source.id)
        && let Some(index) = cache.lookup.get(relative_path).copied()
        && let Some(entry) = cache.entry(index)
    {
        return Ok(entry.tag);
    }
    if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id)
        && let Some(index) = controller.wav_index_for_path(relative_path)
        && let Some(entry) = controller.wav_entries.entry(index)
    {
        return Ok(entry.tag);
    }
    let db = controller
        .database_for(source)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    db.tag_for_path(relative_path)
        .map_err(|err| format!("Failed to read database: {err}"))?
        .ok_or_else(|| "Sample not found in database".to_string())
}

/// Resolve the loop marker state for a wav entry.
pub(super) fn sample_looped_for(
    controller: &mut AppController,
    source: &SampleSource,
    relative_path: &Path,
) -> Result<bool, String> {
    if let Some(cache) = controller.cache.wav.entries.get(&source.id)
        && let Some(index) = cache.lookup.get(relative_path).copied()
        && let Some(entry) = cache.entry(index)
    {
        return Ok(entry.looped);
    }
    if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id)
        && let Some(index) = controller.wav_index_for_path(relative_path)
        && let Some(entry) = controller.wav_entries.entry(index)
    {
        return Ok(entry.looped);
    }
    let db = controller
        .database_for(source)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    db.looped_for_path(relative_path)
        .map_err(|err| format!("Failed to read database: {err}"))?
        .ok_or_else(|| "Sample not found in database".to_string())
}

/// Resolve the last played timestamp for a wav entry, if available.
pub(super) fn sample_last_played_for(
    controller: &mut AppController,
    source: &SampleSource,
    relative_path: &Path,
) -> Result<Option<i64>, String> {
    if let Some(cache) = controller.cache.wav.entries.get(&source.id)
        && let Some(index) = cache.lookup.get(relative_path).copied()
        && let Some(entry) = cache.entry(index)
    {
        return Ok(entry.last_played_at);
    }
    if controller.selection_state.ctx.selected_source.as_ref() == Some(&source.id)
        && let Some(index) = controller.wav_index_for_path(relative_path)
        && let Some(entry) = controller.wav_entries.entry(index)
    {
        return Ok(entry.last_played_at);
    }
    let db = controller
        .database_for(source)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    db.last_played_at_for_path(relative_path)
        .map_err(|err| format!("Failed to read database: {err}"))
}
