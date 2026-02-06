//! Native runtime bridge between sempal controller state and `radiant`.

use crate::{
    app_core::native_shell::{
        browser_focus_target, normalized_from_milli, project_app_model, selected_column_index,
        selection_range_from_milli,
    },
    audio::AudioPlayer,
    egui_app::controller::EguiController,
    waveform::WaveformRenderer,
};
use radiant::app::{AppModel, FrameBuildResult, NativeAppBridge, UiAction};
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

    fn project_model(&mut self) -> AppModel {
        project_app_model(&mut self.controller)
    }

    fn on_select_column(&mut self, target_index: usize) {
        let target_index = target_index.min(2);
        let current_index = selected_column_index(&self.controller.ui);
        let delta = target_index as isize - current_index as isize;
        if delta != 0 {
            self.controller.move_selection_column(delta);
        }
    }

    fn move_browser_focus(&mut self, delta: i8) {
        let Some(target) = browser_focus_target(&self.controller.ui, delta) else {
            return;
        };
        self.controller.focus_browser_row(target);
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
            UiAction::FocusBrowserPanel => self.controller.focus_browser_list(),
            UiAction::FocusSourcesPanel => self.controller.focus_sources_list(),
            UiAction::FocusWaveformPanel => self.controller.focus_waveform(),
            UiAction::FocusLoadedSampleInBrowser => {
                self.controller.focus_loaded_sample_in_browser()
            }
            UiAction::FocusBrowserSearch => self.controller.focus_browser_search(),
            UiAction::FocusFolderSearch => self.controller.focus_folder_search(),
            UiAction::SelectSourceRow { index } => self.controller.select_source_by_index(index),
            UiAction::MoveBrowserFocus { delta } => self.move_browser_focus(delta),
            UiAction::FocusBrowserRow { visible_row } => {
                self.controller.focus_browser_row(visible_row)
            }
            UiAction::ToggleBrowserRowSelection { visible_row } => {
                self.controller.toggle_browser_row_selection(visible_row)
            }
            UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                self.controller.extend_browser_selection_to_row(visible_row)
            }
            UiAction::AddRangeBrowserSelection { visible_row } => {
                self.controller.add_range_browser_selection(visible_row)
            }
            UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                if let Some(target) = browser_focus_target(&self.controller.ui, delta) {
                    self.controller.extend_browser_selection_to_row(target);
                }
            }
            UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                if let Some(target) = browser_focus_target(&self.controller.ui, delta) {
                    self.controller.add_range_browser_selection(target);
                }
            }
            UiAction::ToggleFocusedBrowserRowSelection => {
                self.controller.toggle_focused_selection()
            }
            UiAction::SelectAllBrowserRows => self.controller.select_all_browser_rows(),
            UiAction::SetBrowserSearch { query } => self.controller.set_browser_search(query),
            UiAction::ToggleLoopPlayback => self.controller.toggle_loop(),
            UiAction::SeekWaveform { position_milli } => {
                let normalized = normalized_from_milli(position_milli);
                self.controller.seek_to(normalized);
                self.controller.set_waveform_cursor(normalized);
                self.controller.focus_waveform();
            }
            UiAction::SetWaveformCursor { position_milli } => {
                self.controller
                    .set_waveform_cursor(normalized_from_milli(position_milli));
                self.controller.focus_waveform();
            }
            UiAction::SetWaveformSelectionRange {
                start_milli,
                end_milli,
            } => {
                self.controller
                    .set_selection_range(selection_range_from_milli(start_milli, end_milli));
                self.controller.focus_waveform();
            }
            UiAction::ClearWaveformSelection => {
                self.controller.clear_selection();
                self.controller.focus_waveform();
            }
            UiAction::ZoomWaveform { zoom_in, steps } => {
                self.controller.zoom_waveform_steps_with_factor(
                    zoom_in,
                    u32::from(steps.max(1)),
                    None,
                    None,
                    true,
                    true,
                );
                self.controller.focus_waveform();
            }
            UiAction::ZoomWaveformToSelection => {
                self.controller.zoom_waveform_to_selection();
                self.controller.focus_waveform();
            }
            UiAction::ZoomWaveformFull => {
                self.controller.zoom_waveform_full();
                self.controller.focus_waveform();
            }
            UiAction::Undo => self.controller.undo(),
            UiAction::Redo => self.controller.redo(),
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
