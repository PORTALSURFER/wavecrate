use super::ui::interaction_options::{clamp_scroll_speed, clamp_zoom_factor};
use super::*;
use crate::app::state::FolderPaneId;

mod apply;

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
        let startup_source_repair =
            super::startup_source_repair::repair_persisted_startup_sources(cfg.sources.clone());
        let retained_sources = startup_source_repair.retained_sources;
        let removed_transient_roots = startup_source_repair.removed_roots;
        self.apply_core_settings(&cfg.core);
        self.apply_control_settings(&cfg.core);
        self.apply_startup_audio_settings(&cfg.core);
        self.apply_startup_sources(&cfg.core, retained_sources);
        self.maybe_check_for_updates_on_startup();
        self.start_analysis_runtime();
        self.persist_transient_source_cleanup(&removed_transient_roots);
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
        self.runtime.startup.deferred_source_db_maintenance_jobs = jobs;
        self.runtime.startup.deferred_source_db_maintenance_armed = !self
            .runtime
            .startup
            .deferred_source_db_maintenance_jobs
            .is_empty();
        self.runtime.startup.frame_prepare_count = 0;
    }

    /// Launch deferred startup source-db maintenance after the first prepared frame.
    pub(crate) fn flush_deferred_startup_source_db_maintenance(&mut self) {
        if !self.runtime.startup.deferred_source_db_maintenance_armed {
            return;
        }
        self.runtime.startup.frame_prepare_count =
            self.runtime.startup.frame_prepare_count.saturating_add(1);
        if self.runtime.startup.frame_prepare_count < 2 {
            return;
        }
        if self.runtime.jobs.source_db_maintenance_in_progress() {
            return;
        }
        let mut ready = Vec::new();
        let mut deferred = Vec::new();
        for job in std::mem::take(&mut self.runtime.startup.deferred_source_db_maintenance_jobs) {
            if self.source_has_pending_file_mutations(&job.source_id) {
                deferred.push(job);
            } else {
                ready.push(job);
            }
        }
        // Browser file ops own the source write lane briefly; maintenance is
        // recoverable and can wait for the same source while other sources run.
        self.runtime.startup.deferred_source_db_maintenance_jobs = deferred;
        self.runtime.startup.deferred_source_db_maintenance_armed = !self
            .runtime
            .startup
            .deferred_source_db_maintenance_jobs
            .is_empty();
        self.runtime.jobs.begin_source_db_maintenance(ready);
    }

    /// Return true when startup-deferred source-db maintenance is armed.
    pub(crate) fn has_pending_startup_source_db_maintenance(&self) -> bool {
        self.runtime.startup.deferred_source_db_maintenance_armed
    }

    /// Clear probed audio option state and arm a refresh after the first presented frame.
    fn stage_deferred_startup_audio_refresh(&mut self) {
        self.ui.audio.hosts.clear();
        self.ui.audio.devices.clear();
        self.ui.audio.sample_rates.clear();
        self.ui.audio.warning = None;
        self.ui.audio.output_runtime_error = None;
        self.ui.audio.input_hosts.clear();
        self.ui.audio.input_devices.clear();
        self.ui.audio.input_sample_rates.clear();
        self.ui.audio.input_channel_count = 0;
        self.ui.audio.input_warning = None;
        self.runtime.startup.deferred_audio_refresh.armed = true;
        self.runtime.startup.deferred_audio_refresh.prepare_count = 0;
    }

    /// Run the deferred startup audio refresh after first paint reaches the screen.
    pub(crate) fn flush_deferred_startup_audio_refresh(&mut self) {
        if !self.runtime.startup.deferred_audio_refresh.armed {
            return;
        }
        self.runtime.startup.deferred_audio_refresh.prepare_count = self
            .runtime
            .startup
            .deferred_audio_refresh
            .prepare_count
            .saturating_add(1);
        if self.runtime.startup.deferred_audio_refresh.prepare_count < 2 {
            return;
        }
        self.ensure_startup_audio_refresh();
    }

    /// Return true when startup audio probing is still pending.
    pub(crate) fn has_pending_startup_audio_refresh(&self) -> bool {
        self.runtime.startup.deferred_audio_refresh.armed
    }

    /// Complete the deferred startup audio probe immediately when settings are opened early.
    pub(crate) fn ensure_startup_audio_refresh(&mut self) {
        if !self.runtime.startup.deferred_audio_refresh.armed {
            return;
        }
        self.runtime.startup.deferred_audio_refresh.armed = false;
        self.runtime.startup.deferred_audio_refresh.prepare_count = 0;
        self.perform_startup_audio_refresh();
    }

    /// Refresh startup audio host/device state unless tests stub the probe boundary.
    fn perform_startup_audio_refresh(&mut self) {
        #[cfg(test)]
        if crate::app::controller::startup_audio_test_support::record_startup_audio_refresh_for_tests()
        {
            return;
        }
        self.refresh_audio_options(true);
        self.refresh_audio_input_options(true);
        let _ = self.rebuild_audio_player();
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
                upper_folder_pane_source: self.folder_pane_source(FolderPaneId::Upper),
                lower_folder_pane_source: self.folder_pane_source(FolderPaneId::Upper),
                active_folder_pane: Some(FolderPaneId::Upper.as_str().to_string()),
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
                            .filter(|id| self.library.sources.iter().any(|s| &s.id == id))
                    }),
                audio_output: self.settings.audio_output.clone(),
                audio_input: self.settings.audio_input.clone(),
                audio_write_format: self.settings.audio_write_format.clone(),
                volume: self.ui.volume,
                controls: self.settings.controls.clone(),
                similarity: self.settings.similarity.clone(),
                collection_names: self.settings.collection_names.clone(),
                default_identifier: self.settings.default_identifier.clone(),
                tag_dictionary: self.settings.tag_dictionary.clone(),
            },
        })
    }

    /// Open the `.wavecrate` config directory in the OS file explorer.
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

fn parse_active_folder_pane(value: Option<&str>) -> FolderPaneId {
    match value {
        Some("lower") => FolderPaneId::Lower,
        _ => FolderPaneId::Upper,
    }
}

fn normalize_bpm_value(value: f32) -> Option<f32> {
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        None
    }
}
