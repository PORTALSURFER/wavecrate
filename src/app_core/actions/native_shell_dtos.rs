//! Sempal-owned native shell projection DTOs.
//!
//! These models describe Sempal application state as projected for the current
//! native shell. Radiant still consumes a compatibility copy at the runtime
//! boundary, so this module also provides field-for-field adapters that preserve
//! the legacy shell snapshot contract without making Radiant the owner of the
//! Sempal projection types.

use radiant::compat::sempal_shell as compat;

use super::{
    NativeBrowserActionsModel, NativeBrowserChromeModel, NativeBrowserPanelModel,
    NativeColumnModel, NativeFocusContextModel, NativeMapPanelModel, NativeSourcesPanelModel,
    NativeWaveformChromeModel, NativeWaveformPanelModel,
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
    pub waveform: NativeWaveformPanelModel,
    /// Waveform chrome labels consumed by the native waveform header.
    pub waveform_chrome: NativeWaveformChromeModel,
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
            waveform: value.waveform,
            waveform_chrome: value.waveform_chrome,
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
            waveform: value.waveform,
            waveform_chrome: value.waveform_chrome,
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
