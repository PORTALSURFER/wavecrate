//! Sempal shell and overlay models used by the legacy Radiant compatibility path.

pub use crate::gui::feedback::HealthState as StatusChipStateModel;
pub use crate::gui::feedback::PromptIntent as ConfirmPromptKind;
pub use crate::gui::form::PairedPickerTarget as PairedPickerTargetModel;
pub use crate::gui::form::PairedStatusPanel as PairedDevicePanelModel;
pub use crate::gui::form::SummaryField as SummaryFieldModel;
pub use crate::app_core::actions::{
    NativeDragOverlayModel as DragOverlayModel,
    NativeProgressOverlayModel as ProgressOverlayModel,
    NativeStatusBarModel as StatusBarModel,
    NativeUpdatePanelModel as UpdatePanelModel,
    NativeUpdateStatusModel as UpdateStatusModel,
};

use super::{
    BrowserActionsModel, BrowserChromeModel, BrowserPanelModel, ColumnModel, FocusContextModel,
    FolderActionsModel, FolderPaneIdModel, FolderPaneModel, FolderRecoveryModel, MapPanelModel,
    OptionsPanelModel, SourcesPanelModel, WaveformChromeModel, WaveformPanelModel,
};

/// Raw value carried by one paired-picker option.
pub type PairedPickerValueModel = crate::gui::form::PairedPickerValue<String, u32>;

/// One selectable item shown inside a paired picker.
pub type PairedPickerOptionModel = crate::gui::form::OptionItem<PairedPickerValueModel>;

/// Modal confirmation prompt projected into the native shell.
pub type ConfirmPromptModel = crate::gui::feedback::ConfirmPrompt<ConfirmPromptKind>;

/// Snapshot of app state required by the native shell renderer.
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
    /// Paired device/status state rendered in the top-right chrome and options panel.
    pub paired_device: PairedDevicePanelModel,
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
                center: String::from("rows: 0 | selected: 0 | anchor: — | search: —"),
                right: String::from("col: 2/3"),
            },
            paired_device: PairedDevicePanelModel::default(),
            browser_actions: BrowserActionsModel::default(),
            options_panel: OptionsPanelModel::default(),
            progress_overlay: ProgressOverlayModel::default(),
            confirm_prompt: ConfirmPromptModel::default(),
            drag_overlay: DragOverlayModel::default(),
            columns: [
                ColumnModel::new("Trash", 0),
                ColumnModel::new("Items", 0),
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
                rows: super::RetainedVec::new(),
                tree_rows: super::RetainedVec::new(),
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

impl AppModel {
    /// Return the generic paired-device/status panel projection.
    pub fn paired_device_panel(&self) -> &PairedDevicePanelModel {
        &self.paired_device
    }
}
