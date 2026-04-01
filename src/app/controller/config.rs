use super::ui::interaction_options::{clamp_scroll_speed, clamp_zoom_factor};
use super::*;

impl AppController {
    /// Load persisted configuration and populate initial UI state.
    #[allow(clippy::result_large_err)]
    pub fn load_configuration(&mut self) -> Result<(), crate::sample_sources::config::ConfigError> {
        let cfg = crate::sample_sources::config::load_or_default()?;
        self.apply_configuration(cfg)
    }

    /// Apply a preloaded configuration snapshot to the controller state.
    #[allow(clippy::result_large_err)]
    pub fn apply_configuration(
        &mut self,
        cfg: crate::sample_sources::config::AppConfig,
    ) -> Result<(), crate::sample_sources::config::ConfigError> {
        let (sources, removed_transient_benchmark_sources) =
            prune_transient_benchmark_sources(cfg.sources.clone());
        self.settings.feature_flags = cfg.core.feature_flags;
        self.settings.analysis = cfg.core.analysis;
        self.settings.analysis.max_analysis_duration_seconds =
            super::library::analysis_options::clamp_max_analysis_duration_seconds(
                self.settings.analysis.max_analysis_duration_seconds,
            );
        self.settings.analysis.long_sample_threshold_seconds =
            super::library::analysis_options::clamp_long_sample_threshold_seconds(
                self.settings.analysis.long_sample_threshold_seconds,
            );
        self.settings.updates = cfg.core.updates.clone();
        self.settings.job_message_queue_capacity = cfg.core.job_message_queue_capacity;
        self.settings.app_data_dir = cfg.core.app_data_dir.clone();
        self.settings.trash_folder = cfg.core.trash_folder.clone();
        self.settings.drop_targets = cfg.core.drop_targets.clone();
        self.settings.audio_output = cfg.core.audio_output.clone();
        self.ui.audio.selected = self.settings.audio_output.clone();
        self.settings.audio_input = cfg.core.audio_input.clone();
        self.ui.audio.input_selected = self.settings.audio_input.clone();
        self.settings.controls = cfg.core.controls.clone();
        self.settings.controls.waveform_scroll_speed =
            clamp_scroll_speed(self.settings.controls.waveform_scroll_speed);
        self.settings.controls.wheel_zoom_factor =
            clamp_zoom_factor(self.settings.controls.wheel_zoom_factor);
        self.settings.controls.keyboard_zoom_factor =
            clamp_zoom_factor(self.settings.controls.keyboard_zoom_factor);
        self.settings.controls.anti_clip_fade_ms =
            super::ui::interaction_options::clamp_anti_clip_fade_ms(
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
        if let Some(value) = self.ui.waveform.bpm_value {
            let rounded = value.round();
            if (value - rounded).abs() < 0.01 {
                self.ui.waveform.bpm_input = format!("{rounded:.0}");
            } else {
                self.ui.waveform.bpm_input = format!("{value:.2}");
            }
        } else {
            self.ui.waveform.bpm_input.clear();
        }
        self.refresh_audio_options(true);
        self.refresh_audio_input_options(true);
        self.apply_volume(cfg.core.volume);
        self.ui.trash_folder = cfg.core.trash_folder.clone();
        self.ui.update.last_seen_nightly_published_at =
            cfg.core.updates.last_seen_nightly_published_at.clone();
        self.library.sources = sources;
        self.rebuild_missing_sources();
        if !self.library.missing.sources.is_empty() {
            let count = self.library.missing.sources.len();
            let suffix = if count == 1 { "" } else { "s" };
            self.set_status(
                format!("{count} source{suffix} unavailable"),
                StatusTone::Warning,
            );
        }
        self.stage_deferred_startup_source_db_maintenance();
        self.selection_state.ctx.selected_source = cfg
            .core
            .last_selected_source
            .filter(|id| self.library.sources.iter().any(|s| &s.id == id));
        self.selection_state.ctx.last_selected_browsable_source =
            self.selection_state.ctx.selected_source.clone();
        self.refresh_sources_ui();
        if self.selection_state.ctx.selected_source.is_some() {
            let _ = self.refresh_wavs();
        }
        self.maybe_check_for_updates_on_startup();
        self.runtime.analysis.set_max_analysis_duration_seconds(
            self.settings.analysis.max_analysis_duration_seconds,
        );
        self.runtime
            .analysis
            .set_worker_count(self.settings.analysis.analysis_worker_count);
        self.runtime
            .analysis
            .start(self.runtime.jobs.message_sender());
        if removed_transient_benchmark_sources > 0
            && let Err(err) = self.save_full_config()
        {
            let suffix = if removed_transient_benchmark_sources == 1 {
                ""
            } else {
                "s"
            };
            self.set_status(
                format!(
                    "Removed {removed_transient_benchmark_sources} transient benchmark source{suffix}, but failed to persist cleanup: {err}"
                ),
                StatusTone::Warning,
            );
        }
        Ok(())
    }

    /// Queue deferred source-db maintenance so startup can reach first paint quickly.
    fn stage_deferred_startup_source_db_maintenance(&mut self) {
        let jobs = self
            .library
            .sources
            .iter()
            .filter(|source| source.root.is_dir())
            .map(
                |source| crate::app::controller::jobs::SourceDbMaintenanceJob {
                    source_id: source.id.clone(),
                    source_root: source.root.clone(),
                },
            )
            .collect::<Vec<_>>();
        self.runtime.deferred_startup_source_db_maintenance_jobs = jobs;
        self.runtime.deferred_startup_source_db_maintenance_armed = !self
            .runtime
            .deferred_startup_source_db_maintenance_jobs
            .is_empty();
        self.runtime.startup_frame_prepare_count = 0;
    }

    /// Launch deferred startup source-db maintenance after the first prepared frame.
    pub(crate) fn flush_deferred_startup_source_db_maintenance(&mut self) {
        if !self.runtime.deferred_startup_source_db_maintenance_armed {
            return;
        }
        self.runtime.startup_frame_prepare_count =
            self.runtime.startup_frame_prepare_count.saturating_add(1);
        if self.runtime.startup_frame_prepare_count < 2 {
            return;
        }
        if self.runtime.jobs.source_db_maintenance_in_progress() {
            return;
        }
        let jobs = std::mem::take(&mut self.runtime.deferred_startup_source_db_maintenance_jobs);
        self.runtime.deferred_startup_source_db_maintenance_armed = false;
        self.runtime.jobs.begin_source_db_maintenance(jobs);
    }

    /// Return true when startup-deferred source-db maintenance is armed.
    pub(crate) fn has_pending_startup_source_db_maintenance(&self) -> bool {
        self.runtime.deferred_startup_source_db_maintenance_armed
    }

    pub(super) fn persist_config(&mut self, error_prefix: &str) -> Result<(), String> {
        self.save_full_config()
            .map_err(|err| format!("{error_prefix}: {err}"))
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn save_full_config(
        &self,
    ) -> Result<(), crate::sample_sources::config::ConfigError> {
        crate::sample_sources::config::save(&crate::sample_sources::config::AppConfig {
            sources: self.library.sources.clone(),
            core: crate::sample_sources::config::AppSettingsCore {
                feature_flags: self.settings.feature_flags.clone(),
                analysis: self.settings.analysis.clone(),
                updates: self.settings.updates.clone(),
                job_message_queue_capacity: self.settings.job_message_queue_capacity,
                app_data_dir: self.settings.app_data_dir.clone(),
                trash_folder: self.settings.trash_folder.clone(),
                drop_targets: self.settings.drop_targets.clone(),
                last_selected_source: self
                    .selection_state
                    .ctx
                    .selected_source
                    .clone()
                    .filter(|id| self.library.sources.iter().any(|s| &s.id == id))
                    .or_else(|| {
                        self.selection_state
                            .ctx
                            .last_selected_browsable_source
                            .clone()
                    }),
                audio_output: self.settings.audio_output.clone(),
                audio_input: self.settings.audio_input.clone(),
                volume: self.ui.volume,
                controls: self.settings.controls.clone(),
            },
        })
    }

    /// Open the `.sempal` config directory in the OS file explorer.
    pub fn open_config_folder(&mut self) {
        match crate::app_dirs::app_root_dir() {
            Ok(path) => {
                if let Err(err) = open::that(&path) {
                    self.set_status(
                        format!("Could not open config folder {}: {err}", path.display()),
                        StatusTone::Error,
                    );
                }
            }
            Err(err) => {
                self.set_status(
                    format!("Could not resolve config folder: {err}"),
                    StatusTone::Error,
                );
            }
        }
    }
}

fn normalize_bpm_value(value: f32) -> Option<f32> {
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        None
    }
}

/// Remove transient benchmark sources that were incorrectly persisted into user config.
fn prune_transient_benchmark_sources(
    sources: Vec<crate::sample_sources::SampleSource>,
) -> (Vec<crate::sample_sources::SampleSource>, usize) {
    let mut retained = Vec::with_capacity(sources.len());
    let mut removed = 0usize;
    for source in sources {
        if is_transient_benchmark_source(&source) {
            removed = removed.saturating_add(1);
        } else {
            retained.push(source);
        }
    }
    (retained, removed)
}

/// Identify benchmark-generated GUI sources that should never survive into normal app state.
fn is_transient_benchmark_source(source: &crate::sample_sources::SampleSource) -> bool {
    source.root.starts_with(std::env::temp_dir())
        && source
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("gui-source"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn prune_transient_benchmark_sources_keeps_user_sources() {
        let retained_dir = tempdir().expect("retained tempdir");
        let retained_root = retained_dir.path().join("user-source");
        std::fs::create_dir_all(&retained_root).expect("create retained source");
        let transient_dir = tempdir().expect("transient tempdir");
        let transient_root = transient_dir.path().join("gui-source");
        std::fs::create_dir_all(&transient_root).expect("create transient source");
        let retained_source = crate::sample_sources::SampleSource::new(retained_root.clone());

        let (sources, removed) = prune_transient_benchmark_sources(vec![
            crate::sample_sources::SampleSource::new(transient_root),
            retained_source.clone(),
        ]);

        assert_eq!(removed, 1);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].root, retained_source.root);
    }
}
