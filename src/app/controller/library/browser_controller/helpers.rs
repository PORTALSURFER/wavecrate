use super::*;
use crate::app::controller::jobs::{
    FileOpResult, NormalizationJob, SampleAutoRenameResult, SampleAutoRenameSuccess,
    SampleRenameResult,
};
use crate::app::controller::undo;
use std::sync::{Arc, atomic::AtomicBool};

pub(crate) struct BrowserController<'a> {
    controller: &'a mut AppController,
}

impl<'a> BrowserController<'a> {
    pub(crate) fn new(controller: &'a mut AppController) -> Self {
        Self { controller }
    }
}

impl std::ops::Deref for BrowserController<'_> {
    type Target = AppController;

    fn deref(&self) -> &Self::Target {
        self.controller
    }
}

impl std::ops::DerefMut for BrowserController<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.controller
    }
}

pub(crate) struct TriageSampleContext {
    pub(crate) source: SampleSource,
    pub(crate) entry: WavEntry,
    pub(crate) absolute_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct DeleteBrowserFocusPlan {
    pub(crate) preferred_path: Option<PathBuf>,
    pub(crate) fallback_visible_row: Option<usize>,
}

impl BrowserController<'_> {
    pub(crate) fn try_normalize_browser_sample(&mut self, row: usize) -> Result<(), String> {
        let ctx = self.resolve_browser_sample(row)?;
        self.try_normalize_browser_sample_ctx(&ctx)
    }

    pub(crate) fn try_normalize_browser_sample_ctx(
        &mut self,
        ctx: &TriageSampleContext,
    ) -> Result<(), String> {
        if self.controller.warn_if_retained_delete_path_busy(
            &ctx.source.id,
            &ctx.entry.relative_path,
            "normalizing",
        ) {
            return Ok(());
        }
        if cfg!(test) {
            return self.normalize_browser_sample_sync(ctx);
        }
        self.controller.begin_pending_sample_overwrite_transaction(
            crate::app::controller::history::PendingHistoryTransactionKey::Normalization {
                source_id: ctx.source.id.clone(),
                relative_path: ctx.entry.relative_path.clone(),
            },
            format!("Normalized {}", ctx.entry.relative_path.display()),
            ctx.source.id.clone(),
            ctx.entry.relative_path.clone(),
            ctx.absolute_path.clone(),
        )?;
        let job = NormalizationJob {
            source: ctx.source.clone(),
            relative_path: ctx.entry.relative_path.clone(),
            absolute_path: ctx.absolute_path.clone(),
        };

        if self.controller.ui.progress.task != Some(ProgressTaskKind::Normalization) {
            self.controller.show_status_progress(
                ProgressTaskKind::Normalization,
                format!("Normalizing {}", ctx.entry.relative_path.display()),
                1,
                false,
            );
        }

        self.controller.runtime.jobs.begin_normalization(job);
        Ok(())
    }

    fn normalize_browser_sample_sync(&mut self, ctx: &TriageSampleContext) -> Result<(), String> {
        let before = self.capture_meaningful_ui_snapshot();
        let backup = undo::OverwriteBackup::capture_before(&ctx.absolute_path)?;
        let was_playing = self.is_playing();
        let was_looping = self.ui.waveform.loop_enabled;
        let playhead_position = self.ui.waveform.playhead.position;

        let (file_size, modified_ns, tag) = self.normalize_and_save_for_path(
            &ctx.source,
            &ctx.entry.relative_path,
            &ctx.absolute_path,
        )?;
        let entry_index = self.wav_index_for_path(&ctx.entry.relative_path);
        let looped = entry_index
            .and_then(|idx| self.wav_entries.entry(idx))
            .map(|entry| entry.looped)
            .unwrap_or(false);
        let last_played_at = entry_index
            .and_then(|idx| self.wav_entries.entry(idx))
            .and_then(|entry| entry.last_played_at);
        let updated = WavEntry {
            relative_path: ctx.entry.relative_path.clone(),
            file_size,
            modified_ns,
            content_hash: None,
            tag,
            looped,
            sound_type: entry_index
                .and_then(|idx| self.wav_entries.entry(idx))
                .and_then(|entry| entry.sound_type),
            locked: entry_index
                .and_then(|idx| self.wav_entries.entry(idx))
                .map(|entry| entry.locked)
                .unwrap_or(false),
            missing: false,
            last_played_at,
            user_tag: entry_index
                .and_then(|idx| self.wav_entries.entry(idx))
                .and_then(|entry| entry.user_tag.clone()),
        };

        let is_currently_loaded = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                audio.source_id == ctx.source.id && audio.relative_path == ctx.entry.relative_path
            });
        if is_currently_loaded && was_playing {
            let start_override = if playhead_position.is_finite() {
                Some(f64::from(playhead_position.clamp(0.0, 1.0)))
            } else {
                None
            };
            self.runtime
                .jobs
                .set_pending_playback(Some(PendingPlayback {
                    source_id: ctx.source.id.clone(),
                    relative_path: ctx.entry.relative_path.clone(),
                    looped: was_looping,
                    start_override,
                    force_loaded_audio: false,
                }));
        }

        self.update_cached_entry(&ctx.source, &ctx.entry.relative_path, updated);
        if self.selection_state.ctx.selected_source.as_ref() == Some(&ctx.source.id) {
            self.rebuild_browser_lists();
        }
        self.refresh_waveform_for_sample(&ctx.source, &ctx.entry.relative_path);
        self.set_status(
            format!("Normalized {}", ctx.entry.relative_path.display()),
            StatusTone::Info,
        );
        backup.capture_after(&ctx.absolute_path)?;
        let after = self.capture_meaningful_ui_snapshot();
        let entry = self.selection_edit_undo_entry(
            format!("Normalized {}", ctx.entry.relative_path.display()),
            ctx.source.id.clone(),
            ctx.entry.relative_path.clone(),
            ctx.absolute_path.clone(),
            backup,
        );
        self.push_undo_entry(AppController::attach_meaningful_ui_restore(
            entry, before, after,
        ));
        Ok(())
    }
    pub(crate) fn next_browser_focus_after_delete(
        &mut self,
        rows: &[usize],
    ) -> Option<DeleteBrowserFocusPlan> {
        if rows.is_empty() || self.ui.browser.viewport.visible.len() == 0 {
            return None;
        }
        let mut sorted = rows.to_vec();
        sorted.sort_unstable();
        let highest = sorted.last().copied()?;
        let first = sorted.first().copied().unwrap_or(highest);
        let after = highest
            .checked_add(1)
            .and_then(|idx| self.ui.browser.viewport.visible.get(idx))
            .and_then(|entry_idx| self.wav_entry(entry_idx))
            .map(|entry| entry.relative_path.clone());
        let fallback_visible_row = if after.is_some() {
            Some(first)
        } else {
            first.checked_sub(1)
        };
        let preferred_path = after.or_else(|| {
            first
                .checked_sub(1)
                .and_then(|idx| self.ui.browser.viewport.visible.get(idx))
                .and_then(|entry_idx| self.wav_entry(entry_idx))
                .map(|entry| entry.relative_path.clone())
        });
        if preferred_path.is_none() && fallback_visible_row.is_none() {
            return None;
        }
        Some(DeleteBrowserFocusPlan {
            preferred_path,
            fallback_visible_row,
        })
    }

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

    pub(super) fn warn_if_any_browser_context_busy(
        &mut self,
        contexts: &[TriageSampleContext],
        action: &str,
    ) -> bool {
        let Some(ctx) = contexts.iter().find(|ctx| {
            self.controller
                .runtime
                .active_retained_delete_resolution
                .as_ref()
                .is_some_and(|active| {
                    active.entries.iter().any(|entry| {
                        entry.source_id == ctx.source.id
                            && entry.contains_path(&ctx.entry.relative_path)
                    })
                })
        }) else {
            return false;
        };
        self.controller.warn_if_retained_delete_path_busy(
            &ctx.source.id,
            &ctx.entry.relative_path,
            action,
        )
    }
}

fn run_sample_rename_job(
    ctx: TriageSampleContext,
    new_relative: PathBuf,
    tag: crate::sample_sources::Rating,
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

pub(crate) struct SampleAutoRenameRequest {
    pub(crate) old_relative: PathBuf,
    pub(crate) new_relative: PathBuf,
    pub(crate) tag: crate::sample_sources::Rating,
    pub(crate) resume_playback: bool,
    pub(crate) resume_looped: bool,
    pub(crate) resume_start_override: Option<f64>,
}

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

fn perform_sample_rename(
    source: &SampleSource,
    old_absolute: &Path,
    old_relative: &Path,
    new_relative: &Path,
    tag: crate::sample_sources::Rating,
    fallback_looped: bool,
    fallback_locked: bool,
    fallback_last_played_at: Option<i64>,
) -> Result<WavEntry, String> {
    let new_absolute = source.root.join(new_relative);
    std::fs::rename(old_absolute, &new_absolute)
        .map_err(|err| format!("Failed to rename file: {err}"))
        .and_then(|_| {
            let (file_size, modified_ns) = file_metadata(&new_absolute)?;
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
                .map_err(|err| format!("Failed to load sound type: {err}"))?;
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
                .upsert_file(&new_relative, file_size, modified_ns)
                .map_err(|err| format!("Failed to register renamed file: {err}"))?;
            batch
                .set_tag(&new_relative, tag)
                .map_err(|err| format!("Failed to copy tag: {err}"))?;
            batch
                .set_looped(&new_relative, looped)
                .map_err(|err| format!("Failed to copy loop marker: {err}"))?;
            batch
                .set_sound_type(&new_relative, sound_type)
                .map_err(|err| format!("Failed to copy sound type: {err}"))?;
            batch
                .set_user_tag(&new_relative, user_tag.as_deref())
                .map_err(|err| format!("Failed to copy custom tag: {err}"))?;
            if let Some(last_played_at) = last_played_at {
                batch
                    .set_last_played_at(&new_relative, last_played_at)
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
        })
}
