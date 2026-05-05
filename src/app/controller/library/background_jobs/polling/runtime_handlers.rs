//! Runtime-maintenance and map-build background-job handlers.

use super::*;
use crate::app::controller::jobs::{
    ConfigPersistJob, ConfigPersistResult, MetadataMutationResult, SourceDbMaintenanceRefresh,
    SourceDbMaintenanceResult, UmapBuildResult, UmapClusterBuildResult, WaveformRenderResult,
    WaveformTransientResult,
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
            .source_lane
            .mutations
            .finish_metadata_mutation(message.request_id)
        else {
            return;
        };
        self.extend_selected_source_mutation_claim_grace(&pending.source_id);
        if let Err(err) = message.result {
            self.rollback_metadata_mutation(&pending.source_id, &pending.rollback);
            if !pending.blocks_file_mutation && is_busy_lock_error_message(&err) {
                warn!(
                    source_id = %pending.source_id,
                    request_id = message.request_id,
                    elapsed_ms = message.elapsed.as_millis(),
                    error = %err,
                    "background analysis metadata mutation hit busy lock"
                );
                return;
            }
            self.set_status(format!("Metadata update failed: {err}"), StatusTone::Error);
            return;
        }
        self.finish_metadata_mutation_intents(&pending.source_id, &pending.rollback);
        if pending.refresh_browser_projection
            && self.selection_state.ctx.selected_source.as_ref() == Some(&pending.source_id)
        {
            let source_revision = self
                .current_source()
                .filter(|source| source.id == pending.source_id)
                .and_then(|source| self.database_for(&source).ok())
                .and_then(|db| db.get_revision().ok());
            self.ui_cache
                .browser
                .pipeline
                .sync_source_revision(source_revision);
            self.mark_browser_search_projection_revision_dirty();
            let metadata_delta_paths = pending.paths.iter().cloned().collect::<Vec<_>>();
            if self.should_dispatch_browser_search_async() {
                self.dispatch_search_job_with_metadata_delta(metadata_delta_paths);
            } else {
                self.rebuild_browser_lists_with_metadata_delta(metadata_delta_paths);
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
        if !self.waveform_render_key_matches_current_view(message.key) {
            self.runtime.pending_waveform_render = None;
            self.refresh_waveform_image();
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
        let Some(pending) = self.runtime.pending_waveform_transient_compute.as_ref() else {
            return;
        };
        if pending.request_id != message.request_id || pending.cache_token != message.cache_token {
            return;
        }
        self.runtime.pending_waveform_transient_compute = None;
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
                .deferred_startup_source_db_maintenance_jobs
                .extend(deferred);
            self.runtime.deferred_startup_source_db_maintenance_armed = true;
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

    fn rollback_metadata_mutation(&mut self, source_id: &SourceId, rollback: &[MetadataRollback]) {
        let active_source_matches =
            self.selection_state.ctx.selected_source.as_ref() == Some(source_id);
        for entry in rollback {
            match entry {
                MetadataRollback::TagAndLocked {
                    relative_path,
                    before_tag,
                    before_locked,
                    expected_tag,
                    expected_locked,
                } => self.rollback_tag_and_locked_metadata(
                    source_id,
                    relative_path,
                    *before_tag,
                    *before_locked,
                    *expected_tag,
                    *expected_locked,
                    active_source_matches,
                ),
                MetadataRollback::Looped {
                    relative_path,
                    intent_id,
                    before_looped,
                    expected_looped,
                } => self.rollback_looped_metadata(
                    source_id,
                    relative_path,
                    *intent_id,
                    *before_looped,
                    *expected_looped,
                    active_source_matches,
                ),
                MetadataRollback::SoundType {
                    relative_path,
                    before_sound_type,
                    expected_sound_type,
                } => self.rollback_sound_type_metadata(
                    source_id,
                    relative_path,
                    *before_sound_type,
                    *expected_sound_type,
                    active_source_matches,
                ),
                MetadataRollback::UserTag {
                    relative_path,
                    before_user_tag,
                    expected_user_tag,
                } => self.rollback_user_tag_metadata(
                    source_id,
                    relative_path,
                    before_user_tag,
                    expected_user_tag,
                    active_source_matches,
                ),
                MetadataRollback::NormalTag {
                    relative_path,
                    normalized_text,
                    display_label,
                    before_present,
                    expected_present,
                } => self.rollback_normal_tag_metadata(
                    source_id,
                    relative_path,
                    normalized_text,
                    display_label,
                    *before_present,
                    *expected_present,
                    active_source_matches,
                ),
                MetadataRollback::LastPlayedAt {
                    relative_path,
                    before_last_played_at,
                    expected_last_played_at,
                } => self.rollback_last_played_metadata(
                    source_id,
                    relative_path,
                    *before_last_played_at,
                    *expected_last_played_at,
                    active_source_matches,
                ),
                MetadataRollback::Bpm {
                    relative_path,
                    before_bpm,
                    expected_bpm,
                } => {
                    self.rollback_bpm_metadata(source_id, relative_path, *before_bpm, *expected_bpm)
                }
            }
        }
        if !active_source_matches {
            return;
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

    /// Handles rollback tag and locked metadata.
    fn rollback_tag_and_locked_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        before_tag: Rating,
        before_locked: bool,
        expected_tag: Rating,
        expected_locked: bool,
        active_source_matches: bool,
    ) {
        if active_source_matches && let Some(index) = self.wav_index_for_path(relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index)
                && wav.tag == expected_tag
                && wav.locked == expected_locked
            {
                wav.tag = before_tag;
                wav.locked = before_locked;
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
            && wav.tag == expected_tag
            && wav.locked == expected_locked
        {
            wav.tag = before_tag;
            wav.locked = before_locked;
        }
    }

    /// Handles rollback looped metadata.
    fn rollback_looped_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        intent_id: u64,
        before_looped: bool,
        expected_looped: bool,
        active_source_matches: bool,
    ) {
        let relative_path = self.resolve_looped_rollback_path(source_id, relative_path, intent_id);
        if !self
            .runtime
            .source_lane
            .mutations
            .looped_metadata_intent_matches(source_id, &relative_path, intent_id)
        {
            return;
        }
        if active_source_matches && let Some(index) = self.wav_index_for_path(&relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index)
                && wav.looped == expected_looped
            {
                wav.looped = before_looped;
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(&relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
            && wav.looped == expected_looped
        {
            wav.looped = before_looped;
        }
        self.runtime
            .source_lane
            .mutations
            .finish_looped_metadata_intent(source_id, &relative_path, intent_id);
    }

    /// Handles rollback sound type metadata.
    fn rollback_sound_type_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        before_sound_type: Option<crate::sample_sources::SampleSoundType>,
        expected_sound_type: Option<crate::sample_sources::SampleSoundType>,
        active_source_matches: bool,
    ) {
        if active_source_matches && let Some(index) = self.wav_index_for_path(relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index)
                && wav.sound_type == expected_sound_type
            {
                wav.sound_type = before_sound_type;
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
            && wav.sound_type == expected_sound_type
        {
            wav.sound_type = before_sound_type;
        }
    }

    /// Handles rollback user tag metadata.
    fn rollback_user_tag_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        before_user_tag: &Option<String>,
        expected_user_tag: &Option<String>,
        active_source_matches: bool,
    ) {
        if active_source_matches && let Some(index) = self.wav_index_for_path(relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index)
                && wav.user_tag == *expected_user_tag
            {
                wav.user_tag = before_user_tag.clone();
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
            && wav.user_tag == *expected_user_tag
        {
            wav.user_tag = before_user_tag.clone();
        }
    }

    /// Handles rollback normal tag metadata.
    fn rollback_normal_tag_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        normalized_text: &str,
        display_label: &str,
        before_present: bool,
        expected_present: bool,
        active_source_matches: bool,
    ) {
        if active_source_matches && let Some(index) = self.wav_index_for_path(relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index) {
                rollback_normal_tag_labels(
                    &mut wav.normal_tags,
                    normalized_text,
                    display_label,
                    before_present,
                    expected_present,
                );
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
        {
            rollback_normal_tag_labels(
                &mut wav.normal_tags,
                normalized_text,
                display_label,
                before_present,
                expected_present,
            );
        }
        let tags = self
            .ui_cache
            .browser
            .normal_tags
            .entry(source_id.clone())
            .or_default()
            .entry(relative_path.to_path_buf())
            .or_default();
        let current_present = tags
            .iter()
            .any(|tag| tag.normalized_text == normalized_text);
        if current_present != expected_present {
            return;
        }
        if before_present {
            if !current_present {
                tags.push(crate::sample_sources::db::SourceTag {
                    id: 0,
                    display_label: display_label.to_string(),
                    normalized_text: normalized_text.to_string(),
                });
                tags.sort_by(|left, right| {
                    left.display_label
                        .to_ascii_lowercase()
                        .cmp(&right.display_label.to_ascii_lowercase())
                        .then_with(|| left.normalized_text.cmp(&right.normalized_text))
                });
            }
        } else {
            tags.retain(|tag| tag.normalized_text != normalized_text);
        }
    }

    /// Handles rollback last played metadata.
    fn rollback_last_played_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        before_last_played_at: Option<i64>,
        expected_last_played_at: Option<i64>,
        active_source_matches: bool,
    ) {
        if active_source_matches && let Some(index) = self.wav_index_for_path(relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index)
                && wav.last_played_at == expected_last_played_at
            {
                wav.last_played_at = before_last_played_at;
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
            && wav.last_played_at == expected_last_played_at
        {
            wav.last_played_at = before_last_played_at;
        }
    }

    /// Handles rollback bpm metadata.
    fn rollback_bpm_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        before_bpm: Option<f32>,
        expected_bpm: Option<f32>,
    ) {
        let cache = self
            .ui_cache
            .browser
            .bpm_values
            .entry(source_id.clone())
            .or_default();
        if cache.get(relative_path).copied().flatten() == expected_bpm {
            cache.insert(relative_path.to_path_buf(), before_bpm);
        }
    }

    fn finish_metadata_mutation_intents(
        &mut self,
        source_id: &SourceId,
        rollback: &[MetadataRollback],
    ) {
        for entry in rollback {
            if let MetadataRollback::Looped {
                relative_path,
                intent_id,
                ..
            } = entry
            {
                let relative_path =
                    self.resolve_looped_rollback_path(source_id, relative_path, *intent_id);
                self.runtime
                    .source_lane
                    .mutations
                    .finish_looped_metadata_intent(source_id, &relative_path, *intent_id);
            }
        }
    }

    fn resolve_looped_rollback_path(
        &self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        intent_id: u64,
    ) -> std::path::PathBuf {
        if self
            .runtime
            .source_lane
            .mutations
            .looped_metadata_intent_matches(source_id, relative_path, intent_id)
        {
            return relative_path.to_path_buf();
        }
        if let Some(new_relative) =
            crate::app::controller::library::source_write_priority::completed_browser_rename_target(
                source_id,
                relative_path,
            )
            && self
                .runtime
                .source_lane
                .mutations
                .looped_metadata_intent_matches(source_id, &new_relative, intent_id)
        {
            return new_relative;
        }
        relative_path.to_path_buf()
    }
}

fn is_busy_lock_error_message(err: &str) -> bool {
    let lowered = err.to_ascii_lowercase();
    lowered.contains("busy") || lowered.contains("locked")
}

fn rollback_normal_tag_labels(
    labels: &mut Vec<String>,
    normalized_text: &str,
    display_label: &str,
    before_present: bool,
    expected_present: bool,
) {
    let current_present = labels
        .iter()
        .any(|label| label.to_ascii_lowercase() == normalized_text);
    if current_present != expected_present {
        return;
    }
    if before_present {
        if !current_present {
            labels.push(display_label.to_string());
            labels.sort_by_key(|label| label.to_ascii_lowercase());
        }
    } else {
        labels.retain(|label| label.to_ascii_lowercase() != normalized_text);
    }
}
