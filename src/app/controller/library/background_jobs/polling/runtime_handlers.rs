//! Runtime-maintenance and map-build background-job handlers.

use super::*;
use crate::app::controller::jobs::{
    ConfigPersistJob, ConfigPersistResult, SourceDbMaintenanceRefresh, SourceDbMaintenanceResult,
    UmapBuildResult, UmapClusterBuildResult, WaveformRenderResult, WaveformTransientResult,
};

mod metadata;
mod metadata_intents;
mod metadata_normal_tags;
mod metadata_rollback;

use tracing::{info, warn};

impl AppController {
    /// Apply one deferred configuration persistence completion.
    pub(super) fn handle_config_persist_finished_message(&mut self, message: ConfigPersistResult) {
        let Some(pending) = self
            .runtime
            .config_persistence
            .pending_config_persist
            .as_ref()
        else {
            return;
        };
        if pending.request_id != message.request_id {
            return;
        }
        self.runtime.config_persistence.pending_config_persist = None;
        match (message.job, message.result) {
            (ConfigPersistJob::SaveVolume { volume, .. }, Ok(())) => {
                self.runtime.config_persistence.volume_persist_dirty = false;
                self.runtime.config_persistence.volume_persist_deadline = None;
                self.runtime.config_persistence.last_persisted_volume_milli =
                    Some(((volume.clamp(0.0, 1.0) * 1000.0).round() as u16).min(1000));
            }
            (ConfigPersistJob::SaveVolume { .. }, Err(err)) => {
                warn!(error = %err, "volume persistence failed");
                self.runtime.config_persistence.volume_persist_deadline =
                    Some(std::time::Instant::now() + std::time::Duration::from_millis(120));
                self.set_status(format!("Failed to save volume: {err}"), StatusTone::Error);
            }
        }
    }

    /// Apply one latest-only waveform image render completion.
    pub(super) fn handle_waveform_rendered_message(&mut self, message: WaveformRenderResult) {
        let Some(pending) = self.runtime.waveform.pending_render.as_ref() else {
            return;
        };
        if pending.request_id != message.request_id || pending.key != message.key {
            return;
        }
        if !self.waveform_render_key_matches_current_view(message.key) {
            self.runtime.waveform.pending_render = None;
            self.refresh_waveform_image();
            return;
        }
        self.runtime.waveform.pending_render = None;
        match message.result {
            Ok(visual) => {
                self.store_prepared_waveform_image(
                    visual.image,
                    visual.projected_image,
                    visual.render_meta,
                );
                self.finalize_staged_audio_handoff(message.key.cache_token);
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

    /// Apply one latest-only waveform transient-marker completion.
    pub(super) fn handle_waveform_transients_computed_message(
        &mut self,
        message: WaveformTransientResult,
    ) {
        let Some(pending) = self.runtime.waveform.pending_transient_compute.as_ref() else {
            return;
        };
        if pending.request_id != message.request_id || pending.cache_token != message.cache_token {
            return;
        }
        self.runtime.waveform.pending_transient_compute = None;
        let decoded_matches = self
            .sample_view
            .waveform
            .decoded
            .as_ref()
            .is_some_and(|decoded| decoded.cache_token == message.cache_token);
        if !decoded_matches {
            return;
        }
        match message.result {
            Ok(transients) => {
                self.ui.waveform.transients = transients;
                self.ui.waveform.transient_cache_token = Some(message.cache_token);
                if self.ui.waveform.transient_markers_enabled {
                    self.refresh_waveform_image();
                }
                info!(
                    request_id = message.request_id,
                    elapsed_ms = message.elapsed.as_millis(),
                    "applied deferred waveform transients"
                );
            }
            Err(err) => {
                self.set_status(
                    format!("Waveform transient detection failed: {err}"),
                    StatusTone::Error,
                );
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
        let mut deferred = Vec::new();
        let mut sources_to_reconcile = Vec::new();
        let mut sources_to_reload = Vec::new();
        for outcome in message.outcomes {
            if outcome.deferred_due_to_file_op {
                deferred.push(crate::app::controller::jobs::SourceDbMaintenanceJob {
                    source_id: outcome.source_id.clone(),
                    source_root: outcome.source_root.clone(),
                });
                continue;
            }
            match outcome.refresh {
                SourceDbMaintenanceRefresh::None => {}
                SourceDbMaintenanceRefresh::FileOpReconcile => {
                    sources_to_reconcile.push(outcome.source_id.clone());
                }
                SourceDbMaintenanceRefresh::FullSourceReload => {
                    sources_to_reload.push(outcome.source_id.clone());
                }
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
        for source_id in sources_to_reconcile {
            self.apply_source_db_maintenance_file_op_reconcile(&source_id);
        }
        for source_id in sources_to_reload {
            if let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| source.id == source_id)
                .cloned()
            {
                self.refresh_wav_entries_for_source(&source);
            }
        }
        if !deferred.is_empty() {
            self.runtime
                .startup
                .deferred_source_db_maintenance_jobs
                .extend(deferred);
            self.runtime.startup.deferred_source_db_maintenance_armed = true;
        }
        if failed > 0 {
            let suffix = if failed == 1 { "" } else { "s" };
            self.set_status(
                format!("Deferred source DB maintenance failed for {failed} source{suffix}"),
                StatusTone::Warning,
            );
        }
    }

    fn apply_source_db_maintenance_file_op_reconcile(&mut self, source_id: &SourceId) {
        self.rebuild_missing_lookup_for_source(source_id);
        if self.selection_state.ctx.selected_source.as_ref() != Some(source_id) {
            return;
        }
        let source = self
            .library
            .sources
            .iter()
            .find(|source| &source.id == source_id)
            .cloned();
        let source_revision = source
            .as_ref()
            .and_then(|source| self.database_for(source).ok())
            .and_then(|db| db.get_revision().ok());
        self.ui_cache
            .browser
            .pipeline
            .sync_source_revision(source_revision);
        self.mark_browser_search_projection_revision_dirty();
        if self.should_dispatch_browser_search_async() {
            self.dispatch_search_job();
        } else {
            self.rebuild_browser_lists();
        }
    }
}
