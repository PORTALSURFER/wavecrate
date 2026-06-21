use super::*;
use crate::app::state::FolderPaneId;
use crate::sample_sources::SampleSource;
use crate::sample_sources::config::AppSettingsCore;
use std::path::PathBuf;
use tracing::info;

impl AppController {
    pub(super) fn apply_core_settings(&mut self, core: &AppSettingsCore) {
        self.settings.feature_flags = core.feature_flags.clone();
        self.settings.analysis = core.analysis.clone();
        self.settings.analysis.max_analysis_duration_seconds =
            super::super::library::analysis_options::clamp_max_analysis_duration_seconds(
                self.settings.analysis.max_analysis_duration_seconds,
            );
        self.settings.analysis.long_sample_threshold_seconds =
            super::super::library::analysis_options::clamp_long_sample_threshold_seconds(
                self.settings.analysis.long_sample_threshold_seconds,
            );
        self.settings.updates = core.updates.clone();
        self.settings.job_message_queue_capacity = core.job_message_queue_capacity;
        self.settings.app_data_dir = core.app_data_dir.clone();
        self.settings.trash_folder = core.trash_folder.clone();
        self.settings.drop_targets = core.drop_targets.clone();
        self.settings.collection_names = core.collection_names.clone();
        self.settings.folder_locks = core.folder_locks.clone();
        self.settings.audio_output = core.audio_output.clone();
        self.ui.audio.selected = self.settings.audio_output.clone();
        self.settings.audio_input = core.audio_input.clone();
        self.ui.audio.input_selected = self.settings.audio_input.clone();
        self.settings.audio_write_format = core.audio_write_format.clone();
        self.ui.audio.write_format = self.settings.audio_write_format.clone();
        self.settings.similarity = core.similarity.clone().normalized();
        self.settings.default_identifier = default_identifier(&core.default_identifier);
        self.ui.options_panel.default_identifier = self.settings.default_identifier.clone();
        self.settings.tag_dictionary = core.tag_dictionary.clone();
    }

    pub(super) fn apply_control_settings(&mut self, core: &AppSettingsCore) {
        self.settings.controls = core.controls.clone();
        self.settings.controls.waveform_scroll_speed =
            clamp_scroll_speed(self.settings.controls.waveform_scroll_speed);
        self.settings.controls.wheel_zoom_factor =
            clamp_zoom_factor(self.settings.controls.wheel_zoom_factor);
        self.settings.controls.keyboard_zoom_factor =
            clamp_zoom_factor(self.settings.controls.keyboard_zoom_factor);
        self.settings.controls.anti_clip_fade_ms =
            super::super::ui::interaction_options::clamp_anti_clip_fade_ms(
                self.settings.controls.anti_clip_fade_ms,
            );
        self.ui.controls = crate::app::state::InteractionOptionsState {
            invert_waveform_scroll: self.settings.controls.invert_waveform_scroll,
            waveform_scroll_speed: self.settings.controls.waveform_scroll_speed,
            wheel_zoom_factor: self.settings.controls.wheel_zoom_factor,
            keyboard_zoom_factor: self.settings.controls.keyboard_zoom_factor,
            anti_clip_fade_enabled: self.settings.controls.anti_clip_fade_enabled,
            anti_clip_fade_ms: self.settings.controls.anti_clip_fade_ms,
            auto_edge_fades_on_selection_exports: self
                .settings
                .controls
                .auto_edge_fades_on_selection_exports,
            destructive_yolo_mode: self.settings.controls.destructive_yolo_mode,
            waveform_channel_view: self.settings.controls.waveform_channel_view,
            input_monitoring_enabled: self.settings.controls.input_monitoring_enabled,
            advance_after_rating: self.settings.controls.advance_after_rating,
            tooltip_mode: self.settings.controls.tooltip_mode,
        };
        self.sync_waveform_control_ui();
    }

    pub(super) fn apply_startup_audio_settings(&mut self, core: &AppSettingsCore) {
        self.stage_deferred_startup_audio_refresh();
        self.apply_volume(core.volume);
        self.ui.trash_folder = core.trash_folder.clone();
        self.ui.update.last_seen_nightly_published_at =
            core.updates.last_seen_nightly_published_at.clone();
    }

    pub(super) fn apply_startup_sources(
        &mut self,
        core: &AppSettingsCore,
        retained_sources: Vec<SampleSource>,
    ) {
        self.library.sources = retained_sources;
        self.rebuild_missing_sources();
        self.warn_about_missing_sources();
        self.stage_deferred_startup_source_db_maintenance();

        let persisted_selected = valid_source_id(&self.library.sources, &core.last_selected_source);
        let upper_source = valid_source_id(&self.library.sources, &core.upper_folder_pane_source);
        let lower_source = valid_source_id(&self.library.sources, &core.lower_folder_pane_source);
        let active_source = startup_active_source(
            core,
            &self.library.sources,
            &persisted_selected,
            &upper_source,
            &lower_source,
        );

        self.ui.sources.folder_panes.upper.source_id = active_source.clone();
        self.ui.sources.folder_panes.lower.source_id = active_source;
        self.ui.sources.active_folder_pane = FolderPaneId::Upper;
        self.load_active_folder_ui_from_pane();
        self.selection_state.ctx.selected_source =
            self.folder_pane_source(self.ui.sources.active_folder_pane);
        self.selection_state.ctx.last_selected_browsable_source = self
            .selection_state
            .ctx
            .selected_source
            .clone()
            .or(persisted_selected);
        self.refresh_sources_ui();
        if self.selection_state.ctx.selected_source.is_some() {
            let _ = self.refresh_wavs();
        }
    }

    pub(super) fn start_analysis_runtime(&mut self) {
        self.runtime.analysis.set_max_analysis_duration_seconds(
            self.settings.analysis.max_analysis_duration_seconds,
        );
        self.runtime
            .analysis
            .set_worker_count(self.settings.analysis.analysis_worker_count);
        self.runtime
            .analysis
            .start(self.runtime.jobs.message_sender());
    }

    pub(super) fn persist_transient_source_cleanup(&mut self, removed_transient_roots: &[PathBuf]) {
        let removed_count = removed_transient_roots.len();
        if removed_count == 0 {
            return;
        }

        let suffix = if removed_count == 1 { "" } else { "s" };
        info!(
            removed_transient_test_sources = removed_count,
            roots = ?removed_transient_roots,
            "Removed persisted transient test startup sources."
        );
        if let Err(err) = self.save_full_config() {
            self.set_status(
                format!(
                    "Removed {removed_count} transient test source{suffix}, but failed to persist cleanup: {err}"
                ),
                StatusTone::Warning,
            );
        } else {
            self.set_status(
                format!(
                    "Removed {removed_count} transient test source{suffix} from startup config"
                ),
                StatusTone::Info,
            );
        }
    }

    fn sync_waveform_control_ui(&mut self) {
        self.ui.waveform.channel_view = self.settings.controls.waveform_channel_view;
        self.ui.waveform.bpm_snap_enabled = self.settings.controls.bpm_snap_enabled;
        self.ui.waveform.relative_bpm_grid_enabled =
            self.settings.controls.relative_bpm_grid_enabled;
        self.ui.waveform.bpm_lock_enabled = self.settings.controls.bpm_lock_enabled;
        self.ui.waveform.bpm_stretch_enabled = self.settings.controls.bpm_stretch_enabled;
        self.ui.waveform.bpm_value = normalize_bpm_value(self.settings.controls.bpm_value);
        self.ui.waveform.loop_lock_enabled = self.settings.controls.loop_lock_enabled;
        self.ui.waveform.transient_markers_enabled =
            self.settings.controls.transient_markers_enabled;
        self.ui.waveform.transient_snap_enabled = self.settings.controls.transient_snap_enabled
            && self.settings.controls.transient_markers_enabled;
        self.ui.waveform.normalized_audition_enabled =
            self.settings.controls.normalized_audition_enabled;
        sync_waveform_bpm_input(&mut self.ui.waveform.bpm_input, self.ui.waveform.bpm_value);
    }

    fn warn_about_missing_sources(&mut self) {
        if self.library.missing.sources.is_empty() {
            return;
        }

        let count = self.library.missing.sources.len();
        let suffix = if count == 1 { "" } else { "s" };
        self.set_status(
            format!("{count} source{suffix} unavailable"),
            StatusTone::Warning,
        );
    }
}

fn default_identifier(value: &str) -> String {
    if value.trim().is_empty() {
        String::from("portal")
    } else {
        value.trim().to_string()
    }
}

fn sync_waveform_bpm_input(input: &mut String, bpm_value: Option<f32>) {
    if let Some(value) = bpm_value {
        let rounded = value.round();
        if (value - rounded).abs() < 0.01 {
            *input = format!("{rounded:.0}");
        } else {
            *input = format!("{value:.2}");
        }
    } else {
        input.clear();
    }
}

fn valid_source_id(
    sources: &[SampleSource],
    source_id: &Option<crate::sample_sources::SourceId>,
) -> Option<crate::sample_sources::SourceId> {
    source_id
        .clone()
        .filter(|id| sources.iter().any(|source| &source.id == id))
}

fn startup_active_source(
    core: &AppSettingsCore,
    sources: &[SampleSource],
    persisted_selected: &Option<crate::sample_sources::SourceId>,
    upper_source: &Option<crate::sample_sources::SourceId>,
    lower_source: &Option<crate::sample_sources::SourceId>,
) -> Option<crate::sample_sources::SourceId> {
    let active_pane_source = match parse_active_folder_pane(core.active_folder_pane.as_deref()) {
        FolderPaneId::Upper => upper_source.clone(),
        FolderPaneId::Lower => lower_source.clone(),
    };
    active_pane_source
        .or_else(|| upper_source.clone())
        .or_else(|| lower_source.clone())
        .or_else(|| persisted_selected.clone())
        .or_else(|| sources.first().map(|source| source.id.clone()))
}
