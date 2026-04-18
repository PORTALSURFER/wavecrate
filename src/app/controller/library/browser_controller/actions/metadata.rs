use super::super::helpers::TriageSampleContext;
use super::super::{
    auto_rename::{AutoRenameInput, build_auto_rename_stem},
    helpers::{SampleAutoRenameRequest, run_sample_auto_rename_job},
};
use super::common::format_bpm_label;
use super::*;
use crate::app::controller::jobs::{AnalysisMetadataMutationOp, FileOpResult};
use crate::app::controller::state::runtime::MetadataRollback;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};
use tracing::{info, warn};

impl BrowserController<'_> {
    pub(super) fn tag_browser_sample_action(
        &mut self,
        row: usize,
        tag: crate::sample_sources::Rating,
    ) -> Result<(), String> {
        info!(row, ?tag, "triage tag: single row");
        let result: Result<(), String> = (|| {
            let ctx = self.resolve_browser_sample(row)?;
            self.set_sample_tag_for_source(&ctx.source, &ctx.entry.relative_path, tag, true)?;
            self.set_status(
                format!("Tagged {} as {:?}", ctx.entry.relative_path.display(), tag),
                StatusTone::Info,
            );
            Ok(())
        })();
        if let Err(err) = &result {
            warn!(row, ?tag, error = %err, "triage tag failed");
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    pub(super) fn tag_browser_samples_action(
        &mut self,
        rows: &[usize],
        tag: crate::sample_sources::Rating,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        info!(?rows, ?tag, primary_visible_row, "triage tag: multi row");
        let (contexts, mut last_error) = self.resolve_unique_browser_contexts(rows);
        info!(count = contexts.len(), "triage tag: resolved contexts");
        for ctx in contexts {
            if let Err(err) =
                self.set_sample_tag_for_source(&ctx.source, &ctx.entry.relative_path, tag, true)
            {
                last_error = Some(err);
            } else {
                self.set_status(
                    format!("Tagged {} as {:?}", ctx.entry.relative_path.display(), tag),
                    StatusTone::Info,
                );
            }
        }
        self.refocus_after_filtered_removal(primary_visible_row);
        if let Some(err) = last_error {
            warn!(?rows, ?tag, error = %err, "triage tag failed for multi row");
            Err(err)
        } else {
            Ok(())
        }
    }

    pub(super) fn set_loop_marker_browser_samples_action(
        &mut self,
        rows: &[usize],
        looped: bool,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        let (contexts, last_error) = self.resolve_unique_browser_contexts(rows);
        self.apply_loop_marker_contexts(contexts, last_error, looped, primary_visible_row)
    }

    pub(crate) fn set_loop_marker_browser_sample_paths_action(
        &mut self,
        paths: &[PathBuf],
        looped: bool,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        let (contexts, last_error) = self.resolve_unique_browser_contexts_for_paths(paths);
        self.apply_loop_marker_contexts(contexts, last_error, looped, primary_visible_row)
    }

    pub(super) fn set_bpm_browser_samples_action(
        &mut self,
        rows: &[usize],
        bpm: f32,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        let (contexts, last_error) = self.resolve_unique_browser_contexts(rows);
        self.apply_bpm_contexts(contexts, last_error, bpm, primary_visible_row)
    }

    pub(crate) fn set_bpm_browser_sample_paths_action(
        &mut self,
        paths: &[PathBuf],
        bpm: f32,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        let (contexts, last_error) = self.resolve_unique_browser_contexts_for_paths(paths);
        self.apply_bpm_contexts(contexts, last_error, bpm, primary_visible_row)
    }

    fn apply_loop_marker_contexts(
        &mut self,
        contexts: Vec<TriageSampleContext>,
        mut last_error: Option<String>,
        looped: bool,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        let action_label = if looped {
            "Marked loop"
        } else {
            "Cleared loop"
        };
        for ctx in contexts {
            if let Err(err) = self.set_sample_looped_for_source(
                &ctx.source,
                &ctx.entry.relative_path,
                looped,
                true,
            ) {
                last_error = Some(err);
            } else {
                self.set_status(
                    format!("{action_label} {}", ctx.entry.relative_path.display()),
                    StatusTone::Info,
                );
            }
        }
        self.refocus_after_filtered_removal(primary_visible_row);
        if let Some(err) = last_error {
            warn!(looped, error = %err, "loop marker failed for multi row");
            Err(err)
        } else {
            Ok(())
        }
    }

    fn apply_bpm_contexts(
        &mut self,
        contexts: Vec<TriageSampleContext>,
        last_error: Option<String>,
        bpm: f32,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        if !bpm.is_finite() || bpm <= 0.0 {
            return Err("BPM must be a positive number".to_string());
        }
        let mut grouped: HashMap<SourceId, (SampleSource, BTreeSet<PathBuf>)> = HashMap::new();
        for ctx in contexts {
            grouped
                .entry(ctx.source.id.clone())
                .or_insert_with(|| (ctx.source.clone(), BTreeSet::new()))
                .1
                .insert(ctx.entry.relative_path.clone());
        }
        let mut updated = 0usize;
        for (source_id, (source, paths)) in grouped {
            updated = updated.saturating_add(paths.len());
            let rollback = paths
                .iter()
                .map(|path| MetadataRollback::Bpm {
                    relative_path: path.clone(),
                    before_bpm: self
                        .ui_cache
                        .browser
                        .bpm_values
                        .get(&source_id)
                        .and_then(|cache| cache.get(path).copied().flatten()),
                    expected_bpm: Some(bpm),
                })
                .collect::<Vec<_>>();
            let analysis_ops = paths
                .iter()
                .map(|path| AnalysisMetadataMutationOp::SetBpm {
                    relative_path: path.clone(),
                    bpm: Some(bpm),
                })
                .collect::<Vec<_>>();
            let cache = self
                .ui_cache
                .browser
                .bpm_values
                .entry(source_id.clone())
                .or_default();
            for path in &paths {
                cache.insert(path.clone(), Some(bpm));
            }
            let loaded_matches = self
                .sample_view
                .wav
                .loaded_audio
                .as_ref()
                .is_some_and(|audio| {
                    audio.source_id == source_id && paths.contains(&audio.relative_path)
                });
            if loaded_matches {
                self.set_waveform_bpm_input(Some(bpm));
            }
            self.queue_metadata_mutation(&source, Vec::new(), analysis_ops, rollback, false);
        }
        if updated > 0 {
            self.mark_browser_row_metadata_projection_revision_dirty();
            let label = format_bpm_label(bpm);
            let sample_label = if updated == 1 { "sample" } else { "samples" };
            self.set_status(
                format!("Set BPM {label} for {updated} {sample_label}"),
                StatusTone::Info,
            );
        }
        self.refocus_after_filtered_removal(primary_visible_row);
        if let Some(err) = last_error {
            warn!(bpm, error = %err, "bpm update failed for browser samples");
            Err(err)
        } else {
            Ok(())
        }
    }

    pub(super) fn rename_browser_sample_action(
        &mut self,
        row: usize,
        new_name: &str,
    ) -> Result<(), String> {
        let ctx = self.resolve_browser_sample(row)?;
        if self.warn_if_any_browser_context_busy(std::slice::from_ref(&ctx), "renaming") {
            return Ok(());
        }
        let result = self.try_rename_browser_sample(row, new_name);
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    pub(crate) fn auto_rename_browser_sample_paths_action(
        &mut self,
        paths: &[PathBuf],
    ) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }
        let Some(source) = self.current_source() else {
            return Err(String::from("No source selected"));
        };
        if self.runtime.jobs.file_ops_in_progress() {
            return Err("File operation already in progress".to_string());
        }
        let db = self
            .database_for(&source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        let mut requests = Vec::new();
        let mut reserved_targets = HashSet::new();
        let identifier = self.settings.default_identifier.clone();
        self.preload_bpm_values_for_paths(paths);
        for relative_path in paths {
            let tag = self.sample_tag_for(&source, relative_path)?;
            let looped = self.sample_looped_for(&source, relative_path)?;
            let sound_type = self
                .live_sound_type_for_path(&source, relative_path)
                .or(db
                    .sound_type_for_path(relative_path)
                    .map_err(|err| format!("Failed to read sound type: {err}"))?)
                .or_else(|| {
                    relative_path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .and_then(crate::sample_sources::SampleSoundType::infer_from_name)
                });
            if let Some(sound_type) = sound_type {
                let _ = db.set_sound_type(relative_path, Some(sound_type));
            }
            let stem = build_auto_rename_stem(&AutoRenameInput {
                identifier: identifier.clone(),
                looped,
                sound_type,
                bpm: self.bpm_value_for_path(relative_path),
            });
            let new_relative = self.resolve_auto_rename_target(
                &source.root,
                relative_path,
                stem.tagged_basename.as_deref(),
                &stem.fallback_identifier,
                &mut reserved_targets,
            )?;
            let is_currently_loaded =
                self.sample_view
                    .wav
                    .loaded_audio
                    .as_ref()
                    .is_some_and(|audio| {
                        audio.source_id == source.id && audio.relative_path == *relative_path
                    });
            let playhead_position = self.ui.waveform.playhead.position;
            requests.push(SampleAutoRenameRequest {
                old_relative: relative_path.clone(),
                new_relative,
                tag,
                resume_playback: is_currently_loaded && self.is_playing(),
                resume_looped: self.ui.waveform.loop_enabled,
                resume_start_override: playhead_position
                    .is_finite()
                    .then(|| f64::from(playhead_position.clamp(0.0, 1.0))),
            });
        }
        let requested_paths = requests
            .iter()
            .map(|request| request.old_relative.clone())
            .collect::<Vec<_>>();
        self.begin_pending_file_mutation(&source.id, requested_paths.clone());
        if cfg!(test) {
            let result = run_sample_auto_rename_job(
                source.clone(),
                requests,
                Arc::new(AtomicBool::new(false)),
            );
            self.apply_file_op_result(FileOpResult::SampleAutoRename(result));
            return Ok(());
        }
        self.set_status(
            format!("Auto renaming {} sample(s)...", requested_paths.len()),
            StatusTone::Busy,
        );
        let pending_source_id = source.id.clone();
        if let Err(err) = self.runtime.jobs.begin_one_shot_file_op(move |cancel| {
            FileOpResult::SampleAutoRename(run_sample_auto_rename_job(source, requests, cancel))
        }) {
            self.finish_pending_file_mutation(&pending_source_id, requested_paths);
            return Err(err);
        }
        Ok(())
    }

    fn resolve_auto_rename_target(
        &self,
        root: &Path,
        relative_path: &Path,
        tagged_basename: Option<&str>,
        fallback_identifier: &str,
        reserved_targets: &mut HashSet<PathBuf>,
    ) -> Result<PathBuf, String> {
        if let Some(tagged_basename) = tagged_basename {
            if let Some(path) =
                self.try_auto_rename_target(root, relative_path, tagged_basename, reserved_targets)?
            {
                reserved_targets.insert(path.clone());
                return Ok(path);
            }
            for index in 1..=999 {
                let suffixed_basename = format!("{tagged_basename}_{index:03}");
                if let Some(path) = self.try_auto_rename_target(
                    root,
                    relative_path,
                    &suffixed_basename,
                    reserved_targets,
                )? {
                    reserved_targets.insert(path.clone());
                    return Ok(path);
                }
            }
        }
        for index in 1..=999 {
            let fallback_basename = format!("{fallback_identifier}_{index:03}");
            if let Some(path) = self.try_auto_rename_target(
                root,
                relative_path,
                &fallback_basename,
                reserved_targets,
            )? {
                reserved_targets.insert(path.clone());
                return Ok(path);
            }
        }
        Err(format!(
            "Unable to find a unique auto-rename target for {}",
            relative_path.display()
        ))
    }

    fn try_auto_rename_target(
        &self,
        root: &Path,
        relative_path: &Path,
        basename: &str,
        reserved_targets: &HashSet<PathBuf>,
    ) -> Result<Option<PathBuf>, String> {
        let full_name = self.name_with_preserved_extension(relative_path, basename)?;
        let new_relative = self.validate_new_sample_name_in_parent(relative_path, root, &full_name);
        match new_relative {
            Ok(path) if path == relative_path || !reserved_targets.contains(&path) => {
                Ok(Some(path))
            }
            Ok(_) => Ok(None),
            Err(err) if err.contains("already exists") => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn live_sound_type_for_path(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Option<crate::sample_sources::SampleSoundType> {
        self.wav_index_for_path(relative_path)
            .and_then(|index| {
                let _ = self.ensure_wav_page_loaded(index);
                self.wav_entry(index).and_then(|entry| entry.sound_type)
            })
            .or_else(|| {
                self.cache
                    .wav
                    .entries
                    .get(&source.id)
                    .and_then(|cache| cache.lookup.get(relative_path).copied())
                    .and_then(|index| self.cache.wav.entries.get(&source.id)?.entry(index))
                    .and_then(|entry| entry.sound_type)
            })
    }
}
