//! Top-level native shell application projection DTOs.

use radiant::gui::chrome;
use radiant::gui::feedback;
use radiant::gui::list;
use radiant::gui::visualization;

use super::{
    AudioEngineModel, BrowserActionsModel, BrowserChromeModel, BrowserPanelModel,
    FolderActionsModel, FolderPaneIdModel, FolderPaneModel, FolderRecoveryModel, OptionsPanelModel,
    RetainedVec, SourcesPanelModel, WaveformChromeModel, WaveformPanelModel,
};

/// Structured footer status content for left/center/right status segments.
pub type StatusBarModel = chrome::StatusSegments;

/// Progress overlay state projected into the native shell.
pub type ProgressOverlayModel = feedback::ProgressOverlay;

/// Drag/drop overlay content for native-shell feedback.
pub type DragOverlayModel = feedback::DragOverlay;

/// Render data for one triage/browser column.
pub type ColumnModel = list::ColumnSummary;

/// Render mode label for the map panel.
pub type MapRenderModeModel = visualization::PointRenderMode;

/// Summary of map state consumed by the native shell map tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapPanelModel {
    /// Whether the map panel is currently active.
    pub active: bool,
    /// Human-readable map summary line.
    pub summary: String,
    /// Legend/status label for render mode and point density.
    pub legend_label: String,
    /// Selection/focus label for the currently highlighted sample.
    pub selection_label: String,
    /// Hover label for the currently hovered sample, when any.
    pub hover_label: String,
    /// Cluster summary label for projected points.
    pub cluster_label: String,
    /// Viewport label describing zoom/pan state.
    pub viewport_label: String,
    /// Optional map error text.
    pub error: Option<String>,
    /// Current point render mode.
    pub render_mode: MapRenderModeModel,
    /// Host item id currently selected in map state, when any.
    pub selected_item_id: Option<String>,
    /// Host item id currently focused from a related list, when any.
    pub focused_item_id: Option<String>,
    /// Points available for rendering in normalized map coordinates.
    pub points: std::sync::Arc<[MapPointModel]>,
}

impl Default for MapPanelModel {
    fn default() -> Self {
        Self {
            active: false,
            summary: String::new(),
            legend_label: String::new(),
            selection_label: String::new(),
            hover_label: String::new(),
            cluster_label: String::new(),
            viewport_label: String::new(),
            error: None,
            render_mode: MapRenderModeModel::default(),
            selected_item_id: None,
            focused_item_id: None,
            points: std::sync::Arc::default(),
        }
    }
}

impl From<MapPanelModel> for visualization::SpatialPanel {
    fn from(value: MapPanelModel) -> Self {
        Self {
            status: visualization::SpatialPanelStatus {
                active: value.active,
                summary: value.summary,
                error: value.error,
            },
            labels: visualization::SpatialPanelLabels {
                legend_label: value.legend_label,
                selection_label: value.selection_label,
                hover_label: value.hover_label,
                cluster_label: value.cluster_label,
                viewport_label: value.viewport_label,
            },
            selection: visualization::SpatialPanelSelection {
                selected_item_id: value.selected_item_id,
                focused_item_id: value.focused_item_id,
            },
            points: visualization::SpatialPanelPoints {
                render_mode: value.render_mode,
                points: value.points,
            },
        }
    }
}

impl From<visualization::SpatialPanel> for MapPanelModel {
    fn from(value: visualization::SpatialPanel) -> Self {
        Self {
            active: value.status.active,
            summary: value.status.summary,
            legend_label: value.labels.legend_label,
            selection_label: value.labels.selection_label,
            hover_label: value.labels.hover_label,
            cluster_label: value.labels.cluster_label,
            viewport_label: value.labels.viewport_label,
            error: value.status.error,
            render_mode: value.points.render_mode,
            selected_item_id: value.selection.selected_item_id,
            focused_item_id: value.selection.focused_item_id,
            points: value.points.points,
        }
    }
}

/// Render data for one point shown in the native map canvas.
pub type MapPointModel = visualization::SpatialPoint;

/// Update-check status projected into the native shell.
pub type UpdateStatusModel = feedback::UpdateStatus;

/// Update panel state used by native top-bar actions.
pub type UpdatePanelModel = feedback::UpdatePanel;

/// Modal confirmation prompt projected into the native shell.
pub type ConfirmPromptModel = feedback::ConfirmPrompt<ConfirmPromptKind>;

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
