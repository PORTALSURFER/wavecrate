use super::*;
use crate::app::controller::library::wav_io;
use std::path::{Path, PathBuf};

/// Normalize a wav file and return updated metadata + tag.
pub(super) fn normalize_and_save_for_path(
    controller: &mut AppController,
    source: &SampleSource,
    relative_path: &Path,
    absolute_path: &Path,
) -> Result<(u64, i64, crate::sample_sources::Rating), String> {
    let (mut samples, spec) = wav_io::read_samples_for_normalization(absolute_path)?;
    if samples.is_empty() {
        return Err("No audio data to normalize".into());
    }
    crate::analysis::audio::normalize_peak_in_place(&mut samples);

    let target_spec = hound::WavSpec {
        channels: spec.channels.max(1),
        sample_rate: spec.sample_rate.max(1),
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    wav_io::write_normalized_wav(absolute_path, &samples, target_spec)?;

    let (file_size, modified_ns) = wav_io::file_metadata(absolute_path)?;
    let tag = controller.sample_tag_for(source, relative_path)?;
    Ok((file_size, modified_ns, tag))
}

/// Persist a rename or path change in the per-source database.
pub(super) fn rewrite_db_entry_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    old_relative: &Path,
    new_relative: &Path,
    file_size: u64,
    modified_ns: i64,
    tag: crate::sample_sources::Rating,
) -> Result<(), String> {
    let db = controller
        .database_for(source)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    let last_played_at = db
        .last_played_at_for_path(old_relative)
        .map_err(|err| format!("Failed to load playback age: {err}"))?;
    let looped = db
        .looped_for_path(old_relative)
        .map_err(|err| format!("Failed to load loop marker: {err}"))?
        .unwrap_or(false);
    let mut batch = db
        .write_batch()
        .map_err(|err| format!("Failed to start database update: {err}"))?;
    batch
        .remove_file(old_relative)
        .map_err(|err| format!("Failed to drop old entry: {err}"))?;
    batch
        .upsert_file(new_relative, file_size, modified_ns)
        .map_err(|err| format!("Failed to register renamed file: {err}"))?;
    batch
        .set_tag(new_relative, tag)
        .map_err(|err| format!("Failed to copy tag: {err}"))?;
    batch
        .set_looped(new_relative, looped)
        .map_err(|err| format!("Failed to copy loop marker: {err}"))?;
    if let Some(last_played_at) = last_played_at {
        batch
            .set_last_played_at(new_relative, last_played_at)
            .map_err(|err| format!("Failed to copy playback age: {err}"))?;
    }
    batch
        .commit()
        .map_err(|err| format!("Failed to save rename: {err}"))
}

/// Upsert file metadata into the source database.
pub(super) fn upsert_metadata_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    relative_path: &Path,
    file_size: u64,
    modified_ns: i64,
) -> Result<(), String> {
    let db = controller
        .database_for(source)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    db.upsert_file(relative_path, file_size, modified_ns)
        .map_err(|err| format!("Failed to refresh metadata: {err}"))
}

/// Validate and sanitize a renamed file while preserving its extension.
pub(super) fn name_with_preserved_extension(
    current_relative: &Path,
    new_name: &str,
) -> Result<String, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    let Some(ext) = current_relative.extension().and_then(|ext| ext.to_str()) else {
        return Ok(trimmed.to_string());
    };
    let ext_lower = ext.to_ascii_lowercase();
    let should_strip_suffix = |suffix: &str| -> bool {
        let suffix_lower = suffix.to_ascii_lowercase();
        suffix_lower == ext_lower
            || matches!(
                suffix_lower.as_str(),
                "wav" | "wave" | "flac" | "aif" | "aiff" | "mp3" | "ogg" | "opus"
            )
    };
    let stem = if let Some((stem, suffix)) = trimmed.rsplit_once('.') {
        if !stem.is_empty() && should_strip_suffix(suffix) {
            stem
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    let stem = stem.trim_end_matches('.');
    if stem.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }
    Ok(format!("{stem}.{ext}"))
}

/// Validate that a new file name is safe and available in its parent folder.
pub(super) fn validate_new_sample_name_in_parent(
    relative_path: &Path,
    root: &Path,
    new_name: &str,
) -> Result<PathBuf, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if trimmed.contains(['/', '\\']) {
        return Err("Name cannot contain path separators".into());
    }
    let parent = relative_path.parent().unwrap_or(Path::new(""));
    let new_relative = parent.join(trimmed);
    let new_absolute = root.join(&new_relative);
    if new_absolute.exists() {
        return Err(format!(
            "A file named {} already exists",
            new_relative.display()
        ));
    }
    Ok(new_relative)
}

/// Validate that a moved file name is safe and available in the target folder.
pub(super) fn validate_new_sample_name_in_folder(
    current_relative: &Path,
    root: &Path,
    target_folder: &Path,
    new_name: &str,
) -> Result<PathBuf, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if trimmed.contains(['/', '\\']) {
        return Err("Name cannot contain path separators".into());
    }
    let file_name = name_with_preserved_extension(current_relative, trimmed)?;
    let new_relative = folder_relative_path(target_folder, &file_name);
    if root.join(&new_relative).exists() && new_relative != current_relative {
        return Err(format!(
            "A file named {} already exists",
            new_relative.display()
        ));
    }
    Ok(new_relative)
}

/// Suggest the next available numbered name for moving a file into a target folder.
pub(super) fn suggest_numbered_sample_name_in_folder(
    current_relative: &Path,
    root: &Path,
    target_folder: &Path,
) -> Result<String, String> {
    let stem = current_relative
        .file_stem()
        .and_then(|name| name.to_str())
        .or_else(|| current_relative.file_name().and_then(|name| name.to_str()))
        .unwrap_or("sample");
    for index in 1..=999 {
        let candidate = format!("{stem}_{index:03}");
        let file_name = name_with_preserved_extension(current_relative, &candidate)?;
        let target_relative = folder_relative_path(target_folder, &file_name);
        if !root.join(target_relative).exists() {
            return Ok(candidate);
        }
    }
    Err("Unable to find a unique destination name".into())
}

fn folder_relative_path(target_folder: &Path, file_name: &str) -> PathBuf {
    if target_folder.as_os_str().is_empty() {
        PathBuf::from(file_name)
    } else {
        target_folder.join(file_name)
    }
}

/// Update all cached structures after a file path or metadata change.
pub(super) fn update_cached_entry(
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
pub(super) fn insert_cached_entry(
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
pub(super) fn update_selection_paths(
    controller: &mut AppController,
    source: &SampleSource,
    old_path: &Path,
    new_path: &Path,
) {
    controller.update_compare_anchor_path(&source.id, old_path, new_path);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn numbered_folder_move_name_starts_at_001() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("dest")).unwrap();
        fs::write(root.join("dest/one.wav"), b"taken").unwrap();

        let suggested =
            suggest_numbered_sample_name_in_folder(Path::new("one.wav"), root, Path::new("dest"))
                .unwrap();

        assert_eq!(suggested, "one_001");
    }

    #[test]
    fn numbered_folder_move_name_skips_taken_suffixes() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("dest")).unwrap();
        fs::write(root.join("dest/one.wav"), b"taken").unwrap();
        fs::write(root.join("dest/one_001.wav"), b"taken").unwrap();

        let suggested =
            suggest_numbered_sample_name_in_folder(Path::new("one.wav"), root, Path::new("dest"))
                .unwrap();

        assert_eq!(suggested, "one_002");
    }

    #[test]
    fn folder_move_name_validation_preserves_extension() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("dest")).unwrap();

        let validated = validate_new_sample_name_in_folder(
            Path::new("one.wav"),
            root,
            Path::new("dest"),
            "one_001",
        )
        .unwrap();

        assert_eq!(validated, PathBuf::from("dest/one_001.wav"));
    }

    #[test]
    fn numbered_folder_move_name_handles_extensionless_files() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("dest")).unwrap();
        fs::write(root.join("dest/loop"), b"taken").unwrap();

        let suggested =
            suggest_numbered_sample_name_in_folder(Path::new("loop"), root, Path::new("dest"))
                .unwrap();

        assert_eq!(suggested, "loop_001");
    }
}
