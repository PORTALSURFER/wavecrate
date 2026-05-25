//! Wavecrate shell and overlay models used by the runtime compatibility contract.

pub use crate::app_core::actions::{
    NativeDragOverlayModel as DragOverlayModel, NativeProgressOverlayModel as ProgressOverlayModel,
    NativeStatusBarModel as StatusBarModel, NativeUpdatePanelModel as UpdatePanelModel,
    NativeUpdateStatusModel as UpdateStatusModel,
};
pub use crate::gui::feedback::HealthState as StatusChipStateModel;
pub use crate::gui::feedback::PromptIntent as ConfirmPromptKind;
pub use crate::gui::form::PairedPickerTarget as PairedPickerTargetModel;
pub use crate::gui::form::SummaryField as SummaryFieldModel;

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

/// Output/input paired device state rendered by the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PairedDevicePanelModel {
    /// Compact chip health state.
    pub status_state: StatusChipStateModel,
    /// Compact chip label shown in surrounding chrome.
    pub status_label: String,
    /// Optional detail or error text shown inside the panel overview.
    pub detail_label: Option<String>,
    /// Primary-side group summary row.
    pub primary_group: SummaryFieldModel,
    /// Primary-side item summary row.
    pub primary_item: SummaryFieldModel,
    /// Primary-side numeric-setting summary row.
    pub primary_number: SummaryFieldModel,
    /// Secondary-side group summary row.
    pub secondary_group: SummaryFieldModel,
    /// Secondary-side item summary row.
    pub secondary_item: SummaryFieldModel,
    /// Secondary-side numeric-setting summary row.
    pub secondary_number: SummaryFieldModel,
    /// Currently expanded picker, or `None` for the overview.
    pub active_picker: Option<PairedPickerTargetModel>,
    /// Primary-side group choices.
    pub primary_group_options: Vec<PairedPickerOptionModel>,
    /// Primary-side item choices.
    pub primary_item_options: Vec<PairedPickerOptionModel>,
    /// Primary-side numeric-setting choices.
    pub primary_number_options: Vec<PairedPickerOptionModel>,
    /// Secondary-side group choices.
    pub secondary_group_options: Vec<PairedPickerOptionModel>,
    /// Secondary-side item choices.
    pub secondary_item_options: Vec<PairedPickerOptionModel>,
    /// Secondary-side numeric-setting choices.
    pub secondary_number_options: Vec<PairedPickerOptionModel>,
}

impl PairedDevicePanelModel {
    /// Return the compact status chip state.
    pub fn status_state(&self) -> StatusChipStateModel {
        self.status_state
    }

    /// Return the compact status chip label.
    pub fn status_label(&self) -> &str {
        &self.status_label
    }

    /// Return optional detail text for the overview.
    pub fn detail_label(&self) -> Option<&str> {
        self.detail_label.as_deref()
    }

    /// Return the primary group summary field.
    pub fn primary_group(&self) -> &SummaryFieldModel {
        &self.primary_group
    }

    /// Return the primary item summary field.
    pub fn primary_item(&self) -> &SummaryFieldModel {
        &self.primary_item
    }

    /// Return the primary numeric-setting summary field.
    pub fn primary_number(&self) -> &SummaryFieldModel {
        &self.primary_number
    }

    /// Return the secondary group summary field.
    pub fn secondary_group(&self) -> &SummaryFieldModel {
        &self.secondary_group
    }

    /// Return the secondary item summary field.
    pub fn secondary_item(&self) -> &SummaryFieldModel {
        &self.secondary_item
    }

    /// Return the secondary numeric-setting summary field.
    pub fn secondary_number(&self) -> &SummaryFieldModel {
        &self.secondary_number
    }

    /// Return the currently expanded picker target, if any.
    pub fn active_picker(&self) -> Option<PairedPickerTargetModel> {
        self.active_picker
    }

    /// Return the option list associated with a paired-picker target.
    pub fn options_for(&self, target: PairedPickerTargetModel) -> &[PairedPickerOptionModel] {
        match target {
            PairedPickerTargetModel::PrimaryGroup => &self.primary_group_options,
            PairedPickerTargetModel::PrimaryItem => &self.primary_item_options,
            PairedPickerTargetModel::PrimaryNumber => &self.primary_number_options,
            PairedPickerTargetModel::SecondaryGroup => &self.secondary_group_options,
            PairedPickerTargetModel::SecondaryItem => &self.secondary_item_options,
            PairedPickerTargetModel::SecondaryNumber => &self.secondary_number_options,
        }
    }
}

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
    /// Browser sidebar metadata-facet filter state consumed by Wavecrate sidebar chrome.
    pub sidebar_filters: crate::app_core::state::BrowserSidebarFilterState,
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
            sidebar_filters: Default::default(),
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
