use super::common::format_bpm_label;
use super::*;
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
        let (contexts, mut last_error) = self.resolve_unique_browser_contexts(rows);
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
            warn!(?rows, looped, error = %err, "loop marker failed for multi row");
            Err(err)
        } else {
            Ok(())
        }
    }

    pub(super) fn set_bpm_browser_samples_action(
        &mut self,
        rows: &[usize],
        bpm: f32,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        if !bpm.is_finite() || bpm <= 0.0 {
            return Err("BPM must be a positive number".to_string());
        }
        let (contexts, mut last_error) = self.resolve_unique_browser_contexts(rows);
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
            let mut conn = match analysis_jobs::open_source_db(&source.root) {
                Ok(conn) => conn,
                Err(err) => {
                    last_error = Some(format!("Failed to open source DB for BPM save: {err}"));
                    continue;
                }
            };
            let sample_ids: Vec<String> = paths
                .iter()
                .map(|path| analysis_jobs::build_sample_id(source_id.as_str(), path))
                .collect();
            match analysis_jobs::update_sample_bpms(&mut conn, &sample_ids, Some(bpm)) {
                Ok(count) => updated = updated.saturating_add(count),
                Err(err) => last_error = Some(err),
            }
            if let Some(cache) = self.ui_cache.browser.bpm_values.get_mut(&source_id) {
                for path in &paths {
                    cache.insert(path.clone(), Some(bpm));
                }
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
        }
        if updated > 0 {
            let label = format_bpm_label(bpm);
            let sample_label = if updated == 1 { "sample" } else { "samples" };
            self.set_status(
                format!("Set BPM {label} for {updated} {sample_label}"),
                StatusTone::Info,
            );
        }
        self.refocus_after_filtered_removal(primary_visible_row);
        if let Some(err) = last_error {
            warn!(?rows, bpm, error = %err, "bpm update failed for browser samples");
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
        let result = self.try_rename_browser_sample(row, new_name);
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }
}
