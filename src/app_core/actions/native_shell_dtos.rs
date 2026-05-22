//! Wavecrate-owned native shell projection DTOs.
//!
//! These models describe Wavecrate application state as projected for the current
//! native shell. The runtime adapter in `gui_runtime` converts these app-core
//! DTOs into the Wavecrate-owned native runtime contract consumed by Radiant.

use radiant::gui::chrome;
use radiant::gui::feedback;
use radiant::gui::form;
use radiant::gui::frame;
use radiant::gui::list;
use radiant::gui::panel;
use radiant::gui::range;
use radiant::gui::retained;
use radiant::gui::visualization;

mod automation;
mod browser;
mod motion;
mod retained_segments;
mod waveform;

pub use self::automation::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    GuiAutomationSnapshot,
};
pub use self::browser::{
    BrowserActionsModel, BrowserChromeModel, BrowserPanelModel, BrowserRowModel,
    BrowserRowProcessingState, BrowserTagPillModel, BrowserTagSidebarModel, BrowserTagState,
    PlaybackAgeBucket, PlaybackAgeFilterChip,
};
pub use self::motion::NativeMotionModel;
pub use self::retained_segments::{DirtySegments, SegmentRevisions};
pub use self::waveform::{
    parse_waveform_tempo_number_text, WaveformChannelViewModel, WaveformChromeModel,
    WaveformChromeStateModel, WaveformEditPreviewModel, WaveformFeedbackEventsModel,
    WaveformImagePreviewModel, WaveformMotionModel, WaveformPanelModel, WaveformPresentationModel,
    WaveformSlicePreviewModel, WaveformSurfaceModel, WaveformToolStateModel,
    WaveformTransportModel, WaveformViewportModel,
};

/// Shared storage used by retained app-model snapshots.
pub type RetainedVec<T> = retained::RetainedVec<T>;

/// Frame-level feedback from renderer to host bridge.
pub type FrameBuildResult = frame::FrameBuildResult;

/// Normalized interval with deterministic milli, micro, and nano projections.
pub type NormalizedRangeModel = range::NormalizedRange;

/// Structured footer status content for left/center/right status segments.
pub type StatusBarModel = chrome::StatusSegments;

/// Progress overlay state projected into the native shell.
pub type ProgressOverlayModel = feedback::ProgressOverlay;

/// Drag/drop overlay content for native-shell feedback.
pub type DragOverlayModel = feedback::DragOverlay;

/// Render data for one triage/browser column.
pub type ColumnModel = list::ColumnSummary;

/// Render data for one folder row shown in the sidebar folder tree.
pub type FolderRowKind = list::EditableRowKind;

/// Render data for one folder row shown in the sidebar folder tree.
pub type FolderRowModel = list::EditableTreeRow;

/// Build one folder row projection from Wavecrate folder-browser state.
pub fn folder_row_model(
    label: impl Into<String>,
    detail: impl Into<String>,
    depth: usize,
    selected: bool,
    focused: bool,
    is_root: bool,
    has_children: bool,
    expanded: bool,
) -> FolderRowModel {
    FolderRowModel::from_parts(list::EditableTreeRowParts {
        label: label.into(),
        detail: detail.into(),
        depth,
        selected,
        focused,
        is_root,
        has_children,
        expanded,
    })
}

/// Native folder-action availability consumed by sidebar action surfaces.
pub type FolderActionsModel = list::EditableTreeActions;

/// Stable identifier for one side of the split folder pane surface.
pub type FolderPaneIdModel = panel::SplitPaneSlot;

/// Projected data for one fixed folder pane shown in the sidebar.
pub type FolderPaneModel = panel::SplitPaneTreePanel<FolderRowModel>;

/// Render data for one source row shown in the sidebar.
pub type SourceRowModel = panel::SplitPaneAssignedRow;

/// Render mode label for the map panel.
pub type MapRenderModeModel = visualization::PointRenderMode;

/// Summary of map state consumed by the native shell map tab.
pub type MapPanelModel = visualization::SpatialPanel;

/// Render data for one point shown in the native map canvas.
pub type MapPointModel = visualization::SpatialPoint;

/// Update-check status projected into the native shell.
pub type UpdateStatusModel = feedback::UpdateStatus;

/// Update panel state used by native top-bar actions.
pub type UpdatePanelModel = feedback::UpdatePanel;

/// Modal confirmation prompt projected into the native shell.
pub type ConfirmPromptModel = feedback::ConfirmPrompt<ConfirmPromptKind>;

/// Health state of the compact audio-engine status chip.
pub type AudioEngineChipStateModel = feedback::HealthState;

/// Delete-recovery status for staged folder delete recovery in the sidebar.
pub type FolderRecoveryModel = feedback::RecoverySummary;

/// One selectable item shown inside an audio picker.
pub type AudioOptionItemModel = form::OptionItem<AudioOptionValueModel>;

/// Overview row shown for one audio field inside the options panel.
pub type AudioFieldModel = form::SummaryField;

/// Generic preference/settings panel state used by native overlay projections.
pub type PreferencePanelStateModel<const TOGGLES: usize> = form::PreferencePanelState<TOGGLES>;

impl AppModel {
    /// Audio device and engine panel state for the native shell options view.
    pub fn paired_device_panel(&self) -> &AudioEngineModel {
        &self.audio_engine
    }
}

/// Audio field currently expanded into a picker inside the options panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioPickerTargetModel {
    /// Output host/backend picker.
    OutputHost,
    /// Output device picker.
    OutputDevice,
    /// Output sample-rate picker.
    OutputSampleRate,
    /// Input host/backend picker.
    InputHost,
    /// Input device picker.
    InputDevice,
    /// Input sample-rate picker.
    InputSampleRate,
}

/// Raw value carried by one audio picker option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioOptionValueModel {
    /// Output host identifier, or `None` for the system default.
    OutputHost(Option<String>),
    /// Output device name, or `None` for the host default.
    OutputDevice(Option<String>),
    /// Output sample rate in Hz, or `None` for the device default.
    OutputSampleRate(Option<u32>),
    /// Input host identifier, or `None` for the system default.
    InputHost(Option<String>),
    /// Input device name, or `None` for the host default.
    InputDevice(Option<String>),
    /// Input sample rate in Hz, or `None` for the device default.
    InputSampleRate(Option<u32>),
}

/// Output/input audio engine state projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AudioEngineModel {
    /// Compact chip health state.
    pub chip_state: AudioEngineChipStateModel,
    /// Compact chip label shown in the top-right chrome.
    pub chip_label: String,
    /// Optional detail or error text shown inside the options overview.
    pub detail_label: Option<String>,
    /// Output host summary row.
    pub output_host: AudioFieldModel,
    /// Output device summary row.
    pub output_device: AudioFieldModel,
    /// Output sample-rate summary row.
    pub output_sample_rate: AudioFieldModel,
    /// Input host summary row.
    pub input_host: AudioFieldModel,
    /// Input device summary row.
    pub input_device: AudioFieldModel,
    /// Input sample-rate summary row.
    pub input_sample_rate: AudioFieldModel,
    /// Currently expanded picker, or `None` for the overview.
    pub active_picker: Option<AudioPickerTargetModel>,
    /// Output host choices.
    pub output_host_options: Vec<AudioOptionItemModel>,
    /// Output device choices.
    pub output_device_options: Vec<AudioOptionItemModel>,
    /// Output sample-rate choices.
    pub output_sample_rate_options: Vec<AudioOptionItemModel>,
    /// Input host choices.
    pub input_host_options: Vec<AudioOptionItemModel>,
    /// Input device choices.
    pub input_device_options: Vec<AudioOptionItemModel>,
    /// Input sample-rate choices.
    pub input_sample_rate_options: Vec<AudioOptionItemModel>,
}

impl AudioEngineModel {
    /// Compact health state for the audio engine status chip.
    pub fn status_state(&self) -> AudioEngineChipStateModel {
        self.chip_state
    }

    /// Compact label for the audio engine status chip.
    pub fn status_label(&self) -> &str {
        &self.chip_label
    }

    /// Optional secondary audio-engine detail or error label.
    pub fn detail_label(&self) -> Option<&str> {
        self.detail_label.as_deref()
    }

    /// Primary picker group, currently mapped to output host.
    pub fn primary_group(&self) -> &AudioFieldModel {
        &self.output_host
    }

    /// Primary picker item, currently mapped to output device.
    pub fn primary_item(&self) -> &AudioFieldModel {
        &self.output_device
    }

    /// Primary picker number field, currently mapped to output sample rate.
    pub fn primary_number(&self) -> &AudioFieldModel {
        &self.output_sample_rate
    }

    /// Secondary picker group, currently mapped to input host.
    pub fn secondary_group(&self) -> &AudioFieldModel {
        &self.input_host
    }

    /// Secondary picker item, currently mapped to input device.
    pub fn secondary_item(&self) -> &AudioFieldModel {
        &self.input_device
    }

    /// Secondary picker number field, currently mapped to input sample rate.
    pub fn secondary_number(&self) -> &AudioFieldModel {
        &self.input_sample_rate
    }

    /// Currently active generic paired-picker target.
    pub fn active_picker(&self) -> Option<form::PairedPickerTarget> {
        self.active_picker.map(Into::into)
    }

    /// Option rows for the requested generic paired-picker target.
    pub fn options_for(&self, target: form::PairedPickerTarget) -> &[AudioOptionItemModel] {
        match target {
            form::PairedPickerTarget::PrimaryGroup => &self.output_host_options,
            form::PairedPickerTarget::PrimaryItem => &self.output_device_options,
            form::PairedPickerTarget::PrimaryNumber => &self.output_sample_rate_options,
            form::PairedPickerTarget::SecondaryGroup => &self.input_host_options,
            form::PairedPickerTarget::SecondaryItem => &self.input_device_options,
            form::PairedPickerTarget::SecondaryNumber => &self.input_sample_rate_options,
        }
    }
}

/// Options-panel state projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct OptionsPanelModel {
    /// Whether the panel is currently visible.
    pub visible: bool,
    /// Current default identifier used by auto rename.
    pub default_identifier: String,
    /// Whether input monitoring is enabled.
    pub input_monitoring_enabled: bool,
    /// Whether rating advances browser focus.
    pub advance_after_rating_enabled: bool,
    /// Whether destructive edits skip confirmation.
    pub destructive_yolo_mode_enabled: bool,
    /// Whether waveform scrolling is inverted.
    pub invert_waveform_scroll_enabled: bool,
    /// Short display label for the configured trash folder, when available.
    pub trash_folder_label: Option<String>,
    /// Short display label for the configured audio write format.
    pub audio_write_format_label: Option<String>,
}

impl OptionsPanelModel {
    /// Return this panel's generic preference/settings state.
    pub fn preference_state(&self) -> PreferencePanelStateModel<4> {
        PreferencePanelStateModel::new(
            self.visible,
            self.default_identifier.clone(),
            [
                self.input_monitoring_enabled,
                self.advance_after_rating_enabled,
                self.destructive_yolo_mode_enabled,
                self.invert_waveform_scroll_enabled,
            ],
            self.trash_folder_label.clone(),
        )
    }
}

/// Prompt types that can block interaction in the native shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmPromptKind {
    /// Pending destructive waveform edit prompt.
    DestructiveEdit,
    /// Pending browser rename prompt.
    BrowserRename,
    /// Pending folder rename prompt.
    FolderRename,
    /// Pending folder creation prompt.
    FolderCreate,
    /// Pending retained folder-delete restore prompt.
    RestoreRetainedFolderDeletes,
    /// Pending retained folder-delete purge prompt.
    PurgeRetainedFolderDeletes,
    /// Pending options-panel default-identifier prompt.
    OptionsDefaultIdentifier,
}

/// Logical focus buckets projected into the native runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FocusContextModel {
    /// No UI surface currently owns keyboard focus.
    #[default]
    None,
    /// The waveform viewer handles navigation and shortcuts.
    Waveform,
    /// The sample browser handles row navigation and browser shortcuts.
    SampleBrowser,
    /// The folder tree handles folder navigation and folder shortcuts.
    SourceFolders,
    /// The source list handles source-row navigation and shortcuts.
    SourcesList,
}

/// Sidebar model for source browsing controls.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SourcesPanelModel {
    /// Header text for the source panel.
    pub header: String,
    /// Active source-search query.
    pub search_query: String,
    /// Pane that currently drives browser and waveform state.
    pub active_folder_pane: FolderPaneIdModel,
    /// Upper fixed folder pane.
    pub upper_folder_pane: FolderPaneModel,
    /// Lower fixed folder pane.
    pub lower_folder_pane: FolderPaneModel,
    /// Active folder-search query.
    pub tree_search_query: String,
    /// Whether the folder browser currently includes empty on-disk folders.
    pub show_all_items: bool,
    /// Whether the folder-visibility toggle is currently actionable.
    pub can_toggle_show_all_items: bool,
    /// Whether folder filtering includes descendant files in a flattened list.
    pub flattened_view: bool,
    /// Whether the folder flattened-view toggle is currently actionable.
    pub can_toggle_flattened_view: bool,
    /// Selected row index, if any.
    pub selected_row: Option<usize>,
    /// Source row currently hydrating in the background, if any.
    pub loading_row: Option<usize>,
    /// Source row currently running a background file or folder mutation, if any.
    pub mutation_busy_row: Option<usize>,
    /// Focused folder row index, if any.
    pub focused_tree_row: Option<usize>,
    /// Rows to render in the source panel.
    pub rows: RetainedVec<SourceRowModel>,
    /// Folder rows to render in the folder browser section.
    pub tree_rows: RetainedVec<FolderRowModel>,
    /// Folder action availability for native sidebar controls.
    pub tree_actions: FolderActionsModel,
    /// Folder delete-recovery summary for native sidebar status.
    pub recovery: FolderRecoveryModel,
}

impl SourcesPanelModel {
    /// Borrow one pane model by id.
    pub fn folder_pane(&self, pane: FolderPaneIdModel) -> &FolderPaneModel {
        pane.select(&self.upper_folder_pane, &self.lower_folder_pane)
    }

    /// Borrow the pane that currently drives browser and waveform state.
    pub fn active_folder_pane_model(&self) -> &FolderPaneModel {
        self.folder_pane(self.active_folder_pane)
    }

    /// Return this source/sidebar model as a generic split-pane sidebar state.
    pub fn split_pane_sidebar(
        &self,
    ) -> panel::SplitPaneSidebarState<SourceRowModel, FolderRowModel> {
        panel::SplitPaneSidebarState {
            header: self.header.clone(),
            search_query: self.search_query.clone(),
            active_pane: self.active_folder_pane,
            upper_pane: self.upper_folder_pane.clone(),
            lower_pane: self.lower_folder_pane.clone(),
            tree_search_query: self.tree_search_query.clone(),
            show_all_items: self.show_all_items,
            can_toggle_show_all_items: self.can_toggle_show_all_items,
            flattened_view: self.flattened_view,
            can_toggle_flattened_view: self.can_toggle_flattened_view,
            selected_row: self.selected_row,
            loading_row: self.loading_row,
            mutation_busy_row: self.mutation_busy_row,
            focused_tree_row: self.focused_tree_row,
            rows: self.rows.clone(),
            tree_rows: self.tree_rows.clone(),
            tree_actions: self.tree_actions.clone(),
            recovery: self.recovery.clone(),
        }
    }
}

/// Snapshot of Wavecrate state required by the native shell renderer.
#[derive(Clone, Debug, PartialEq)]
pub struct AppModel {
    /// Main title rendered in the top bar.
    pub title: String,
    /// Backend description shown in top-bar metadata.
    pub backend_label: String,
    /// Sidebar header label.
    pub sources_label: String,
    /// Footer status text.
    pub status_text: String,
    /// Structured footer status segments used by the native shell footer.
    pub status: StatusBarModel,
    /// Output/input audio engine state rendered in the top-right chrome and options panel.
    pub audio_engine: AudioEngineModel,
    /// Browser action availability for native action surfaces.
    pub browser_actions: BrowserActionsModel,
    /// Options-panel overlay projection.
    pub options_panel: OptionsPanelModel,
    /// Progress overlay projection.
    pub progress_overlay: ProgressOverlayModel,
    /// Modal confirm prompt projection.
    pub confirm_prompt: ConfirmPromptModel,
    /// Drag/drop overlay projection.
    pub drag_overlay: DragOverlayModel,
    /// Logical triage/browser columns.
    pub columns: [ColumnModel; 3],
    /// Selected column index (0..=2).
    pub selected_column: usize,
    /// Master output volume normalized to `0.0..=1.0`.
    pub volume: f32,
    /// Whether transport/animation should be considered running.
    pub transport_running: bool,
    /// Source panel model consumed by the native renderer.
    pub sources: SourcesPanelModel,
    /// Browser panel summary consumed by the native renderer.
    pub browser: BrowserPanelModel,
    /// Browser chrome labels consumed by native tabs/toolbar/footer text.
    pub browser_chrome: BrowserChromeModel,
    /// Map panel summary consumed by the native renderer.
    pub map: MapPanelModel,
    /// Waveform panel summary consumed by the native renderer.
    pub waveform: WaveformPanelModel,
    /// Waveform chrome labels consumed by the native waveform header.
    pub waveform_chrome: WaveformChromeModel,
    /// Update surface summary consumed by the native top bar.
    pub update: UpdatePanelModel,
    /// Current keyboard focus bucket used for contextual native key routing.
    pub focus_context: FocusContextModel,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            title: String::from(crate::gui_runtime::DEFAULT_NATIVE_WINDOW_TITLE),
            backend_label: String::from("backend: native_vello"),
            sources_label: String::from("Sources"),
            status_text: String::new(),
            status: StatusBarModel {
                left: String::new(),
                center: String::from("rows: 0 | selected: 0 | anchor: - | search: -"),
                right: String::from("col: 2/3"),
            },
            audio_engine: AudioEngineModel::default(),
            browser_actions: BrowserActionsModel::default(),
            options_panel: OptionsPanelModel::default(),
            progress_overlay: ProgressOverlayModel::default(),
            confirm_prompt: ConfirmPromptModel::default(),
            drag_overlay: DragOverlayModel::default(),
            columns: [
                ColumnModel::new("Trash", 0),
                ColumnModel::new("Samples", 0),
                ColumnModel::new("Keep", 0),
            ],
            selected_column: 1,
            volume: 1.0,
            transport_running: true,
            sources: SourcesPanelModel {
                header: String::from("Sources"),
                search_query: String::new(),
                active_folder_pane: FolderPaneIdModel::Upper,
                upper_folder_pane: FolderPaneModel {
                    pane: FolderPaneIdModel::Upper,
                    title: String::from("Upper"),
                    ..FolderPaneModel::default()
                },
                lower_folder_pane: FolderPaneModel {
                    pane: FolderPaneIdModel::Lower,
                    title: String::from("Lower"),
                    ..FolderPaneModel::default()
                },
                tree_search_query: String::new(),
                show_all_items: false,
                can_toggle_show_all_items: false,
                flattened_view: false,
                can_toggle_flattened_view: false,
                selected_row: None,
                loading_row: None,
                mutation_busy_row: None,
                focused_tree_row: None,
                rows: RetainedVec::new(),
                tree_rows: RetainedVec::new(),
                tree_actions: FolderActionsModel::default(),
                recovery: FolderRecoveryModel::default(),
            },
            browser: BrowserPanelModel::default(),
            browser_chrome: BrowserChromeModel::default(),
            map: MapPanelModel::default(),
            waveform: WaveformPanelModel::default(),
            waveform_chrome: WaveformChromeModel::default(),
            update: UpdatePanelModel::default(),
            focus_context: FocusContextModel::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_waveform_tempo_number_text, WaveformPanelModel};

    #[test]
    fn waveform_panel_default_bpm_grid_origin_is_zero() {
        assert_eq!(WaveformPanelModel::default().bpm_grid_origin_micros, 0);
    }

    #[test]
    fn options_panel_projects_generic_preference_state() {
        let model = super::OptionsPanelModel {
            visible: true,
            default_identifier: String::from("portal"),
            input_monitoring_enabled: true,
            advance_after_rating_enabled: false,
            destructive_yolo_mode_enabled: true,
            invert_waveform_scroll_enabled: false,
            trash_folder_label: Some(String::from("Trash")),
            audio_write_format_label: Some(String::from("Source rate, 32-bit float")),
        };
        let preferences = model.preference_state();

        assert!(preferences.visible);
        assert_eq!(preferences.primary_text_value, "portal");
        assert_eq!(preferences.toggles, [true, false, true, false]);
        assert_eq!(preferences.auxiliary_label.as_deref(), Some("Trash"));
    }

    #[test]
    fn waveform_panel_projects_generic_feedback_events() {
        let model = WaveformPanelModel {
            selection_export_flash_nonce: 11,
            selection_export_failure_flash_nonce: 12,
            edit_selection_apply_flash_nonce: 13,
            ..WaveformPanelModel::default()
        };
        let events = model.feedback_events();

        assert_eq!(events.primary_success_nonce, 11);
        assert_eq!(events.primary_failure_nonce, 12);
        assert_eq!(events.secondary_success_nonce, 13);
    }

    #[test]
    fn waveform_panel_projects_generic_presentation_state() {
        let model = WaveformPanelModel {
            beat_step_micros: Some(100_000),
            bpm_grid_origin_micros: 50_000,
            loop_enabled: true,
            tempo_label: Some(String::from("150 BPM")),
            zoom_label: Some(String::from("8x")),
            ..WaveformPanelModel::default()
        };
        let presentation = model.presentation();

        assert_eq!(presentation.guide_step_micros, Some(100_000));
        assert_eq!(presentation.guide_origin_micros, 50_000);
        assert!(presentation.repeat_enabled);
        assert_eq!(presentation.primary_label.as_deref(), Some("150 BPM"));
        assert_eq!(presentation.viewport_label.as_deref(), Some("8x"));
    }

    #[test]
    fn waveform_panel_projects_generic_timeline_surface_state() {
        let model = WaveformPanelModel {
            view_start_micros: 125_000,
            playhead_micros: Some(250_250),
            selection_export_failure_flash_nonce: 5,
            loop_enabled: true,
            loaded_label: Some(String::from("Loaded")),
            slices: vec![super::WaveformSlicePreviewModel {
                range: super::NormalizedRangeModel::new(100, 200),
                selected: true,
                focused: false,
                marked_for_export: false,
                review_candidate: false,
                review_exempted: false,
            }],
            ..WaveformPanelModel::default()
        };
        let surface = model.timeline_surface();

        assert_eq!(surface.viewport.start_micros, 125_000);
        assert_eq!(surface.transport.resolved_playhead_micros(), Some(250_250));
        assert_eq!(surface.feedback_events.primary_failure_nonce, 5);
        assert!(surface.presentation.repeat_enabled);
        assert_eq!(
            surface.raster_preview.loaded_label.as_deref(),
            Some("Loaded")
        );
        assert_eq!(surface.markers.len(), 1);
    }

    #[test]
    fn native_motion_projects_generic_timeline_motion_state() {
        let model = super::NativeMotionModel {
            transport_running: true,
            map_active: false,
            active_rating_filters: [false; 8],
            active_playback_age_filters: [false; 3],
            marked_filter_active: false,
            waveform_selection_milli: Some(super::NormalizedRangeModel::new(100, 400)),
            waveform_slices: Vec::new(),
            waveform_selection_export_flash_nonce: 11,
            waveform_selection_export_failure_flash_nonce: 12,
            waveform_edit_selection_apply_flash_nonce: 13,
            waveform_edit_selection_milli: None,
            waveform_edit_fade_in_end_milli: Some(120),
            waveform_edit_fade_in_end_micros: Some(120_000),
            waveform_edit_fade_in_mute_start_milli: None,
            waveform_edit_fade_in_mute_start_micros: None,
            waveform_edit_fade_in_curve_milli: Some(200),
            waveform_edit_fade_out_start_milli: None,
            waveform_edit_fade_out_start_micros: None,
            waveform_edit_fade_out_mute_end_milli: Some(390),
            waveform_edit_fade_out_mute_end_micros: Some(390_000),
            waveform_edit_fade_out_curve_milli: Some(800),
            waveform_loop_enabled: true,
            waveform_loop_lock_enabled: true,
            waveform_cursor_milli: Some(150),
            waveform_playhead_milli: Some(250),
            waveform_playhead_micros: Some(250_500),
            waveform_view_start_milli: 10,
            waveform_view_end_milli: 900,
            waveform_view_start_micros: 10_000,
            waveform_view_end_micros: 900_000,
            waveform_view_start_nanos: 10_000_000,
            waveform_view_end_nanos: 900_000_000,
            waveform_tempo_label: Some(String::from("128 BPM")),
            waveform_zoom_label: Some(String::from("4x")),
            waveform_loaded_label: Some(String::from("Loaded")),
            waveform_loading: true,
            waveform_image_signature: Some(42),
            waveform_transport_hint: String::from("playing"),
            waveform_compare_anchor_available: true,
            waveform_compare_anchor_label: Some(String::from("A")),
            waveform_channel_view: super::WaveformChannelViewModel::Stereo,
            waveform_normalized_audition_enabled: true,
            waveform_bpm_snap_enabled: true,
            waveform_relative_bpm_grid_enabled: false,
            waveform_transient_snap_enabled: true,
            waveform_transient_markers_enabled: true,
            waveform_slice_mode_enabled: false,
            waveform_exact_duplicate_cleanup_available: true,
            status_right: String::from("ready"),
        };

        let motion = model.timeline_motion();

        assert!(motion.transport_running);
        assert_eq!(motion.surface.viewport.start_micros, 10_000);
        assert_eq!(
            motion.surface.transport.resolved_playhead_micros(),
            Some(250_500)
        );
        assert_eq!(motion.surface.feedback_events.primary_success_nonce, 11);
        assert!(motion.surface.presentation.repeat_enabled);
        assert_eq!(
            motion.surface.raster_preview.loaded_label.as_deref(),
            Some("Loaded")
        );
        assert_eq!(motion.chrome.status_hint, "playing");
        assert_eq!(
            motion.chrome.channel_view,
            super::WaveformChannelViewModel::Stereo
        );
        assert!(motion.tools.lock_enabled);
        assert!(motion.tools.cleanup_available);
    }

    #[test]
    fn parse_waveform_tempo_number_text_accepts_integer_and_fractional_labels() {
        assert_eq!(
            parse_waveform_tempo_number_text("128 BPM"),
            Some(String::from("128"))
        );
        assert_eq!(
            parse_waveform_tempo_number_text("128.5 BPM"),
            Some(String::from("128.5"))
        );
    }

    #[test]
    fn parse_waveform_tempo_number_text_rejects_empty_and_invalid_labels() {
        assert_eq!(parse_waveform_tempo_number_text(""), None);
        assert_eq!(parse_waveform_tempo_number_text("0 BPM"), None);
        assert_eq!(parse_waveform_tempo_number_text("-1 BPM"), None);
        assert_eq!(parse_waveform_tempo_number_text("fast BPM"), None);
    }
}
