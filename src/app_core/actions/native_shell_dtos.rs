//! Sempal-owned native shell projection DTOs.
//!
//! These models describe Sempal application state as projected for the current
//! native shell. Radiant still consumes a compatibility copy at the runtime
//! boundary, so this module also provides field-for-field adapters that preserve
//! the legacy shell snapshot contract without making Radiant the owner of the
//! Sempal projection types.

use radiant::compat::sempal_shell as compat;
use radiant::gui::types::ImageRgba;
use std::sync::Arc;

use super::{
    NativeBrowserActionsModel, NativeBrowserChromeModel, NativeBrowserPanelModel,
    NativeColumnModel, NativeFocusContextModel, NativeMapPanelModel, NativeSourcesPanelModel,
};

/// Structured footer status content for left/center/right status segments.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StatusBarModel {
    /// Left-aligned status segment.
    pub left: String,
    /// Center-aligned status segment.
    pub center: String,
    /// Right-aligned status segment.
    pub right: String,
}

/// Health state of the compact audio-engine status chip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AudioEngineChipStateModel {
    /// Output engine is active and matches the requested configuration.
    #[default]
    Healthy,
    /// Output engine is unavailable or degraded.
    Error,
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

/// One selectable item shown inside an audio picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioOptionItemModel {
    /// Human-readable option label.
    pub label: String,
    /// Whether the option is currently selected.
    pub selected: bool,
    /// Raw value applied when the option is chosen.
    pub value: AudioOptionValueModel,
}

/// Overview row shown for one audio field inside the options panel.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AudioFieldModel {
    /// Static row label.
    pub label: String,
    /// Current value summary.
    pub value_label: String,
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

/// Update-check status projected into the native shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UpdateStatusModel {
    /// No update activity in progress.
    #[default]
    Idle,
    /// Update check is running.
    Checking,
    /// A newer update is available.
    Available,
    /// Update check failed.
    Error,
}

/// Update panel state used by native top-bar actions.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct UpdatePanelModel {
    /// Current update-check status.
    pub status: UpdateStatusModel,
    /// Status label rendered in native top-bar chrome.
    pub status_label: String,
    /// Action hint label rendered near update controls.
    pub action_hint_label: String,
    /// Supplemental release-notes label rendered under update hints.
    pub release_notes_label: String,
    /// Available release tag, when present.
    pub available_tag: Option<String>,
    /// Available release URL, when present.
    pub available_url: Option<String>,
    /// Last error message from update checks, if any.
    pub last_error: Option<String>,
}

/// Progress overlay state projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProgressOverlayModel {
    /// Whether the overlay is currently visible.
    pub visible: bool,
    /// Whether the overlay is modal.
    pub modal: bool,
    /// Title text for the progress surface.
    pub title: String,
    /// Optional detail line.
    pub detail: Option<String>,
    /// Completed steps.
    pub completed: usize,
    /// Total steps.
    pub total: usize,
    /// Whether the running operation supports cancel.
    pub cancelable: bool,
    /// Whether cancel has already been requested.
    pub cancel_requested: bool,
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

/// Modal confirmation prompt projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ConfirmPromptModel {
    /// Whether the prompt is currently visible.
    pub visible: bool,
    /// Prompt kind used by the bridge to resolve confirm/cancel behavior.
    pub kind: Option<ConfirmPromptKind>,
    /// Prompt title text.
    pub title: String,
    /// Prompt body text.
    pub message: String,
    /// Confirm action label.
    pub confirm_label: String,
    /// Cancel action label.
    pub cancel_label: String,
    /// Optional target label shown as supplemental metadata.
    pub target_label: Option<String>,
    /// Optional editable prompt input value.
    pub input_value: Option<String>,
    /// Placeholder text for editable prompt input fields.
    pub input_placeholder: Option<String>,
    /// Optional validation error shown below editable prompt input.
    pub input_error: Option<String>,
}

/// Drag/drop overlay content for native-shell feedback.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DragOverlayModel {
    /// Whether a drag payload is currently active.
    pub active: bool,
    /// Human-friendly payload label.
    pub label: String,
    /// Current hover target label.
    pub target_label: String,
    /// Whether the current target is a valid drop.
    pub valid_target: bool,
    /// Cursor anchor x-coordinate for the floating drag chip, when available.
    pub pointer_x: Option<u16>,
    /// Cursor anchor y-coordinate for the floating drag chip, when available.
    pub pointer_y: Option<u16>,
}

/// Normalized waveform range with deterministic milli, micro, and nano projections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalizedRangeModel {
    /// Start position in normalized milli-units.
    pub start_milli: u16,
    /// End position in normalized milli-units.
    pub end_milli: u16,
    /// Start position in normalized micro-units (`0..=1_000_000`).
    pub start_micros: u32,
    /// End position in normalized micro-units (`0..=1_000_000`).
    pub end_micros: u32,
    /// Start position in normalized nanounits (`0..=1_000_000_000`).
    pub start_nanos: u32,
    /// End position in normalized nanounits (`0..=1_000_000_000`).
    pub end_nanos: u32,
}

impl NormalizedRangeModel {
    /// Build a normalized range, clamping bounds to `0..=1000` and ordering them.
    pub fn new(start_milli: u16, end_milli: u16) -> Self {
        Self::from_micros(
            u32::from(start_milli.min(1000)) * 1000,
            u32::from(end_milli.min(1000)) * 1000,
        )
    }

    /// Build a normalized range from micro precision while preserving ordered milli mirrors.
    pub fn from_micros(start_micros: u32, end_micros: u32) -> Self {
        Self::from_nanos(
            start_micros.min(1_000_000).saturating_mul(1000),
            end_micros.min(1_000_000).saturating_mul(1000),
        )
    }

    /// Build a normalized range from nano precision while preserving ordered mirrors.
    pub fn from_nanos(start_nanos: u32, end_nanos: u32) -> Self {
        let start = start_nanos.min(1_000_000_000);
        let end = end_nanos.min(1_000_000_000);
        let ordered_start = start.min(end);
        let ordered_end = end.max(start);
        Self {
            start_milli: nanos_to_milli(ordered_start),
            end_milli: nanos_to_milli(ordered_end),
            start_micros: nanos_to_micros(ordered_start),
            end_micros: nanos_to_micros(ordered_end),
            start_nanos: ordered_start,
            end_nanos: ordered_end,
        }
    }
}

/// One detected waveform slice preview exposed to the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformSlicePreviewModel {
    /// Detected slice range in normalized milli, micro, and nano precision.
    pub range: NormalizedRangeModel,
    /// Whether this slice is currently selected for slice-edit operations.
    pub selected: bool,
    /// Whether this slice is focused for keyboard review audition.
    pub focused: bool,
    /// Whether this slice is marked for export.
    pub marked_for_export: bool,
    /// Whether this slice belongs to a duplicate-cleanup preview batch.
    pub duplicate_cleanup_candidate: bool,
    /// Whether this duplicate preview is currently exempted from cleanup.
    pub duplicate_cleanup_exempted: bool,
}

/// Waveform preview metadata consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformPanelModel {
    /// Display label for the loaded sample, when any.
    pub loaded_label: Option<String>,
    /// Whether a newly focused sample is still loading waveform data.
    pub loading: bool,
    /// Whether a replacement waveform image is still rendering in the background.
    pub image_rendering: bool,
    /// Cursor position in normalized milli-units.
    pub cursor_milli: Option<u16>,
    /// Playhead position in normalized milli-units.
    pub playhead_milli: Option<u16>,
    /// Playhead position in normalized micro-units (`0..=1_000_000`).
    pub playhead_micros: Option<u32>,
    /// Current waveform selection bounds.
    pub selection_milli: Option<NormalizedRangeModel>,
    /// Preview slices detected from silence-splitting the loaded waveform.
    pub slices: Vec<WaveformSlicePreviewModel>,
    /// One-shot token incremented when a waveform-selection export is queued.
    pub selection_export_flash_nonce: u64,
    /// One-shot token incremented when a queued waveform-selection export fails.
    pub selection_export_failure_flash_nonce: u64,
    /// One-shot token incremented when preview edit fades are committed.
    pub edit_selection_apply_flash_nonce: u64,
    /// Current waveform edit-selection bounds.
    pub edit_selection_milli: Option<NormalizedRangeModel>,
    /// End position for the edit fade-in region in normalized milli-units.
    pub edit_fade_in_end_milli: Option<u16>,
    /// End position for the edit fade-in region in normalized micro-units.
    pub edit_fade_in_end_micros: Option<u32>,
    /// Start position for the edit fade-in mute region in normalized milli-units.
    pub edit_fade_in_mute_start_milli: Option<u16>,
    /// Start position for the edit fade-in mute region in normalized micro-units.
    pub edit_fade_in_mute_start_micros: Option<u32>,
    /// Fade-in curve tension in normalized milli-units (`0..=1000`).
    pub edit_fade_in_curve_milli: Option<u16>,
    /// Start position for the edit fade-out region in normalized milli-units.
    pub edit_fade_out_start_milli: Option<u16>,
    /// Start position for the edit fade-out region in normalized micro-units.
    pub edit_fade_out_start_micros: Option<u32>,
    /// End position for the edit fade-out mute region in normalized milli-units.
    pub edit_fade_out_mute_end_milli: Option<u16>,
    /// End position for the edit fade-out mute region in normalized micro-units.
    pub edit_fade_out_mute_end_micros: Option<u32>,
    /// Fade-out curve tension in normalized milli-units (`0..=1000`).
    pub edit_fade_out_curve_milli: Option<u16>,
    /// Visible view start in normalized milli-units.
    pub view_start_milli: u16,
    /// Visible view end in normalized milli-units.
    pub view_end_milli: u16,
    /// Visible view start in normalized micro-units (`0..=1_000_000`).
    pub view_start_micros: u32,
    /// Visible view end in normalized micro-units (`0..=1_000_000`).
    pub view_end_micros: u32,
    /// Visible view start in normalized nanounits (`0..=1_000_000_000`).
    pub view_start_nanos: u32,
    /// Visible view end in normalized nanounits (`0..=1_000_000_000`).
    pub view_end_nanos: u32,
    /// Quarter-note beat spacing in normalized micro-units when BPM/grid data is available.
    pub beat_step_micros: Option<u32>,
    /// BPM grid origin in normalized micro-units.
    pub bpm_grid_origin_micros: u32,
    /// Whether loop playback is enabled.
    pub loop_enabled: bool,
    /// Optional tempo label rendered in waveform metadata.
    pub tempo_label: Option<String>,
    /// Optional zoom label rendered in waveform metadata.
    pub zoom_label: Option<String>,
    /// Cached signature for waveform image updates.
    pub waveform_image_signature: Option<u64>,
    /// Optional rasterized waveform payload for rendering the waveform preview.
    pub waveform_image: Option<Arc<ImageRgba>>,
}

impl Default for WaveformPanelModel {
    fn default() -> Self {
        Self {
            loaded_label: None,
            loading: false,
            image_rendering: false,
            cursor_milli: None,
            playhead_milli: None,
            playhead_micros: None,
            selection_milli: None,
            slices: Vec::new(),
            selection_export_flash_nonce: 0,
            selection_export_failure_flash_nonce: 0,
            edit_selection_apply_flash_nonce: 0,
            edit_selection_milli: None,
            edit_fade_in_end_milli: None,
            edit_fade_in_end_micros: None,
            edit_fade_in_mute_start_milli: None,
            edit_fade_in_mute_start_micros: None,
            edit_fade_in_curve_milli: None,
            edit_fade_out_start_milli: None,
            edit_fade_out_start_micros: None,
            edit_fade_out_mute_end_milli: None,
            edit_fade_out_mute_end_micros: None,
            edit_fade_out_curve_milli: None,
            view_start_milli: 0,
            view_end_milli: 1000,
            view_start_micros: 0,
            view_end_micros: 1_000_000,
            view_start_nanos: 0,
            view_end_nanos: 1_000_000_000,
            beat_step_micros: None,
            bpm_grid_origin_micros: 0,
            loop_enabled: false,
            tempo_label: None,
            zoom_label: None,
            waveform_image_signature: None,
            waveform_image: None,
        }
    }
}

/// Waveform channel-view mode used by waveform rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveformChannelViewModel {
    /// Collapse channels into one mono envelope.
    Mono,
    /// Render left/right channels in split stereo mode.
    Stereo,
}

/// Waveform chrome copy used by metadata lines and control surfaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformChromeModel {
    /// Extra transport metadata hint shown alongside waveform labels.
    pub transport_hint: String,
    /// Whether compare-anchor replay is currently available.
    pub compare_anchor_available: bool,
    /// Label for the stored compare anchor, when available.
    pub compare_anchor_label: Option<String>,
    /// Whether loop state is locked against sample-driven auto-updates.
    pub loop_lock_enabled: bool,
    /// Current channel-view mode used by waveform rendering.
    pub channel_view: WaveformChannelViewModel,
    /// Whether normalized audition playback is enabled.
    pub normalized_audition_enabled: bool,
    /// Whether BPM snapping is enabled for waveform edits.
    pub bpm_snap_enabled: bool,
    /// Whether playback BPM grids and snapping use selection-relative anchors.
    pub relative_bpm_grid_enabled: bool,
    /// Whether transient snapping is enabled for waveform edits.
    pub transient_snap_enabled: bool,
    /// Whether transient markers are visible on the waveform.
    pub transient_markers_enabled: bool,
    /// Whether slice mode is currently active.
    pub slice_mode_enabled: bool,
    /// Whether the current slice batch is an exact-duplicate cleanup preview.
    pub exact_duplicate_cleanup_available: bool,
}

impl Default for WaveformChromeModel {
    fn default() -> Self {
        Self {
            transport_hint: String::from("transport idle"),
            compare_anchor_available: false,
            compare_anchor_label: None,
            loop_lock_enabled: false,
            channel_view: WaveformChannelViewModel::Mono,
            normalized_audition_enabled: false,
            bpm_snap_enabled: false,
            relative_bpm_grid_enabled: false,
            transient_snap_enabled: false,
            transient_markers_enabled: true,
            slice_mode_enabled: false,
            exact_duplicate_cleanup_available: false,
        }
    }
}

/// Extract the numeric BPM portion from one projected tempo label.
pub fn parse_waveform_tempo_number_text(label: &str) -> Option<String> {
    let number = label.split_ascii_whitespace().next()?.trim();
    if number.is_empty() {
        return None;
    }
    let parsed = number.parse::<f32>().ok()?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return None;
    }
    Some(number.to_string())
}

fn micros_to_milli(value_micros: u32) -> u16 {
    ((value_micros.min(1_000_000) + 500) / 1000) as u16
}

fn nanos_to_micros(value_nanos: u32) -> u32 {
    ((value_nanos.min(1_000_000_000) + 500) / 1000).min(1_000_000)
}

fn nanos_to_milli(value_nanos: u32) -> u16 {
    micros_to_milli(nanos_to_micros(value_nanos))
}

/// Snapshot of Sempal state required by the native shell renderer.
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
    pub browser_actions: NativeBrowserActionsModel,
    /// Options-panel overlay projection.
    pub options_panel: OptionsPanelModel,
    /// Progress overlay projection.
    pub progress_overlay: ProgressOverlayModel,
    /// Modal confirm prompt projection.
    pub confirm_prompt: ConfirmPromptModel,
    /// Drag/drop overlay projection.
    pub drag_overlay: DragOverlayModel,
    /// Logical triage/browser columns.
    pub columns: [NativeColumnModel; 3],
    /// Selected column index (0..=2).
    pub selected_column: usize,
    /// Master output volume normalized to `0.0..=1.0`.
    pub volume: f32,
    /// Whether transport/animation should be considered running.
    pub transport_running: bool,
    /// Source panel model consumed by the native renderer.
    pub sources: NativeSourcesPanelModel,
    /// Browser panel summary consumed by the native renderer.
    pub browser: NativeBrowserPanelModel,
    /// Browser chrome labels consumed by native tabs/toolbar/footer text.
    pub browser_chrome: NativeBrowserChromeModel,
    /// Map panel summary consumed by the native renderer.
    pub map: NativeMapPanelModel,
    /// Waveform panel summary consumed by the native renderer.
    pub waveform: WaveformPanelModel,
    /// Waveform chrome labels consumed by the native waveform header.
    pub waveform_chrome: WaveformChromeModel,
    /// Update surface summary consumed by the native top bar.
    pub update: UpdatePanelModel,
    /// Current keyboard focus bucket used for contextual native key routing.
    pub focus_context: NativeFocusContextModel,
}

impl Default for AppModel {
    fn default() -> Self {
        Self::from(compat::AppModel::default())
    }
}

impl From<compat::StatusBarModel> for StatusBarModel {
    fn from(value: compat::StatusBarModel) -> Self {
        Self {
            left: value.left,
            center: value.center,
            right: value.right,
        }
    }
}

impl From<StatusBarModel> for compat::StatusBarModel {
    fn from(value: StatusBarModel) -> Self {
        Self {
            left: value.left,
            center: value.center,
            right: value.right,
        }
    }
}

impl From<&StatusBarModel> for compat::StatusBarModel {
    fn from(value: &StatusBarModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::AudioEngineChipStateModel> for AudioEngineChipStateModel {
    fn from(value: compat::AudioEngineChipStateModel) -> Self {
        match value {
            compat::AudioEngineChipStateModel::Healthy => Self::Healthy,
            compat::AudioEngineChipStateModel::Error => Self::Error,
        }
    }
}

impl From<AudioEngineChipStateModel> for compat::AudioEngineChipStateModel {
    fn from(value: AudioEngineChipStateModel) -> Self {
        match value {
            AudioEngineChipStateModel::Healthy => Self::Healthy,
            AudioEngineChipStateModel::Error => Self::Error,
        }
    }
}

impl From<compat::AudioPickerTargetModel> for AudioPickerTargetModel {
    fn from(value: compat::AudioPickerTargetModel) -> Self {
        match value {
            compat::AudioPickerTargetModel::OutputHost => Self::OutputHost,
            compat::AudioPickerTargetModel::OutputDevice => Self::OutputDevice,
            compat::AudioPickerTargetModel::OutputSampleRate => Self::OutputSampleRate,
            compat::AudioPickerTargetModel::InputHost => Self::InputHost,
            compat::AudioPickerTargetModel::InputDevice => Self::InputDevice,
            compat::AudioPickerTargetModel::InputSampleRate => Self::InputSampleRate,
        }
    }
}

impl From<AudioPickerTargetModel> for compat::AudioPickerTargetModel {
    fn from(value: AudioPickerTargetModel) -> Self {
        match value {
            AudioPickerTargetModel::OutputHost => Self::OutputHost,
            AudioPickerTargetModel::OutputDevice => Self::OutputDevice,
            AudioPickerTargetModel::OutputSampleRate => Self::OutputSampleRate,
            AudioPickerTargetModel::InputHost => Self::InputHost,
            AudioPickerTargetModel::InputDevice => Self::InputDevice,
            AudioPickerTargetModel::InputSampleRate => Self::InputSampleRate,
        }
    }
}

impl From<compat::AudioOptionValueModel> for AudioOptionValueModel {
    fn from(value: compat::AudioOptionValueModel) -> Self {
        match value {
            compat::AudioOptionValueModel::OutputHost(value) => Self::OutputHost(value),
            compat::AudioOptionValueModel::OutputDevice(value) => Self::OutputDevice(value),
            compat::AudioOptionValueModel::OutputSampleRate(value) => Self::OutputSampleRate(value),
            compat::AudioOptionValueModel::InputHost(value) => Self::InputHost(value),
            compat::AudioOptionValueModel::InputDevice(value) => Self::InputDevice(value),
            compat::AudioOptionValueModel::InputSampleRate(value) => Self::InputSampleRate(value),
        }
    }
}

impl From<AudioOptionValueModel> for compat::AudioOptionValueModel {
    fn from(value: AudioOptionValueModel) -> Self {
        match value {
            AudioOptionValueModel::OutputHost(value) => Self::OutputHost(value),
            AudioOptionValueModel::OutputDevice(value) => Self::OutputDevice(value),
            AudioOptionValueModel::OutputSampleRate(value) => Self::OutputSampleRate(value),
            AudioOptionValueModel::InputHost(value) => Self::InputHost(value),
            AudioOptionValueModel::InputDevice(value) => Self::InputDevice(value),
            AudioOptionValueModel::InputSampleRate(value) => Self::InputSampleRate(value),
        }
    }
}

impl From<compat::AudioOptionItemModel> for AudioOptionItemModel {
    fn from(value: compat::AudioOptionItemModel) -> Self {
        Self {
            label: value.label,
            selected: value.selected,
            value: value.value.into(),
        }
    }
}

impl From<AudioOptionItemModel> for compat::AudioOptionItemModel {
    fn from(value: AudioOptionItemModel) -> Self {
        Self {
            label: value.label,
            selected: value.selected,
            value: value.value.into(),
        }
    }
}

impl From<compat::AudioFieldModel> for AudioFieldModel {
    fn from(value: compat::AudioFieldModel) -> Self {
        Self {
            label: value.label,
            value_label: value.value_label,
        }
    }
}

impl From<AudioFieldModel> for compat::AudioFieldModel {
    fn from(value: AudioFieldModel) -> Self {
        Self {
            label: value.label,
            value_label: value.value_label,
        }
    }
}

impl From<compat::AudioEngineModel> for AudioEngineModel {
    fn from(value: compat::AudioEngineModel) -> Self {
        Self {
            chip_state: value.chip_state.into(),
            chip_label: value.chip_label,
            detail_label: value.detail_label,
            output_host: value.output_host.into(),
            output_device: value.output_device.into(),
            output_sample_rate: value.output_sample_rate.into(),
            input_host: value.input_host.into(),
            input_device: value.input_device.into(),
            input_sample_rate: value.input_sample_rate.into(),
            active_picker: value.active_picker.map(Into::into),
            output_host_options: value
                .output_host_options
                .into_iter()
                .map(Into::into)
                .collect(),
            output_device_options: value
                .output_device_options
                .into_iter()
                .map(Into::into)
                .collect(),
            output_sample_rate_options: value
                .output_sample_rate_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_host_options: value
                .input_host_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_device_options: value
                .input_device_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_sample_rate_options: value
                .input_sample_rate_options
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<AudioEngineModel> for compat::AudioEngineModel {
    fn from(value: AudioEngineModel) -> Self {
        Self {
            chip_state: value.chip_state.into(),
            chip_label: value.chip_label,
            detail_label: value.detail_label,
            output_host: value.output_host.into(),
            output_device: value.output_device.into(),
            output_sample_rate: value.output_sample_rate.into(),
            input_host: value.input_host.into(),
            input_device: value.input_device.into(),
            input_sample_rate: value.input_sample_rate.into(),
            active_picker: value.active_picker.map(Into::into),
            output_host_options: value
                .output_host_options
                .into_iter()
                .map(Into::into)
                .collect(),
            output_device_options: value
                .output_device_options
                .into_iter()
                .map(Into::into)
                .collect(),
            output_sample_rate_options: value
                .output_sample_rate_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_host_options: value
                .input_host_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_device_options: value
                .input_device_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_sample_rate_options: value
                .input_sample_rate_options
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<&AudioEngineModel> for compat::AudioEngineModel {
    fn from(value: &AudioEngineModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::UpdateStatusModel> for UpdateStatusModel {
    fn from(value: compat::UpdateStatusModel) -> Self {
        match value {
            compat::UpdateStatusModel::Idle => Self::Idle,
            compat::UpdateStatusModel::Checking => Self::Checking,
            compat::UpdateStatusModel::Available => Self::Available,
            compat::UpdateStatusModel::Error => Self::Error,
        }
    }
}

impl From<UpdateStatusModel> for compat::UpdateStatusModel {
    fn from(value: UpdateStatusModel) -> Self {
        match value {
            UpdateStatusModel::Idle => Self::Idle,
            UpdateStatusModel::Checking => Self::Checking,
            UpdateStatusModel::Available => Self::Available,
            UpdateStatusModel::Error => Self::Error,
        }
    }
}

impl From<compat::UpdatePanelModel> for UpdatePanelModel {
    fn from(value: compat::UpdatePanelModel) -> Self {
        Self {
            status: value.status.into(),
            status_label: value.status_label,
            action_hint_label: value.action_hint_label,
            release_notes_label: value.release_notes_label,
            available_tag: value.available_tag,
            available_url: value.available_url,
            last_error: value.last_error,
        }
    }
}

impl From<UpdatePanelModel> for compat::UpdatePanelModel {
    fn from(value: UpdatePanelModel) -> Self {
        Self {
            status: value.status.into(),
            status_label: value.status_label,
            action_hint_label: value.action_hint_label,
            release_notes_label: value.release_notes_label,
            available_tag: value.available_tag,
            available_url: value.available_url,
            last_error: value.last_error,
        }
    }
}

impl From<&UpdatePanelModel> for compat::UpdatePanelModel {
    fn from(value: &UpdatePanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::ProgressOverlayModel> for ProgressOverlayModel {
    fn from(value: compat::ProgressOverlayModel) -> Self {
        Self {
            visible: value.visible,
            modal: value.modal,
            title: value.title,
            detail: value.detail,
            completed: value.completed,
            total: value.total,
            cancelable: value.cancelable,
            cancel_requested: value.cancel_requested,
        }
    }
}

impl From<ProgressOverlayModel> for compat::ProgressOverlayModel {
    fn from(value: ProgressOverlayModel) -> Self {
        Self {
            visible: value.visible,
            modal: value.modal,
            title: value.title,
            detail: value.detail,
            completed: value.completed,
            total: value.total,
            cancelable: value.cancelable,
            cancel_requested: value.cancel_requested,
        }
    }
}

impl From<&ProgressOverlayModel> for compat::ProgressOverlayModel {
    fn from(value: &ProgressOverlayModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::OptionsPanelModel> for OptionsPanelModel {
    fn from(value: compat::OptionsPanelModel) -> Self {
        Self {
            visible: value.visible,
            default_identifier: value.default_identifier,
            input_monitoring_enabled: value.input_monitoring_enabled,
            advance_after_rating_enabled: value.advance_after_rating_enabled,
            destructive_yolo_mode_enabled: value.destructive_yolo_mode_enabled,
            invert_waveform_scroll_enabled: value.invert_waveform_scroll_enabled,
            trash_folder_label: value.trash_folder_label,
        }
    }
}

impl From<OptionsPanelModel> for compat::OptionsPanelModel {
    fn from(value: OptionsPanelModel) -> Self {
        Self {
            visible: value.visible,
            default_identifier: value.default_identifier,
            input_monitoring_enabled: value.input_monitoring_enabled,
            advance_after_rating_enabled: value.advance_after_rating_enabled,
            destructive_yolo_mode_enabled: value.destructive_yolo_mode_enabled,
            invert_waveform_scroll_enabled: value.invert_waveform_scroll_enabled,
            trash_folder_label: value.trash_folder_label,
        }
    }
}

impl From<&OptionsPanelModel> for compat::OptionsPanelModel {
    fn from(value: &OptionsPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::ConfirmPromptKind> for ConfirmPromptKind {
    fn from(value: compat::ConfirmPromptKind) -> Self {
        match value {
            compat::ConfirmPromptKind::DestructiveEdit => Self::DestructiveEdit,
            compat::ConfirmPromptKind::BrowserRename => Self::BrowserRename,
            compat::ConfirmPromptKind::FolderRename => Self::FolderRename,
            compat::ConfirmPromptKind::FolderCreate => Self::FolderCreate,
            compat::ConfirmPromptKind::RestoreRetainedFolderDeletes => {
                Self::RestoreRetainedFolderDeletes
            }
            compat::ConfirmPromptKind::PurgeRetainedFolderDeletes => {
                Self::PurgeRetainedFolderDeletes
            }
            compat::ConfirmPromptKind::OptionsDefaultIdentifier => Self::OptionsDefaultIdentifier,
        }
    }
}

impl From<ConfirmPromptKind> for compat::ConfirmPromptKind {
    fn from(value: ConfirmPromptKind) -> Self {
        match value {
            ConfirmPromptKind::DestructiveEdit => Self::DestructiveEdit,
            ConfirmPromptKind::BrowserRename => Self::BrowserRename,
            ConfirmPromptKind::FolderRename => Self::FolderRename,
            ConfirmPromptKind::FolderCreate => Self::FolderCreate,
            ConfirmPromptKind::RestoreRetainedFolderDeletes => Self::RestoreRetainedFolderDeletes,
            ConfirmPromptKind::PurgeRetainedFolderDeletes => Self::PurgeRetainedFolderDeletes,
            ConfirmPromptKind::OptionsDefaultIdentifier => Self::OptionsDefaultIdentifier,
        }
    }
}

impl From<compat::ConfirmPromptModel> for ConfirmPromptModel {
    fn from(value: compat::ConfirmPromptModel) -> Self {
        Self {
            visible: value.visible,
            kind: value.kind.map(Into::into),
            title: value.title,
            message: value.message,
            confirm_label: value.confirm_label,
            cancel_label: value.cancel_label,
            target_label: value.target_label,
            input_value: value.input_value,
            input_placeholder: value.input_placeholder,
            input_error: value.input_error,
        }
    }
}

impl From<ConfirmPromptModel> for compat::ConfirmPromptModel {
    fn from(value: ConfirmPromptModel) -> Self {
        Self {
            visible: value.visible,
            kind: value.kind.map(Into::into),
            title: value.title,
            message: value.message,
            confirm_label: value.confirm_label,
            cancel_label: value.cancel_label,
            target_label: value.target_label,
            input_value: value.input_value,
            input_placeholder: value.input_placeholder,
            input_error: value.input_error,
        }
    }
}

impl From<&ConfirmPromptModel> for compat::ConfirmPromptModel {
    fn from(value: &ConfirmPromptModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::DragOverlayModel> for DragOverlayModel {
    fn from(value: compat::DragOverlayModel) -> Self {
        Self {
            active: value.active,
            label: value.label,
            target_label: value.target_label,
            valid_target: value.valid_target,
            pointer_x: value.pointer_x,
            pointer_y: value.pointer_y,
        }
    }
}

impl From<DragOverlayModel> for compat::DragOverlayModel {
    fn from(value: DragOverlayModel) -> Self {
        Self {
            active: value.active,
            label: value.label,
            target_label: value.target_label,
            valid_target: value.valid_target,
            pointer_x: value.pointer_x,
            pointer_y: value.pointer_y,
        }
    }
}

impl From<&DragOverlayModel> for compat::DragOverlayModel {
    fn from(value: &DragOverlayModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::NormalizedRangeModel> for NormalizedRangeModel {
    fn from(value: compat::NormalizedRangeModel) -> Self {
        Self {
            start_milli: value.start_milli,
            end_milli: value.end_milli,
            start_micros: value.start_micros,
            end_micros: value.end_micros,
            start_nanos: value.start_nanos,
            end_nanos: value.end_nanos,
        }
    }
}

impl From<NormalizedRangeModel> for compat::NormalizedRangeModel {
    fn from(value: NormalizedRangeModel) -> Self {
        Self {
            start_milli: value.start_milli,
            end_milli: value.end_milli,
            start_micros: value.start_micros,
            end_micros: value.end_micros,
            start_nanos: value.start_nanos,
            end_nanos: value.end_nanos,
        }
    }
}

impl From<compat::WaveformSlicePreviewModel> for WaveformSlicePreviewModel {
    fn from(value: compat::WaveformSlicePreviewModel) -> Self {
        Self {
            range: value.range.into(),
            selected: value.selected,
            focused: value.focused,
            marked_for_export: value.marked_for_export,
            duplicate_cleanup_candidate: value.duplicate_cleanup_candidate,
            duplicate_cleanup_exempted: value.duplicate_cleanup_exempted,
        }
    }
}

impl From<WaveformSlicePreviewModel> for compat::WaveformSlicePreviewModel {
    fn from(value: WaveformSlicePreviewModel) -> Self {
        Self {
            range: value.range.into(),
            selected: value.selected,
            focused: value.focused,
            marked_for_export: value.marked_for_export,
            duplicate_cleanup_candidate: value.duplicate_cleanup_candidate,
            duplicate_cleanup_exempted: value.duplicate_cleanup_exempted,
        }
    }
}

impl From<compat::WaveformPanelModel> for WaveformPanelModel {
    fn from(value: compat::WaveformPanelModel) -> Self {
        Self {
            loaded_label: value.loaded_label,
            loading: value.loading,
            image_rendering: value.image_rendering,
            cursor_milli: value.cursor_milli,
            playhead_milli: value.playhead_milli,
            playhead_micros: value.playhead_micros,
            selection_milli: value.selection_milli.map(Into::into),
            slices: value.slices.into_iter().map(Into::into).collect(),
            selection_export_flash_nonce: value.selection_export_flash_nonce,
            selection_export_failure_flash_nonce: value.selection_export_failure_flash_nonce,
            edit_selection_apply_flash_nonce: value.edit_selection_apply_flash_nonce,
            edit_selection_milli: value.edit_selection_milli.map(Into::into),
            edit_fade_in_end_milli: value.edit_fade_in_end_milli,
            edit_fade_in_end_micros: value.edit_fade_in_end_micros,
            edit_fade_in_mute_start_milli: value.edit_fade_in_mute_start_milli,
            edit_fade_in_mute_start_micros: value.edit_fade_in_mute_start_micros,
            edit_fade_in_curve_milli: value.edit_fade_in_curve_milli,
            edit_fade_out_start_milli: value.edit_fade_out_start_milli,
            edit_fade_out_start_micros: value.edit_fade_out_start_micros,
            edit_fade_out_mute_end_milli: value.edit_fade_out_mute_end_milli,
            edit_fade_out_mute_end_micros: value.edit_fade_out_mute_end_micros,
            edit_fade_out_curve_milli: value.edit_fade_out_curve_milli,
            view_start_milli: value.view_start_milli,
            view_end_milli: value.view_end_milli,
            view_start_micros: value.view_start_micros,
            view_end_micros: value.view_end_micros,
            view_start_nanos: value.view_start_nanos,
            view_end_nanos: value.view_end_nanos,
            beat_step_micros: value.beat_step_micros,
            bpm_grid_origin_micros: value.bpm_grid_origin_micros,
            loop_enabled: value.loop_enabled,
            tempo_label: value.tempo_label,
            zoom_label: value.zoom_label,
            waveform_image_signature: value.waveform_image_signature,
            waveform_image: value.waveform_image,
        }
    }
}

impl From<WaveformPanelModel> for compat::WaveformPanelModel {
    fn from(value: WaveformPanelModel) -> Self {
        Self {
            loaded_label: value.loaded_label,
            loading: value.loading,
            image_rendering: value.image_rendering,
            cursor_milli: value.cursor_milli,
            playhead_milli: value.playhead_milli,
            playhead_micros: value.playhead_micros,
            selection_milli: value.selection_milli.map(Into::into),
            slices: value.slices.into_iter().map(Into::into).collect(),
            selection_export_flash_nonce: value.selection_export_flash_nonce,
            selection_export_failure_flash_nonce: value.selection_export_failure_flash_nonce,
            edit_selection_apply_flash_nonce: value.edit_selection_apply_flash_nonce,
            edit_selection_milli: value.edit_selection_milli.map(Into::into),
            edit_fade_in_end_milli: value.edit_fade_in_end_milli,
            edit_fade_in_end_micros: value.edit_fade_in_end_micros,
            edit_fade_in_mute_start_milli: value.edit_fade_in_mute_start_milli,
            edit_fade_in_mute_start_micros: value.edit_fade_in_mute_start_micros,
            edit_fade_in_curve_milli: value.edit_fade_in_curve_milli,
            edit_fade_out_start_milli: value.edit_fade_out_start_milli,
            edit_fade_out_start_micros: value.edit_fade_out_start_micros,
            edit_fade_out_mute_end_milli: value.edit_fade_out_mute_end_milli,
            edit_fade_out_mute_end_micros: value.edit_fade_out_mute_end_micros,
            edit_fade_out_curve_milli: value.edit_fade_out_curve_milli,
            view_start_milli: value.view_start_milli,
            view_end_milli: value.view_end_milli,
            view_start_micros: value.view_start_micros,
            view_end_micros: value.view_end_micros,
            view_start_nanos: value.view_start_nanos,
            view_end_nanos: value.view_end_nanos,
            beat_step_micros: value.beat_step_micros,
            bpm_grid_origin_micros: value.bpm_grid_origin_micros,
            loop_enabled: value.loop_enabled,
            tempo_label: value.tempo_label,
            zoom_label: value.zoom_label,
            waveform_image_signature: value.waveform_image_signature,
            waveform_image: value.waveform_image,
        }
    }
}

impl From<&WaveformPanelModel> for compat::WaveformPanelModel {
    fn from(value: &WaveformPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::WaveformChannelViewModel> for WaveformChannelViewModel {
    fn from(value: compat::WaveformChannelViewModel) -> Self {
        match value {
            compat::WaveformChannelViewModel::Mono => Self::Mono,
            compat::WaveformChannelViewModel::Stereo => Self::Stereo,
        }
    }
}

impl From<WaveformChannelViewModel> for compat::WaveformChannelViewModel {
    fn from(value: WaveformChannelViewModel) -> Self {
        match value {
            WaveformChannelViewModel::Mono => Self::Mono,
            WaveformChannelViewModel::Stereo => Self::Stereo,
        }
    }
}

impl From<compat::WaveformChromeModel> for WaveformChromeModel {
    fn from(value: compat::WaveformChromeModel) -> Self {
        Self {
            transport_hint: value.transport_hint,
            compare_anchor_available: value.compare_anchor_available,
            compare_anchor_label: value.compare_anchor_label,
            loop_lock_enabled: value.loop_lock_enabled,
            channel_view: value.channel_view.into(),
            normalized_audition_enabled: value.normalized_audition_enabled,
            bpm_snap_enabled: value.bpm_snap_enabled,
            relative_bpm_grid_enabled: value.relative_bpm_grid_enabled,
            transient_snap_enabled: value.transient_snap_enabled,
            transient_markers_enabled: value.transient_markers_enabled,
            slice_mode_enabled: value.slice_mode_enabled,
            exact_duplicate_cleanup_available: value.exact_duplicate_cleanup_available,
        }
    }
}

impl From<WaveformChromeModel> for compat::WaveformChromeModel {
    fn from(value: WaveformChromeModel) -> Self {
        Self {
            transport_hint: value.transport_hint,
            compare_anchor_available: value.compare_anchor_available,
            compare_anchor_label: value.compare_anchor_label,
            loop_lock_enabled: value.loop_lock_enabled,
            channel_view: value.channel_view.into(),
            normalized_audition_enabled: value.normalized_audition_enabled,
            bpm_snap_enabled: value.bpm_snap_enabled,
            relative_bpm_grid_enabled: value.relative_bpm_grid_enabled,
            transient_snap_enabled: value.transient_snap_enabled,
            transient_markers_enabled: value.transient_markers_enabled,
            slice_mode_enabled: value.slice_mode_enabled,
            exact_duplicate_cleanup_available: value.exact_duplicate_cleanup_available,
        }
    }
}

impl From<&WaveformChromeModel> for compat::WaveformChromeModel {
    fn from(value: &WaveformChromeModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::AppModel> for AppModel {
    fn from(value: compat::AppModel) -> Self {
        Self {
            title: value.title,
            backend_label: value.backend_label,
            sources_label: value.sources_label,
            status_text: value.status_text,
            status: value.status.into(),
            audio_engine: value.audio_engine.into(),
            browser_actions: value.browser_actions,
            options_panel: value.options_panel.into(),
            progress_overlay: value.progress_overlay.into(),
            confirm_prompt: value.confirm_prompt.into(),
            drag_overlay: value.drag_overlay.into(),
            columns: value.columns,
            selected_column: value.selected_column,
            volume: value.volume,
            transport_running: value.transport_running,
            sources: value.sources,
            browser: value.browser,
            browser_chrome: value.browser_chrome,
            map: value.map,
            waveform: value.waveform.into(),
            waveform_chrome: value.waveform_chrome.into(),
            update: value.update.into(),
            focus_context: value.focus_context,
        }
    }
}

impl From<AppModel> for compat::AppModel {
    fn from(value: AppModel) -> Self {
        Self {
            title: value.title,
            backend_label: value.backend_label,
            sources_label: value.sources_label,
            status_text: value.status_text,
            status: value.status.into(),
            audio_engine: value.audio_engine.into(),
            browser_actions: value.browser_actions,
            options_panel: value.options_panel.into(),
            progress_overlay: value.progress_overlay.into(),
            confirm_prompt: value.confirm_prompt.into(),
            drag_overlay: value.drag_overlay.into(),
            columns: value.columns,
            selected_column: value.selected_column,
            volume: value.volume,
            transport_running: value.transport_running,
            sources: value.sources,
            browser: value.browser,
            browser_chrome: value.browser_chrome,
            map: value.map,
            waveform: value.waveform.into(),
            waveform_chrome: value.waveform_chrome.into(),
            update: value.update.into(),
            focus_context: value.focus_context,
        }
    }
}

impl From<&AppModel> for compat::AppModel {
    fn from(value: &AppModel) -> Self {
        value.clone().into()
    }
}

#[cfg(test)]
mod tests {
    use super::{WaveformPanelModel, parse_waveform_tempo_number_text};

    #[test]
    fn waveform_panel_default_bpm_grid_origin_is_zero() {
        assert_eq!(WaveformPanelModel::default().bpm_grid_origin_micros, 0);
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
