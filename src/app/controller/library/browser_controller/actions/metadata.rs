use super::super::helpers::TriageSampleContext;
use super::common::format_bpm_label;
use super::*;
use crate::app::controller::jobs::AnalysisMetadataMutationOp;
use crate::app::controller::state::runtime::MetadataRollback;
use std::collections::HashMap;
use std::path::PathBuf;
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
        let mut grouped: HashMap<SourceId, (SampleSource, Vec<PathBuf>)> = HashMap::new();
        for ctx in contexts {
            grouped
                .entry(ctx.source.id.clone())
                .or_insert_with(|| (ctx.source.clone(), Vec::new()))
                .1
                .push(ctx.entry.relative_path.clone());
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
            self.queue_metadata_mutation(
                &source,
                Vec::new(),
                analysis_ops,
                rollback,
                false,
            );
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
}
