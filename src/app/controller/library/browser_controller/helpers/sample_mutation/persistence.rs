use super::*;

pub(super) fn persist_rename_metadata(
    db: &crate::sample_sources::SourceDatabase,
    old_relative: &Path,
    new_relative: &Path,
    tag: crate::sample_sources::Rating,
    looped: bool,
    locked: bool,
    sound_type: Option<crate::sample_sources::SampleSoundType>,
    user_tag: Option<&str>,
    tag_named: bool,
    last_played_at: Option<i64>,
    file_size: u64,
    modified_ns: i64,
) -> Result<(), String> {
    let mut batch = db
        .write_batch()
        .map_err(|err| format!("Failed to start database update: {err}"))?;
    let normal_tags = batch
        .tag_labels_for_path(old_relative)
        .map_err(|err| format!("Failed to load normal tags: {err}"))?;
    batch
        .upsert_file(new_relative, file_size, modified_ns)
        .map_err(|err| format!("Failed to register renamed file: {err}"))?;
    batch
        .set_tag(new_relative, tag)
        .map_err(|err| format!("Failed to copy tag: {err}"))?;
    batch
        .set_looped(new_relative, looped)
        .map_err(|err| format!("Failed to copy loop marker: {err}"))?;
    batch
        .set_locked(new_relative, locked)
        .map_err(|err| format!("Failed to copy keep lock: {err}"))?;
    batch
        .set_sound_type(new_relative, sound_type)
        .map_err(|err| format!("Failed to copy sound type: {err}"))?;
    batch
        .set_user_tag(new_relative, user_tag)
        .map_err(|err| format!("Failed to copy custom tag: {err}"))?;
    batch
        .set_tag_named(new_relative, tag_named)
        .map_err(|err| format!("Failed to copy tag-name marker: {err}"))?;
    if let Some(last_played_at) = last_played_at {
        batch
            .set_last_played_at(new_relative, last_played_at)
            .map_err(|err| format!("Failed to copy playback age: {err}"))?;
    }
    batch
        .replace_tags_for_path(new_relative, &normal_tags)
        .map_err(|err| format!("Failed to copy normal tags: {err}"))?;
    batch
        .remove_file(old_relative)
        .map_err(|err| format!("Failed to drop old entry: {err}"))?;
    batch
        .remap_analysis_sample_identity(old_relative, new_relative)
        .map_err(|err| format!("Failed to preserve analysis artifacts: {err}"))?;
    batch
        .commit()
        .map_err(|err| format!("Failed to save rename: {err}"))
}
