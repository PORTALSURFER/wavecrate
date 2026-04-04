//! Runtime-maintenance and map-build background-job handlers.

use super::*;
use crate::app::controller::jobs::{
    ConfigPersistJob, ConfigPersistResult, MetadataMutationResult, SourceDbMaintenanceResult,
    UmapBuildResult, UmapClusterBuildResult, WaveformRenderResult,
};
use crate::app::controller::state::runtime::MetadataRollback;
use tracing::{info, warn};

impl AppController {
    /// Apply one completed metadata mutation batch to optimistic controller state.
    pub(crate) fn handle_metadata_mutation_finished_message(
        &mut self,
        message: MetadataMutationResult,
    ) {
        let Some(pending) = self
            .runtime
            .pending_metadata_mutations
            .remove(&message.request_id)
        else {
            return;
        };
        for path in &pending.paths {
            self.runtime
                .pending_metadata_paths
                .remove(&(pending.source_id.clone(), path.clone()));
        }
        if let Err(err) = message.result {
            self.rollback_metadata_mutation(&pending.rollback);
            self.set_status(format!("Metadata update failed: {err}"), StatusTone::Error);
            return;
        }
        if pending.refresh_browser_projection
            && self.selection_state.ctx.selected_source.as_ref() == Some(&pending.source_id)
        {
            self.mark_browser_search_projection_revision_dirty();
            if self.should_dispatch_browser_search_async() {
                self.dispatch_search_job();
            } else {
                self.rebuild_browser_lists();
            }
        }
    }

    /// Apply one deferred configuration persistence completion.
    pub(super) fn handle_config_persist_finished_message(&mut self, message: ConfigPersistResult) {
        let Some(pending) = self.runtime.pending_config_persist.as_ref() else {
            return;
        };
        if pending.request_id != message.request_id {
            return;
        }
        self.runtime.pending_config_persist = None;
        match (message.job, message.result) {
            (ConfigPersistJob::SaveVolume { volume, .. }, Ok(())) => {
                self.runtime.volume_persist_dirty = false;
                self.runtime.volume_persist_deadline = None;
                self.runtime.last_persisted_volume_milli =
                    Some(((volume.clamp(0.0, 1.0) * 1000.0).round() as u16).min(1000));
            }
            (ConfigPersistJob::SaveVolume { .. }, Err(err)) => {
                warn!(error = %err, "volume persistence failed");
                self.runtime.volume_persist_deadline =
                    Some(std::time::Instant::now() + std::time::Duration::from_millis(120));
                self.set_status(format!("Failed to save volume: {err}"), StatusTone::Error);
            }
        }
    }

    /// Apply one latest-only waveform image render completion.
    pub(super) fn handle_waveform_rendered_message(&mut self, message: WaveformRenderResult) {
        let Some(pending) = self.runtime.pending_waveform_render.as_ref() else {
            return;
        };
        if pending.request_id != message.request_id || pending.key != message.key {
            return;
        }
        self.runtime.pending_waveform_render = None;
        match message.result {
            Ok(visual) => {
                self.store_prepared_waveform_image(
                    visual.image,
                    visual.projected_image,
                    visual.render_meta,
                );
                info!(
                    request_id = message.request_id,
                    elapsed_ms = message.elapsed.as_millis(),
                    "applied async waveform render"
                );
            }
            Err(err) => {
                self.set_status(format!("Waveform render failed: {err}"), StatusTone::Error);
            }
        }
    }

    /// Apply one completed similarity-map layout build result.
    pub(super) fn handle_umap_built_message(&mut self, message: UmapBuildResult) {
        self.runtime.jobs.clear_umap_build();
        match message.result {
            Ok(()) => {
                self.ui.map.bounds = None;
                self.ui.map.cached_bounds_source_id = None;
                self.ui.map.cached_bounds_umap_version = None;
                self.ui.map.last_query = None;
                self.ui.map.cached_points.clear();
                self.ui.map.cached_points_source_id = None;
                self.ui.map.cached_points_umap_version = None;
                self.mark_map_dataset_projection_revision_dirty();
                self.mark_map_query_projection_revision_dirty();
                self.set_status(
                    format!("Similarity map layout {} built", message.umap_version),
                    StatusTone::Info,
                );
            }
            Err(err) => {
                self.set_status(
                    format!("Similarity map layout build failed: {err}"),
                    StatusTone::Error,
                );
            }
        }
    }

    /// Apply one completed similarity-map cluster-build result.
    pub(super) fn handle_umap_clusters_built_message(&mut self, message: UmapClusterBuildResult) {
        self.runtime.jobs.clear_umap_cluster_build();
        match message.result {
            Ok(stats) => {
                self.ui.map.last_query = None;
                self.ui.map.cached_points.clear();
                self.ui.map.cached_points_source_id = None;
                self.ui.map.cached_points_umap_version = None;
                self.ui.map.cached_cluster_centroids_key = None;
                self.ui.map.cached_cluster_centroids = None;
                self.ui.map.auto_cluster_build_requested_key = None;
                self.mark_map_dataset_projection_revision_dirty();
                self.mark_map_query_projection_revision_dirty();
                let scope = message
                    .source_id
                    .as_ref()
                    .map(|id| id.as_str())
                    .unwrap_or("all sources");
                self.set_status(
                    format!(
                        "Clusters built for {scope} ({} clusters, {:.1}% noise)",
                        stats.cluster_count,
                        stats.noise_ratio * 100.0
                    ),
                    StatusTone::Info,
                );
            }
            Err(err) => {
                self.set_status(format!("Cluster build failed: {err}"), StatusTone::Error);
            }
        }
    }

    /// Apply one deferred source-DB maintenance batch result.
    pub(super) fn handle_source_db_maintenance_finished_message(
        &mut self,
        message: SourceDbMaintenanceResult,
    ) {
        self.runtime.jobs.clear_source_db_maintenance();
        let mut failed = 0usize;
        let mut sources_to_refresh = Vec::new();
        for outcome in message.outcomes {
            if outcome.refresh_required {
                sources_to_refresh.push(outcome.source_id.clone());
            }
            if let Some(err) = outcome.error {
                failed = failed.saturating_add(1);
                tracing::warn!(
                    "Deferred source DB maintenance failed for {} ({}): {}",
                    outcome.source_id,
                    outcome.source_root.display(),
                    err
                );
            }
        }
        for source_id in sources_to_refresh {
            if let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| source.id == source_id)
                .cloned()
            {
                self.invalidate_wav_entries_for_source(&source);
            }
        }
        if failed > 0 {
            let suffix = if failed == 1 { "" } else { "s" };
            self.set_status(
                format!("Deferred source DB maintenance failed for {failed} source{suffix}"),
                StatusTone::Warning,
            );
        }
    }

    fn rollback_metadata_mutation(&mut self, rollback: &[MetadataRollback]) {
        for entry in rollback {
            match entry {
                MetadataRollback::TagAndLocked {
                    relative_path,
                    before_tag,
                    before_locked,
                    expected_tag,
                    expected_locked,
                } => {
                    if let Some(index) = self.wav_index_for_path(relative_path) {
                        let _ = self.ensure_wav_page_loaded(index);
                        if let Some(wav) = self.wav_entries.entry_mut(index)
                            && wav.tag == *expected_tag
                            && wav.locked == *expected_locked
                        {
                            wav.tag = *before_tag;
                            wav.locked = *before_locked;
                        }
                    }
                    if let Some(source_id) = self.selection_state.ctx.selected_source.as_ref()
                        && let Some(cache) = self.cache.wav.entries.get_mut(source_id)
                        && let Some(index) = cache.lookup.get(relative_path).copied()
                        && let Some(wav) = cache.entry_mut(index)
                        && wav.tag == *expected_tag
                        && wav.locked == *expected_locked
                    {
                        wav.tag = *before_tag;
                        wav.locked = *before_locked;
                    }
                }
                MetadataRollback::Looped {
                    relative_path,
                    before_looped,
                    expected_looped,
                } => {
                    if let Some(index) = self.wav_index_for_path(relative_path) {
                        let _ = self.ensure_wav_page_loaded(index);
                        if let Some(wav) = self.wav_entries.entry_mut(index)
                            && wav.looped == *expected_looped
                        {
                            wav.looped = *before_looped;
                        }
                    }
                    if let Some(source_id) = self.selection_state.ctx.selected_source.as_ref()
                        && let Some(cache) = self.cache.wav.entries.get_mut(source_id)
                        && let Some(index) = cache.lookup.get(relative_path).copied()
                        && let Some(wav) = cache.entry_mut(index)
                        && wav.looped == *expected_looped
                    {
                        wav.looped = *before_looped;
                    }
                }
                MetadataRollback::LastPlayedAt {
                    relative_path,
                    before_last_played_at,
                    expected_last_played_at,
                } => {
                    if let Some(index) = self.wav_index_for_path(relative_path) {
                        let _ = self.ensure_wav_page_loaded(index);
                        if let Some(wav) = self.wav_entries.entry_mut(index)
                            && wav.last_played_at == *expected_last_played_at
                        {
                            wav.last_played_at = *before_last_played_at;
                        }
                    }
                    if let Some(source_id) = self.selection_state.ctx.selected_source.as_ref()
                        && let Some(cache) = self.cache.wav.entries.get_mut(source_id)
                        && let Some(index) = cache.lookup.get(relative_path).copied()
                        && let Some(wav) = cache.entry_mut(index)
                        && wav.last_played_at == *expected_last_played_at
                    {
                        wav.last_played_at = *before_last_played_at;
                    }
                }
                MetadataRollback::Bpm {
                    relative_path,
                    before_bpm,
                    expected_bpm,
                } => {
                    if let Some(source_id) = self.selection_state.ctx.selected_source.as_ref() {
                        let cache = self
                            .ui_cache
                            .browser
                            .bpm_values
                            .entry(source_id.clone())
                            .or_default();
                        if cache.get(relative_path).copied().flatten() == *expected_bpm {
                            cache.insert(relative_path.clone(), *before_bpm);
                        }
                    }
                }
            }
        }
        self.ui_cache.browser.pipeline.invalidate();
        self.mark_browser_row_metadata_projection_revision_dirty();
        self.mark_browser_search_projection_revision_dirty();
        if self.should_dispatch_browser_search_async() {
            self.dispatch_search_job();
        } else {
            self.rebuild_browser_lists();
        }
    }
}
