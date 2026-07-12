use super::*;
use crate::app::controller::library::analysis_jobs::db::telemetry;
use std::thread::sleep;
use tracing::info;

pub(in crate::app::controller::library::browser_controller::helpers) fn perform_sample_rename(
    source: &SampleSource,
    old_absolute: &Path,
    old_relative: &Path,
    new_relative: &Path,
    tag: crate::sample_sources::Rating,
    looped_metadata: RenameLoopedMetadata,
    fallback_locked: bool,
    fallback_last_played_at: Option<i64>,
    fallback_sound_type: Option<crate::sample_sources::SampleSoundType>,
    fallback_user_tag: Option<String>,
    fallback_tag_named: Option<bool>,
) -> Result<WavEntry, String> {
    let db = crate::sample_sources::SourceDatabase::open_for_background_job(&source.root)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    perform_sample_rename_with_db(
        source,
        &db,
        old_absolute,
        old_relative,
        new_relative,
        tag,
        looped_metadata,
        fallback_locked,
        fallback_last_played_at,
        fallback_sound_type,
        fallback_user_tag,
        fallback_tag_named,
    )
}

pub(in crate::app::controller::library::browser_controller::helpers::sample_mutation) fn perform_sample_rename_with_db(
    source: &SampleSource,
    db: &crate::sample_sources::SourceDatabase,
    old_absolute: &Path,
    old_relative: &Path,
    new_relative: &Path,
    tag: crate::sample_sources::Rating,
    looped_metadata: RenameLoopedMetadata,
    fallback_locked: bool,
    fallback_last_played_at: Option<i64>,
    fallback_sound_type: Option<crate::sample_sources::SampleSoundType>,
    fallback_user_tag: Option<String>,
    fallback_tag_named: Option<bool>,
) -> Result<WavEntry, String> {
    let new_absolute = source.root.join(new_relative);
    std::fs::rename(old_absolute, &new_absolute)
        .map_err(|err| format!("Failed to rename file: {err}"))?;
    let result = persist_sample_rename_with_retry(
        db,
        source,
        old_relative,
        new_relative,
        &new_absolute,
        tag,
        looped_metadata,
        fallback_locked,
        fallback_last_played_at,
        fallback_sound_type,
        fallback_user_tag,
        fallback_tag_named,
    );
    match result {
        Ok(entry) => {
            crate::app::controller::library::source_write_priority::record_completed_browser_rename(
                &source.id,
                old_relative,
                new_relative,
            );
            Ok(entry)
        }
        Err(err) => rollback_sample_rename(old_absolute, &new_absolute, err),
    }
}

/// Restore the original filename when the DB rewrite fails after the filesystem rename.
fn rollback_sample_rename(
    old_absolute: &Path,
    new_absolute: &Path,
    message: String,
) -> Result<WavEntry, String> {
    std::fs::rename(new_absolute, old_absolute)
        .map_err(|err| format!("{message}; rollback failed: {err}"))?;
    Err(message)
}

/// Retry the source-db rewrite briefly so a just-queued metadata write can
/// finish before the rename gives up and rolls the filesystem change back.
fn persist_sample_rename_with_retry(
    db: &crate::sample_sources::SourceDatabase,
    source: &SampleSource,
    old_relative: &Path,
    new_relative: &Path,
    new_absolute: &Path,
    tag: crate::sample_sources::Rating,
    looped_metadata: RenameLoopedMetadata,
    fallback_locked: bool,
    fallback_last_played_at: Option<i64>,
    fallback_sound_type: Option<crate::sample_sources::SampleSoundType>,
    fallback_user_tag: Option<String>,
    fallback_tag_named: Option<bool>,
) -> Result<WavEntry, String> {
    let mut last_err = None;
    for attempt in 0..SAMPLE_RENAME_DB_RETRIES {
        match persist_sample_rename_once(
            db,
            source,
            old_relative,
            new_relative,
            new_absolute,
            tag,
            looped_metadata,
            fallback_locked,
            fallback_last_played_at,
            fallback_sound_type,
            fallback_user_tag.clone(),
            fallback_tag_named,
        ) {
            Ok(entry) => return Ok(entry),
            Err(err) if attempt + 1 < SAMPLE_RENAME_DB_RETRIES && is_busy_lock_error(&err) => {
                telemetry::record_retry(
                    "browser_sample_rename",
                    &source.root,
                    attempt + 1,
                    SAMPLE_RENAME_DB_RETRIES,
                    SAMPLE_RENAME_DB_RETRY_DELAY,
                    &err,
                );
                last_err = Some(err);
                sleep(SAMPLE_RENAME_DB_RETRY_DELAY);
            }
            Err(err) => return Err(err),
        }
    }
    Err(last_err.unwrap_or_else(|| "Rename retries exhausted".to_string()))
}

fn persist_sample_rename_once(
    db: &crate::sample_sources::SourceDatabase,
    source: &SampleSource,
    old_relative: &Path,
    new_relative: &Path,
    new_absolute: &Path,
    tag: crate::sample_sources::Rating,
    looped_metadata: RenameLoopedMetadata,
    fallback_locked: bool,
    fallback_last_played_at: Option<i64>,
    fallback_sound_type: Option<crate::sample_sources::SampleSoundType>,
    fallback_user_tag: Option<String>,
    fallback_tag_named: Option<bool>,
) -> Result<WavEntry, String> {
    let (file_size, modified_ns) = file_metadata(new_absolute)?;
    let last_played_at = db
        .last_played_at_for_path(old_relative)
        .map_err(|err| format!("Failed to load playback age: {err}"))?;
    let last_curated_at = db
        .last_curated_at_for_path(old_relative)
        .map_err(|err| format!("Failed to load curation timestamp: {err}"))?;
    let db_looped = db
        .looped_for_path(old_relative)
        .map_err(|err| format!("Failed to load loop marker: {err}"))?;
    let looped = looped_metadata.resolved(db_looped);
    let locked = db
        .locked_for_path(old_relative)
        .map_err(|err| format!("Failed to load lock marker: {err}"))?
        .unwrap_or(fallback_locked);
    let sound_type = db
        .sound_type_for_path(old_relative)
        .map_err(|err| format!("Failed to load sound type: {err}"))?
        .or(fallback_sound_type);
    let user_tag = db
        .user_tag_for_path(old_relative)
        .map_err(|err| format!("Failed to load custom tag: {err}"))?
        .or(fallback_user_tag);
    let tag_named = db
        .tag_named_for_path(old_relative)
        .map_err(|err| format!("Failed to load tag-name marker: {err}"))?
        .or(fallback_tag_named)
        .unwrap_or(false);
    persistence::persist_rename_metadata(
        db,
        old_relative,
        new_relative,
        tag,
        looped,
        locked,
        sound_type,
        user_tag.as_deref(),
        tag_named,
        last_played_at,
        file_size,
        modified_ns,
    )?;
    log_looped_provenance(
        source,
        old_relative,
        new_relative,
        looped_metadata,
        db_looped,
        looped,
    );
    Ok(WavEntry {
        relative_path: new_relative.to_path_buf(),
        file_size,
        modified_ns,
        content_hash: None,
        tag,
        looped,
        sound_type,
        locked,
        missing: false,
        last_played_at: last_played_at.or(fallback_last_played_at),
        last_curated_at,
        user_tag,
        normal_tags: db
            .tags_for_path(new_relative)
            .map_err(|err| format!("Failed to reload normal tags: {err}"))?
            .into_iter()
            .map(|tag| tag.display_label)
            .collect(),
        tag_named,
    })
}

fn log_looped_provenance(
    source: &SampleSource,
    old_relative: &Path,
    new_relative: &Path,
    looped_metadata: RenameLoopedMetadata,
    db_looped: Option<bool>,
    looped: bool,
) {
    info!(
        source_id = %source.id,
        old_path = %old_relative.display(),
        new_path = %new_relative.display(),
        request_looped = looped_metadata.request_value(),
        db_looped = ?db_looped,
        final_looped = looped,
        "auto rename: persisted loop metadata provenance"
    );
    provenance::record_rename_looped_provenance(
        old_relative,
        new_relative,
        looped_metadata.request_value(),
        db_looped,
        looped,
    );
}

fn is_busy_lock_error(err: &str) -> bool {
    let lowered = err.to_ascii_lowercase();
    lowered.contains("busy") || lowered.contains("locked")
}
