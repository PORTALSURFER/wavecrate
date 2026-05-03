use super::*;

/// Cursor-move effect classification used by runtime overlay invalidation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CursorMoveEffect {
    /// Pointer movement did not change observable hover state.
    None,
    /// Only waveform hover-cursor position changed.
    WaveformHoverOnly,
    /// Hovered node and/or hovered row changed.
    GeneralOverlay,
}

/// Stable hover-target identifier for waveform-toolbar tooltip hints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaveformToolbarHoverHint {
    /// Channel-view toggle that swaps between mono and split stereo.
    ChannelView,
    /// Normalized audition toggle.
    NormalizedAudition,
    /// Current playback BPM value display.
    BpmValue,
    /// BPM snap toggle.
    BpmSnap,
    /// Selection-relative BPM grid toggle.
    RelativeBpmGrid,
    /// Transient snap toggle.
    TransientSnap,
    /// Transient marker visibility toggle.
    ShowTransients,
    /// Slice-mode toggle.
    SliceMode,
    /// Silence-split slice detection tool.
    SilenceSplit,
    /// Exact duplicate window detection tool.
    ExactDedupe,
    /// Exact duplicate window cleanup tool.
    CleanDuplicates,
    /// Loop playback toggle.
    Loop,
    /// Compare-anchor replay action.
    Compare,
    /// Stop transport action.
    Stop,
    /// Transport toggle action.
    Play,
    /// Record action (currently disabled).
    Record,
}

/// Stable hover target for waveform selection/edit resize edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaveformResizeHoverEdge {
    /// Start edge of the playback selection.
    SelectionStart,
    /// End edge of the playback selection.
    SelectionEnd,
    /// Start edge of the edit selection.
    EditSelectionStart,
    /// End edge of the edit selection.
    EditSelectionEnd,
}

/// Compact state-overlay fingerprint for change detection in runtime caches.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StateOverlayFingerprint {
    /// Selected browser column index.
    pub selected_column: usize,
    /// Current hovered shell node kind.
    pub hovered: Option<ShellNodeKind>,
    /// Hovered browser row in visible-row space.
    pub hovered_browser_visible_row: Option<usize>,
    /// Hovered folder pane, when the pointer is over a folder pane.
    pub hovered_folder_pane: Option<crate::compat_app_contract::FolderPaneIdModel>,
    /// Hovered folder row by rendered sidebar row index.
    pub hovered_folder_row_index: Option<usize>,
    /// Hovered waveform-toolbar hint target.
    pub hovered_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
    /// Active browser-search editor visual signature.
    pub browser_search_editor_signature: u64,
    /// Active browser pill-editor visual signature.
    pub browser_search_sidebar_signature: u64,
    /// Active inline folder-create editor visual signature.
    pub folder_create_editor_signature: u64,
    /// Whether focused selection emphasis is active.
    pub has_focus_emphasis: bool,
}

/// Compact hover-overlay fingerprint for row hovers, editor chrome, and tooltips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HoverOverlayFingerprint {
    /// Current hovered shell node kind.
    pub hovered: Option<ShellNodeKind>,
    /// Hovered browser row in visible-row space.
    pub hovered_browser_visible_row: Option<usize>,
    /// Hovered folder pane, when the pointer is over a folder pane.
    pub hovered_folder_pane: Option<crate::compat_app_contract::FolderPaneIdModel>,
    /// Hovered folder row by rendered sidebar row index.
    pub hovered_folder_row_index: Option<usize>,
    /// Hovered waveform-toolbar hint target.
    pub hovered_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
    /// Active browser-search editor visual signature.
    pub browser_search_editor_signature: u64,
    /// Active inline folder-create editor visual signature.
    pub folder_create_editor_signature: u64,
}

/// Compact focus-overlay fingerprint for selection and focus emphasis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FocusOverlayFingerprint {
    /// Whether focused selection emphasis is active.
    pub has_focus_emphasis: bool,
}

/// Compact modal-overlay fingerprint for popovers and dialogs above focus chrome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ModalOverlayFingerprint {
    /// Active source-context-menu pane, if any.
    pub source_context_menu_pane: Option<crate::compat_app_contract::FolderPaneIdModel>,
    /// Active source-context-menu row, if any.
    pub source_context_menu_row_index: Option<usize>,
    /// Source-context-menu anchor x-position bits, if any.
    pub source_context_menu_anchor_x_bits: Option<u32>,
    /// Source-context-menu anchor y-position bits, if any.
    pub source_context_menu_anchor_y_bits: Option<u32>,
    /// Active browser-context-menu row, if any.
    pub browser_context_menu_row_index: Option<usize>,
    /// Browser-context-menu anchor x-position bits, if any.
    pub browser_context_menu_anchor_x_bits: Option<u32>,
    /// Browser-context-menu anchor y-position bits, if any.
    pub browser_context_menu_anchor_y_bits: Option<u32>,
}

/// Compact motion-overlay fingerprint for runtime overlay skip checks.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MotionOverlayFingerprint {
    /// Whether transport-running animation is active.
    pub transport_running: bool,
    /// Remaining startup animation ticks.
    pub startup_frame_ticks: u8,
    /// Quantized pulse animation phase.
    pub pulse_phase_bits: u32,
    /// Hovered waveform marker x-position bits in shell-space coordinates.
    pub waveform_hover_x_bits: Option<u32>,
    /// Hovered waveform resize-edge target for highlight overlays.
    pub hovered_waveform_resize_edge: Option<WaveformResizeHoverEdge>,
}

/// Compact waveform-motion fingerprint for cursor/playhead overlay caches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaveformMotionOverlayFingerprint {
    /// Hovered waveform marker x-position bits in shell-space coordinates.
    pub waveform_hover_x_bits: Option<u32>,
    /// Hovered waveform resize-edge target for highlight overlays.
    pub hovered_waveform_resize_edge: Option<WaveformResizeHoverEdge>,
    /// Whether the waveform selection success flash is active.
    pub waveform_selection_flash_active: bool,
    /// Whether the waveform edit-selection apply flash is active.
    pub waveform_edit_selection_flash_active: bool,
    /// Current flash tone for waveform selection export feedback.
    pub waveform_selection_flash_tone: WaveformSelectionFlashTone,
    /// Quantized motion phase to force repaint while dynamic trails fade.
    pub pulse_phase_bits: u32,
}

/// Compact chrome-motion fingerprint for toolbar/tabs/status overlay caches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChromeMotionOverlayFingerprint {
    /// Whether transport-running animation is active.
    pub transport_running: bool,
    /// Remaining startup animation ticks.
    pub startup_frame_ticks: u8,
    /// Hovered browser rating-filter chip level, if any.
    pub hovered_browser_rating_filter_level: Option<i8>,
    /// Hovered browser playback-age filter chip, if any.
    pub hovered_browser_playback_age_filter_chip:
        Option<crate::compat_app_contract::PlaybackAgeFilterChip>,
    /// Whether the browser marked-filter chip is hovered.
    pub hovered_browser_marked_filter: bool,
    /// Whether the browser search field is hovered.
    pub hovered_browser_search_field: bool,
    /// Whether the source-add button is hovered.
    pub hovered_source_add_button: bool,
    /// Whether the status-bar options button is hovered.
    pub hovered_status_options_button: bool,
    /// Whether the status options button is currently in an error state.
    pub status_options_button_error: bool,
    /// Hovered waveform-toolbar icon/button target.
    pub hovered_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
    /// Whether the source-add button is currently click-flashed.
    pub flashed_source_add_button: bool,
    /// Remaining flash ticks for source-add-button click feedback.
    pub source_add_button_flash_ticks: u8,
    /// Whether the status-bar options button is currently click-flashed.
    pub flashed_status_options_button: bool,
    /// Remaining flash ticks for status options button click feedback.
    pub status_options_button_flash_ticks: u8,
    /// Click-flashed waveform-toolbar icon/button target.
    pub flashed_waveform_toolbar_hint: Option<WaveformToolbarHoverHint>,
    /// Remaining flash ticks for waveform-toolbar click feedback.
    pub waveform_toolbar_flash_ticks: u8,
    /// Active waveform-BPM editor visual signature.
    pub waveform_bpm_editor_signature: u64,
    /// Quantized pulse animation phase.
    pub pulse_phase_bits: u32,
}
