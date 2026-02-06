//! Native runtime bridge between sempal controller state and `radiant`.

use crate::{
    audio::AudioPlayer,
    egui_app::{
        controller::EguiController,
        state::{TriageFlagColumn, UiState},
        view_model,
    },
    waveform::WaveformRenderer,
};
use radiant::app::{
    AppModel, BrowserPanelModel, BrowserRowModel, ColumnModel, FrameBuildResult, NativeAppBridge,
    NormalizedRangeModel, SourceRowModel, SourcesPanelModel, UiAction, WaveformPanelModel,
};
use std::collections::HashSet;
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
        let selected_column = selected_column_index(&self.controller.ui);
        let transport_running = self.controller.is_playing();
        let sources = project_sources_model(&self.controller.ui);
        let status_text = self.controller.ui.status.text.clone();
        let column_counts = [
            self.controller.ui.browser.trash.len(),
            self.controller.ui.browser.neutral.len(),
            self.controller.ui.browser.keep.len(),
        ];
        let waveform = project_waveform_model(&self.controller.ui);
        let browser = project_browser_model(&mut self.controller);
        AppModel {
            title: String::from("Sempal"),
            backend_label: String::from("backend: native_vello"),
            sources_label: format!("Sources ({})", sources.rows.len()),
            status_text,
            columns: [
                ColumnModel::new("Trash", column_counts[0]),
                ColumnModel::new("Samples", column_counts[1]),
                ColumnModel::new("Keep", column_counts[2]),
            ],
            selected_column,
            transport_running,
            sources,
            browser,
            waveform,
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

    fn move_browser_focus(&mut self, delta: i8) {
        let Some(target) = self.browser_focus_target(delta) else {
            return;
        };
        self.controller.focus_browser_row(target);
    }

    fn browser_focus_target(&self, delta: i8) -> Option<usize> {
        let visible_count = self.controller.ui.browser.visible.len();
        if visible_count == 0 {
            return None;
        }
        let base = self
            .controller
            .ui
            .browser
            .selected_visible
            .unwrap_or(0)
            .min(visible_count - 1);
        Some((base as isize + delta as isize).clamp(0, visible_count as isize - 1) as usize)
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
            UiAction::FocusLoadedSampleInBrowser => self.controller.focus_loaded_sample_in_browser(),
            UiAction::FocusBrowserSearch => self.controller.focus_browser_search(),
            UiAction::FocusFolderSearch => self.controller.focus_folder_search(),
            UiAction::SelectSourceRow { index } => self.controller.select_source_by_index(index),
            UiAction::MoveBrowserFocus { delta } => self.move_browser_focus(delta),
            UiAction::FocusBrowserRow { visible_row } => self.controller.focus_browser_row(visible_row),
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
                if let Some(target) = self.browser_focus_target(delta) {
                    self.controller.extend_browser_selection_to_row(target);
                }
            }
            UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                if let Some(target) = self.browser_focus_target(delta) {
                    self.controller.add_range_browser_selection(target);
                }
            }
            UiAction::ToggleFocusedBrowserRowSelection => self.controller.toggle_focused_selection(),
            UiAction::SelectAllBrowserRows => self.controller.select_all_browser_rows(),
            UiAction::SetBrowserSearch { query } => self.controller.set_browser_search(query),
            UiAction::ToggleLoopPlayback => self.controller.toggle_loop(),
            UiAction::SeekWaveform { position_milli } => {
                let normalized = milli_to_normalized(position_milli);
                self.controller.seek_to(normalized);
                self.controller.set_waveform_cursor(normalized);
                self.controller.focus_waveform();
            }
            UiAction::SetWaveformCursor { position_milli } => {
                self.controller.set_waveform_cursor(milli_to_normalized(position_milli));
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

fn project_sources_model(ui: &UiState) -> SourcesPanelModel {
    SourcesPanelModel {
        header: format!("Sources ({})", ui.sources.rows.len()),
        search_query: ui.sources.folders.search_query.clone(),
        selected_row: ui.sources.selected,
        rows: ui
            .sources
            .rows
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                SourceRowModel::new(
                    row.name.clone(),
                    row.path.clone(),
                    ui.sources
                        .selected
                        .is_some_and(|selected| selected == row_index),
                    row.missing,
                )
            })
            .collect(),
    }
}

fn project_browser_model(controller: &mut EguiController) -> BrowserPanelModel {
    let visible = controller.ui.browser.visible.clone();
    let selected_visible_row = controller.ui.browser.selected_visible;
    let selected_path_count = controller.ui.browser.selected_paths.len();
    let search_query = controller.ui.browser.search_query.clone();
    let busy = controller.ui.browser.search_busy;
    let focused_sample_label = controller
        .ui
        .loaded_wav
        .as_deref()
        .map(view_model::sample_display_label);
    let anchor_visible_row = controller.ui.browser.selection_anchor_visible;
    let selected_paths: HashSet<_> = controller.ui.browser.selected_paths.iter().cloned().collect();

    let mut rows = Vec::new();
    let visible_count = visible.len();
    let rendered = visible_count.min(MAX_RENDERED_BROWSER_ROWS);
    for visible_row in 0..rendered {
        let Some(absolute_index) = visible.get(visible_row) else {
            continue;
        };
        if let Some(entry) = controller.wav_entry(absolute_index) {
            let selected = selected_paths.contains(&entry.relative_path);
            rows.push(BrowserRowModel::new(
                visible_row,
                view_model::sample_display_label(&entry.relative_path),
                browser_column_index(entry.tag),
                selected,
                selected_visible_row.is_some_and(|focused| focused == visible_row),
            ));
        } else {
            rows.push(BrowserRowModel::new(
                visible_row,
                format!("row {}", visible_row + 1),
                1,
                false,
                selected_visible_row.is_some_and(|focused| focused == visible_row),
            ));
        }
    }

    BrowserPanelModel {
        visible_count,
        selected_visible_row,
        selected_path_count,
        search_query,
        busy,
        focused_sample_label,
        anchor_visible_row,
        rows,
    }
}

fn project_waveform_model(ui: &UiState) -> WaveformPanelModel {
    WaveformPanelModel {
        loaded_label: ui.loaded_wav.as_deref().map(view_model::sample_display_label),
        cursor_milli: ui.waveform.cursor.map(normalized_to_milli),
        playhead_milli: ui
            .waveform
            .playhead
            .visible
            .then_some(normalized_to_milli(ui.waveform.playhead.position)),
        selection_milli: ui.waveform.selection.map(|selection| {
            NormalizedRangeModel::new(
                normalized_to_milli(selection.start()),
                normalized_to_milli(selection.end()),
            )
        }),
        view_start_milli: normalized64_to_milli(ui.waveform.view.start),
        view_end_milli: normalized64_to_milli(ui.waveform.view.end),
        loop_enabled: ui.waveform.loop_enabled,
    }
}

const MAX_RENDERED_BROWSER_ROWS: usize = 48;

fn browser_column_index(tag: crate::sample_sources::Rating) -> usize {
    if tag.is_trash() {
        0
    } else if tag.is_keep() {
        2
    } else {
        1
    }
}

fn normalized_to_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn normalized64_to_milli(value: f64) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn milli_to_normalized(value: u16) -> f32 {
    (value.min(1000) as f32) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_column_defaults_to_middle_column_without_selection() {
        let ui = UiState::default();
        assert_eq!(selected_column_index(&ui), 1);
    }

    #[test]
    fn normalized_to_milli_clamps_bounds() {
        assert_eq!(normalized_to_milli(-0.3), 0);
        assert_eq!(normalized_to_milli(0.455), 455);
        assert_eq!(normalized_to_milli(1.7), 1000);
    }

    #[test]
    fn browser_column_index_maps_rating_buckets() {
        assert_eq!(browser_column_index(crate::sample_sources::Rating::TRASH_1), 0);
        assert_eq!(browser_column_index(crate::sample_sources::Rating::NEUTRAL), 1);
        assert_eq!(browser_column_index(crate::sample_sources::Rating::KEEP_1), 2);
    }
}
