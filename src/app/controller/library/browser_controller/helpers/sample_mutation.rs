//! Browser sample delete, rename, and auto-rename execution helpers.

use super::*;
use crate::app::controller::library::analysis_jobs::db::telemetry;
use std::thread::sleep;
use std::time::Duration;

const SAMPLE_RENAME_DB_RETRIES: usize = 4;
const SAMPLE_RENAME_DB_RETRY_DELAY: Duration = Duration::from_millis(50);

impl BrowserController<'_> {
    /// Delete the resolved browser sample immediately in the current thread.
    pub(crate) fn try_delete_browser_sample_ctx(
        &mut self,
        ctx: &TriageSampleContext,
    ) -> Result<(), String> {
        if self.controller.warn_if_retained_delete_path_busy(
            &ctx.source.id,
            &ctx.entry.relative_path,
            "deleting",
        ) {
            return Ok(());
        }
        std::fs::remove_file(&ctx.absolute_path)
            .map_err(|err| format!("Failed to delete file: {err}"))?;
        let db = self
            .database_for(&ctx.source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        db.remove_file(&ctx.entry.relative_path)
            .map_err(|err| format!("Failed to drop database row: {err}"))?;
        self.prune_cached_sample(&ctx.source, &ctx.entry.relative_path);
        self.set_status(
            format!("Deleted {}", ctx.entry.relative_path.display()),
            StatusTone::Info,
        );
        Ok(())
    }

    /// Rename the browser row at `row` while preserving playback resume details.
    pub(crate) fn try_rename_browser_sample(
        &mut self,
        row: usize,
        new_name: &str,
    ) -> Result<(), String> {
        let ctx = self.resolve_browser_sample(row)?;
        if self.controller.warn_if_retained_delete_path_busy(
            &ctx.source.id,
            &ctx.entry.relative_path,
            "renaming",
        ) {
            return Ok(());
        }
        let tag = self.sample_tag_for(&ctx.source, &ctx.entry.relative_path)?;
        let full_name = self.name_with_preserved_extension(&ctx.entry.relative_path, new_name)?;
        let new_relative = self.validate_new_sample_name_in_parent(
            &ctx.entry.relative_path,
            &ctx.source.root,
            &full_name,
        )?;
        if self.runtime.jobs.file_ops_in_progress() {
            return Err("File operation already in progress".to_string());
        }
        let was_playing = self.is_playing();
        let was_looping = self.ui.waveform.loop_enabled;
        let playhead_position = self.ui.waveform.playhead.position;
        let fallback_sound_type = ctx.entry.sound_type;
        let is_currently_loaded = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                audio.source_id == ctx.source.id && audio.relative_path == ctx.entry.relative_path
            });
        if cfg!(test) {
            self.begin_pending_file_mutation(&ctx.source.id, [ctx.entry.relative_path.clone()]);
            let result = run_sample_rename_job(
                ctx,
                new_relative,
                tag,
                fallback_sound_type,
                is_currently_loaded && was_playing,
                was_looping,
                playhead_position
                    .is_finite()
                    .then(|| f64::from(playhead_position.clamp(0.0, 1.0))),
                Arc::new(AtomicBool::new(false)),
            );
            self.apply_file_op_result(FileOpResult::SampleRename(result));
            return Ok(());
        }
        self.begin_pending_file_mutation(&ctx.source.id, [ctx.entry.relative_path.clone()]);
        self.set_status(
            format!("Renaming {}...", ctx.entry.relative_path.display()),
            StatusTone::Busy,
        );
        let pending_source_id = ctx.source.id.clone();
        let pending_path = ctx.entry.relative_path.clone();
        if let Err(err) = self.runtime.jobs.begin_one_shot_file_op(move |cancel| {
            FileOpResult::SampleRename(run_sample_rename_job(
                ctx,
                new_relative,
                tag,
                fallback_sound_type,
                is_currently_loaded && was_playing,
                was_looping,
                playhead_position
                    .is_finite()
                    .then(|| f64::from(playhead_position.clamp(0.0, 1.0))),
                cancel,
            ))
        }) {
            self.finish_pending_file_mutation(&pending_source_id, [pending_path]);
            return Err(err);
        }
        Ok(())
    }
}

/// Request payload for one browser auto-rename target.
pub(crate) struct SampleAutoRenameRequest {
    pub(crate) old_relative: PathBuf,
    pub(crate) new_relative: PathBuf,
    pub(crate) tag: crate::sample_sources::Rating,
    /// Sound type inferred during controller-side request preparation when the
    /// source DB row does not already store one.
    pub(crate) sound_type: Option<crate::sample_sources::SampleSoundType>,
    pub(crate) resume_playback: bool,
    pub(crate) resume_looped: bool,
    pub(crate) resume_start_override: Option<f64>,
}

fn run_sample_rename_job(
    ctx: TriageSampleContext,
    new_relative: PathBuf,
    tag: crate::sample_sources::Rating,
    fallback_sound_type: Option<crate::sample_sources::SampleSoundType>,
    resume_playback: bool,
    resume_looped: bool,
    resume_start_override: Option<f64>,
    cancel: Arc<AtomicBool>,
) -> SampleRenameResult {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return SampleRenameResult {
            source_id: ctx.source.id,
            old_relative: ctx.entry.relative_path,
            new_relative,
            entry: None,
            resume_playback,
            resume_looped,
            resume_start_override,
            result: Err(String::from("Rename cancelled")),
        };
    }
    let old_relative = ctx.entry.relative_path.clone();
    let result = perform_sample_rename(
        &ctx.source,
        &ctx.absolute_path,
        &old_relative,
        &new_relative,
        tag,
        ctx.entry.looped,
        ctx.entry.locked,
        ctx.entry.last_played_at,
        fallback_sound_type,
    );
    SampleRenameResult {
        source_id: ctx.source.id,
        old_relative,
        new_relative,
        entry: result.as_ref().ok().cloned(),
        resume_playback,
        resume_looped,
        resume_start_override,
        result: result.map(|_| ()),
    }
}

/// Execute a background browser auto-rename batch, collecting renamed, skipped, and failed items.
pub(crate) fn run_sample_auto_rename_job(
    source: SampleSource,
    requests: Vec<SampleAutoRenameRequest>,
    cancel: Arc<AtomicBool>,
) -> SampleAutoRenameResult {
    let requested_paths = requests
        .iter()
        .map(|request| request.old_relative.clone())
        .collect::<Vec<_>>();
    let mut renamed = Vec::new();
    let mut skipped = Vec::new();
    let mut errors = Vec::new();
    for request in requests {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            errors.push((request.old_relative, String::from("Rename cancelled")));
            break;
        }
        let old_absolute = source.root.join(&request.old_relative);
        if request.old_relative == request.new_relative {
            skipped.push((
                request.old_relative,
                String::from("Already matches auto-rename format"),
            ));
            continue;
        }
        match perform_sample_rename(
            &source,
            &old_absolute,
            &request.old_relative,
            &request.new_relative,
            request.tag,
            false,
            false,
            None,
            request.sound_type,
        ) {
            Ok(entry) => renamed.push(SampleAutoRenameSuccess {
                old_relative: request.old_relative,
                new_relative: request.new_relative,
                entry,
                resume_playback: request.resume_playback,
                resume_looped: request.resume_looped,
                resume_start_override: request.resume_start_override,
            }),
            Err(err) if err.contains("already exists") => skipped.push((request.old_relative, err)),
            Err(err) => errors.push((request.old_relative, err)),
        }
    }
    SampleAutoRenameResult {
        source_id: source.id,
        requested_paths,
        renamed,
        skipped,
        errors,
    }
}

pub(super) fn perform_sample_rename(
    source: &SampleSource,
    old_absolute: &Path,
    old_relative: &Path,
    new_relative: &Path,
    tag: crate::sample_sources::Rating,
    fallback_looped: bool,
    fallback_locked: bool,
    fallback_last_played_at: Option<i64>,
    fallback_sound_type: Option<crate::sample_sources::SampleSoundType>,
) -> Result<WavEntry, String> {
    let new_absolute = source.root.join(new_relative);
    std::fs::rename(old_absolute, &new_absolute)
        .map_err(|err| format!("Failed to rename file: {err}"))?;
    let result = persist_sample_rename_with_retry(
        source,
        old_relative,
        new_relative,
        &new_absolute,
        tag,
        fallback_looped,
        fallback_locked,
        fallback_last_played_at,
        fallback_sound_type,
    );
    match result {
        Ok(entry) => Ok(entry),
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
    source: &SampleSource,
    old_relative: &Path,
    new_relative: &Path,
    new_absolute: &Path,
    tag: crate::sample_sources::Rating,
    fallback_looped: bool,
    fallback_locked: bool,
    fallback_last_played_at: Option<i64>,
    fallback_sound_type: Option<crate::sample_sources::SampleSoundType>,
) -> Result<WavEntry, String> {
    let mut last_err = None;
    for attempt in 0..SAMPLE_RENAME_DB_RETRIES {
        match persist_sample_rename_once(
            source,
            old_relative,
            new_relative,
            new_absolute,
            tag,
            fallback_looped,
            fallback_locked,
            fallback_last_played_at,
            fallback_sound_type,
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
    source: &SampleSource,
    old_relative: &Path,
    new_relative: &Path,
    new_absolute: &Path,
    tag: crate::sample_sources::Rating,
    fallback_looped: bool,
    fallback_locked: bool,
    fallback_last_played_at: Option<i64>,
    fallback_sound_type: Option<crate::sample_sources::SampleSoundType>,
) -> Result<WavEntry, String> {
    let (file_size, modified_ns) = file_metadata(new_absolute)?;
    let db = crate::sample_sources::SourceDatabase::open(&source.root)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    let last_played_at = db
        .last_played_at_for_path(old_relative)
        .map_err(|err| format!("Failed to load playback age: {err}"))?;
    let looped = db
        .looped_for_path(old_relative)
        .map_err(|err| format!("Failed to load loop marker: {err}"))?
        .unwrap_or(fallback_looped);
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
        .map_err(|err| format!("Failed to load custom tag: {err}"))?;
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
    batch
        .set_locked(new_relative, locked)
        .map_err(|err| format!("Failed to copy keep lock: {err}"))?;
    batch
        .set_sound_type(new_relative, sound_type)
        .map_err(|err| format!("Failed to copy sound type: {err}"))?;
    batch
        .set_user_tag(new_relative, user_tag.as_deref())
        .map_err(|err| format!("Failed to copy custom tag: {err}"))?;
    if let Some(last_played_at) = last_played_at {
        batch
            .set_last_played_at(new_relative, last_played_at)
            .map_err(|err| format!("Failed to copy playback age: {err}"))?;
    }
    batch
        .commit()
        .map_err(|err| format!("Failed to save rename: {err}"))?;
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
        user_tag,
    })
}

fn is_busy_lock_error(err: &str) -> bool {
    let lowered = err.to_ascii_lowercase();
    lowered.contains("busy") || lowered.contains("locked")
}
