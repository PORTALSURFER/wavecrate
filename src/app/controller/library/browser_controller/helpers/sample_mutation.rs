//! Browser sample delete, rename, and auto-rename execution helpers.

use super::*;
use crate::app::controller::library::analysis_jobs::db::telemetry;
use std::thread::sleep;
use std::time::Duration;
use tracing::info;

#[cfg(test)]
use std::sync::{LazyLock, Mutex};

#[cfg(not(test))]
const SAMPLE_RENAME_DB_RETRIES: usize = SAMPLE_RENAME_DB_RETRIES_PRODUCTION;
#[cfg(test)]
const SAMPLE_RENAME_DB_RETRIES: usize = 4;
#[cfg(not(test))]
const SAMPLE_RENAME_DB_RETRY_DELAY: Duration = SAMPLE_RENAME_DB_RETRY_DELAY_PRODUCTION;
#[cfg(test)]
const SAMPLE_RENAME_DB_RETRY_DELAY: Duration = Duration::from_millis(50);
/// Production retry count for browser sample rename DB rewrites.
pub(super) const SAMPLE_RENAME_DB_RETRIES_PRODUCTION: usize = 80;
/// Production retry delay for browser sample rename DB rewrites.
pub(super) const SAMPLE_RENAME_DB_RETRY_DELAY_PRODUCTION: Duration = Duration::from_millis(100);

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RenameLoopedProvenanceLog {
    pub(super) old_relative: PathBuf,
    pub(super) new_relative: PathBuf,
    pub(super) request_looped: bool,
    pub(super) db_looped: Option<bool>,
    pub(super) final_looped: bool,
}

#[cfg(test)]
static RENAME_LOOPED_PROVENANCE_LOGS: LazyLock<Mutex<Vec<RenameLoopedProvenanceLog>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[cfg(test)]
pub(super) fn take_rename_looped_provenance_logs_for_tests() -> Vec<RenameLoopedProvenanceLog> {
    std::mem::take(&mut *RENAME_LOOPED_PROVENANCE_LOGS.lock().unwrap())
}

impl BrowserController<'_> {
    /// Move the resolved browser sample into the configured trash folder.
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
        let moved = self.move_samples_to_configured_trash_detailed(
            vec![(ctx.source.clone(), ctx.entry.clone())],
            None,
        );
        if moved.moved_count() > 0 {
            return Ok(());
        }
        let err = moved
            .errors
            .last()
            .cloned()
            .unwrap_or_else(|| self.ui.status.text.clone());
        Err(err)
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
        let intent_key = BrowserRenameIntentKey::new(
            ctx.source.id.clone(),
            vec![(ctx.entry.relative_path.clone(), new_relative.clone())],
        );
        if self.runtime.jobs.file_ops_in_progress() {
            if self
                .runtime
                .source_lane
                .mutations
                .browser_rename_intent_is_active(&intent_key)
            {
                self.set_file_op_status("Rename already in progress...", StatusTone::Busy);
                return Ok(());
            }
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
            self.runtime
                .source_lane
                .mutations
                .begin_browser_rename_intent(intent_key);
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
        self.runtime
            .source_lane
            .mutations
            .begin_browser_rename_intent(intent_key);
        self.begin_pending_file_mutation(&ctx.source.id, [ctx.entry.relative_path.clone()]);
        self.set_file_op_status(
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
            self.runtime
                .source_lane
                .mutations
                .clear_browser_rename_intent();
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
    pub(crate) looped: bool,
    pub(crate) locked: bool,
    /// Sound type inferred during controller-side request preparation when the
    /// source DB row does not already store one.
    pub(crate) sound_type: Option<crate::sample_sources::SampleSoundType>,
    pub(crate) user_tag: Option<String>,
    pub(crate) tag_named: bool,
    pub(crate) last_played_at: Option<i64>,
    pub(crate) resume_playback: bool,
    pub(crate) resume_looped: bool,
    pub(crate) resume_start_override: Option<f64>,
}

#[derive(Clone, Copy)]
pub(super) enum RenameLoopedMetadata {
    DbOrFallback(bool),
    RequestSnapshot(bool),
}

impl RenameLoopedMetadata {
    fn request_value(self) -> bool {
        match self {
            RenameLoopedMetadata::DbOrFallback(looped)
            | RenameLoopedMetadata::RequestSnapshot(looped) => looped,
        }
    }

    fn resolved(self, db_looped: Option<bool>) -> bool {
        match self {
            RenameLoopedMetadata::DbOrFallback(fallback_looped) => {
                db_looped.unwrap_or(fallback_looped)
            }
            RenameLoopedMetadata::RequestSnapshot(looped) => looped,
        }
    }
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
        RenameLoopedMetadata::DbOrFallback(ctx.entry.looped),
        ctx.entry.locked,
        ctx.entry.last_played_at,
        fallback_sound_type,
        ctx.entry.user_tag.clone(),
        None,
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
    progress: Option<FileOpProgressSender>,
) -> SampleAutoRenameResult {
    #[cfg(test)]
    let started_at = std::time::Instant::now();
    let source_id = source.id.clone();
    let requested_paths = requests
        .iter()
        .map(|request| request.old_relative.clone())
        .collect::<Vec<_>>();
    let db = match crate::sample_sources::SourceDatabase::open_with_role(
        &source.root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    ) {
        Ok(db) => db,
        Err(err) => {
            return SampleAutoRenameResult {
                source_id,
                requested_paths: requested_paths.clone(),
                renamed: Vec::new(),
                skipped: Vec::new(),
                errors: requested_paths
                    .into_iter()
                    .map(|path| (path, format!("Database unavailable: {err}")))
                    .collect(),
            };
        }
    };
    let mut renamed = Vec::new();
    let mut skipped = Vec::new();
    let mut errors = Vec::new();
    for (index, request) in requests.into_iter().enumerate() {
        let completed = index;
        emit_auto_rename_item_progress(
            progress.as_ref(),
            completed,
            None,
            SampleAutoRenameProgress::Active {
                old_relative: request.old_relative.clone(),
            },
        );
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            emit_auto_rename_item_progress(
                progress.as_ref(),
                completed,
                Some(format!("Cancelled at {}", request.old_relative.display())),
                SampleAutoRenameProgress::Failed {
                    old_relative: request.old_relative.clone(),
                    error: String::from("Rename cancelled"),
                },
            );
            errors.push((request.old_relative, String::from("Rename cancelled")));
            break;
        }
        let old_absolute = source.root.join(&request.old_relative);
        if request.old_relative == request.new_relative {
            match mark_sample_tag_named_with_db(&db, &request.old_relative, request.tag_named)
                .and_then(|entry| {
                    entry.ok_or_else(|| {
                        format!("Sample not found: {}", request.old_relative.display())
                    })
                }) {
                Ok(entry) => {
                    emit_auto_rename_item_progress(
                        progress.as_ref(),
                        completed + 1,
                        Some(format!("Renamed {}", request.old_relative.display())),
                        SampleAutoRenameProgress::Completed {
                            old_relative: request.old_relative.clone(),
                            new_relative: request.new_relative.clone(),
                        },
                    );
                    renamed.push(SampleAutoRenameSuccess {
                        old_relative: request.old_relative.clone(),
                        new_relative: request.new_relative.clone(),
                        entry,
                        resume_playback: request.resume_playback,
                        resume_looped: request.resume_looped,
                        resume_start_override: request.resume_start_override,
                    });
                }
                Err(err) => {
                    emit_auto_rename_item_progress(
                        progress.as_ref(),
                        completed + 1,
                        Some(format!("Failed {}", request.old_relative.display())),
                        SampleAutoRenameProgress::Failed {
                            old_relative: request.old_relative.clone(),
                            error: err.clone(),
                        },
                    );
                    errors.push((request.old_relative, err));
                }
            }
            continue;
        }
        match perform_sample_rename_with_db(
            &source,
            &db,
            &old_absolute,
            &request.old_relative,
            &request.new_relative,
            request.tag,
            RenameLoopedMetadata::RequestSnapshot(request.looped),
            request.locked,
            request.last_played_at,
            request.sound_type,
            request.user_tag,
            Some(request.tag_named),
        ) {
            Ok(entry) => {
                emit_auto_rename_item_progress(
                    progress.as_ref(),
                    completed + 1,
                    Some(format!("Renamed {}", request.new_relative.display())),
                    SampleAutoRenameProgress::Completed {
                        old_relative: request.old_relative.clone(),
                        new_relative: request.new_relative.clone(),
                    },
                );
                renamed.push(SampleAutoRenameSuccess {
                    old_relative: request.old_relative,
                    new_relative: request.new_relative,
                    entry,
                    resume_playback: request.resume_playback,
                    resume_looped: request.resume_looped,
                    resume_start_override: request.resume_start_override,
                });
            }
            Err(err) if err.contains("already exists") => {
                emit_auto_rename_item_progress(
                    progress.as_ref(),
                    completed + 1,
                    Some(format!("Skipped {}", request.old_relative.display())),
                    SampleAutoRenameProgress::Skipped {
                        old_relative: request.old_relative.clone(),
                        reason: err.clone(),
                    },
                );
                skipped.push((request.old_relative, err));
            }
            Err(err) => {
                emit_auto_rename_item_progress(
                    progress.as_ref(),
                    completed + 1,
                    Some(format!("Failed {}", request.old_relative.display())),
                    SampleAutoRenameProgress::Failed {
                        old_relative: request.old_relative.clone(),
                        error: err.clone(),
                    },
                );
                errors.push((request.old_relative, err));
            }
        }
    }
    let result = SampleAutoRenameResult {
        source_id,
        requested_paths,
        renamed,
        skipped,
        errors,
    };
    #[cfg(test)]
    crate::app::controller::batch_latency::record(
        crate::app::controller::batch_latency::BatchLatencySample::new(
            crate::app::controller::batch_latency::BatchLatencyPhase::AutoRenameWorker,
            result.requested_paths.len(),
            started_at.elapsed(),
        )
        .with_detail_count(result.renamed.len() + result.skipped.len() + result.errors.len()),
    );
    result
}

fn emit_auto_rename_item_progress(
    progress: Option<&FileOpProgressSender>,
    completed: usize,
    detail: Option<String>,
    item: SampleAutoRenameProgress,
) {
    if let Some(progress) = progress {
        progress.auto_rename_progress(completed, detail, item);
    }
}

pub(super) fn perform_sample_rename(
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
    let db = crate::sample_sources::SourceDatabase::open_with_role(
        &source.root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    )
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

fn perform_sample_rename_with_db(
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
        .map_err(|err| format!("Failed to load custom tag: {err}"))?;
    let user_tag = user_tag.or(fallback_user_tag);
    let tag_named = db
        .tag_named_for_path(old_relative)
        .map_err(|err| format!("Failed to load tag-name marker: {err}"))?
        .or(fallback_tag_named)
        .unwrap_or(false);
    let mut batch = db
        .write_batch()
        .map_err(|err| format!("Failed to start database update: {err}"))?;
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
    batch
        .set_tag_named(new_relative, tag_named)
        .map_err(|err| format!("Failed to copy tag-name marker: {err}"))?;
    if let Some(last_played_at) = last_played_at {
        batch
            .set_last_played_at(new_relative, last_played_at)
            .map_err(|err| format!("Failed to copy playback age: {err}"))?;
    }
    let normal_tags = batch
        .tag_labels_for_path(old_relative)
        .map_err(|err| format!("Failed to load normal tags: {err}"))?;
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
        .map_err(|err| format!("Failed to save rename: {err}"))?;
    info!(
        source_id = %source.id,
        old_path = %old_relative.display(),
        new_path = %new_relative.display(),
        request_looped = looped_metadata.request_value(),
        db_looped = ?db_looped,
        final_looped = looped,
        "auto rename: persisted loop metadata provenance"
    );
    #[cfg(test)]
    RENAME_LOOPED_PROVENANCE_LOGS
        .lock()
        .unwrap()
        .push(RenameLoopedProvenanceLog {
            old_relative: old_relative.to_path_buf(),
            new_relative: new_relative.to_path_buf(),
            request_looped: looped_metadata.request_value(),
            db_looped,
            final_looped: looped,
        });
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
        normal_tags,
        tag_named,
    })
}

fn mark_sample_tag_named_with_db(
    db: &crate::sample_sources::SourceDatabase,
    relative_path: &Path,
    tag_named: bool,
) -> Result<Option<WavEntry>, String> {
    db.set_tag_named(relative_path, tag_named)
        .map_err(|err| format!("Failed to mark tag-name status: {err}"))?;
    db.entry_for_path(relative_path)
        .map_err(|err| format!("Failed to reload tag-name marker: {err}"))
}

fn is_busy_lock_error(err: &str) -> bool {
    let lowered = err.to_ascii_lowercase();
    lowered.contains("busy") || lowered.contains("locked")
}
