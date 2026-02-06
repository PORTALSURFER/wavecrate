//! Native runtime bridge between sempal controller state and `radiant`.

use crate::{
    audio::AudioPlayer,
    egui_app::{
        controller::EguiController,
        state::{TriageFlagColumn, UiState},
    },
    waveform::WaveformRenderer,
};
use radiant::app::{AppModel, ColumnModel, FrameBuildResult, NativeAppBridge, UiAction};
use std::{cell::RefCell, rc::Rc};

/// Host bridge used by the native `radiant` runtime.
pub struct SempalNativeBridge {
    controller: EguiController,
}

impl SempalNativeBridge {
    /// Build a new native bridge initialized with persisted sempal configuration.
    pub fn new(
        renderer: WaveformRenderer,
        player: Option<Rc<RefCell<AudioPlayer>>>,
    ) -> Result<Self, String> {
        let cfg = crate::sample_sources::config::load_or_default()
            .map_err(|err| format!("Failed to load config: {err}"))?;
        let mut controller = EguiController::new_with_job_message_queue_capacity(
            renderer,
            player,
            cfg.core.job_message_queue_capacity as usize,
        );
        controller
            .apply_configuration(cfg)
            .map_err(|err| format!("Failed to load config: {err}"))?;
        controller.select_first_source();
        Ok(Self { controller })
    }

    fn project_model(&self) -> AppModel {
        let ui = &self.controller.ui;
        let selected_column = selected_column_index(ui);
        let transport_running = self.controller.is_playing();
        AppModel {
            title: String::from("Sempal"),
            backend_label: String::from("backend: native_vello"),
            sources_label: format!("Sources ({})", ui.sources.rows.len()),
            status_text: ui.status.text.clone(),
            columns: [
                ColumnModel::new("Trash", ui.browser.trash.len()),
                ColumnModel::new("Samples", ui.browser.neutral.len()),
                ColumnModel::new("Keep", ui.browser.keep.len()),
            ],
            selected_column,
            transport_running,
        }
    }

    fn on_select_column(&mut self, target_index: usize) {
        let target_index = target_index.min(2);
        let current_index = selected_column_index(&self.controller.ui);
        let delta = target_index as isize - current_index as isize;
        if delta != 0 {
            self.controller.move_selection_column(delta);
        }
    }
}

impl NativeAppBridge for SempalNativeBridge {
    fn pull_model(&mut self) -> AppModel {
        self.controller.tick_playhead();
        self.controller.poll_background_jobs();
        self.controller.update_performance_governor(false);
        self.project_model()
    }

    fn on_action(&mut self, action: UiAction) {
        match action {
            UiAction::SelectColumn { index } => self.on_select_column(index),
            UiAction::MoveColumn { delta } => {
                self.controller.move_selection_column(delta as isize);
            }
            UiAction::ToggleTransport => self.controller.toggle_play_pause(),
        }
    }

    fn on_frame_result(&mut self, _result: FrameBuildResult) {}

    fn on_exit(&mut self) {
        if let Err(err) = self.controller.save_full_config() {
            eprintln!("Failed to persist config on native runtime exit: {err}");
        }
    }
}

/// Construct a native runtime bridge for the current sempal controller stack.
pub fn new_native_bridge(
    renderer: WaveformRenderer,
    player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<SempalNativeBridge, String> {
    SempalNativeBridge::new(renderer, player)
}

fn selected_column_index(ui: &UiState) -> usize {
    ui.browser
        .selected
        .map(|selected| match selected.column {
            TriageFlagColumn::Trash => 0,
            TriageFlagColumn::Neutral => 1,
            TriageFlagColumn::Keep => 2,
        })
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_column_defaults_to_middle_column_without_selection() {
        let ui = UiState::default();
        assert_eq!(selected_column_index(&ui), 1);
    }
}
