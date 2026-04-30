//! Migration-facing aliases and catalog helpers for native runtime action/model types.
//!
//! These aliases centralize runtime-facing type dependencies in `app_core`,
//! so bridge/controller glue does not import concrete runtime types directly.
//! Sempal-owned projection DTOs live here while Radiant-specific compatibility
//! types remain isolated to the runtime boundary.

mod catalog;
mod native_shell_actions;
mod native_shell_dtos;

pub use self::catalog::{
    GUI_ACTION_CATALOG, GuiActionCatalogEntry, GuiActionKind, GuiCoverageLayer, GuiDispatchPolicy,
    GuiEffectClass, GuiHistoryPolicy, GuiSurface, action_catalog_entry, action_catalog_entry_by_id,
    action_kind, representative_action_for_kind,
};

/// Native runtime browser action metadata model.
pub type NativeBrowserActionsModel = self::native_shell_dtos::BrowserActionsModel;

/// Native runtime browser chrome model.
pub type NativeBrowserChromeModel = self::native_shell_dtos::BrowserChromeModel;

/// Native runtime browser panel model.
pub type NativeBrowserPanelModel = self::native_shell_dtos::BrowserPanelModel;

/// Native runtime browser tag-sidebar model.
pub type NativeBrowserTagSidebarModel = self::native_shell_dtos::BrowserTagSidebarModel;

/// Native runtime browser tag pill model.
pub type NativeBrowserTagPillModel = self::native_shell_dtos::BrowserTagPillModel;

/// Native runtime browser tag pill state.
pub type NativeBrowserTagState = self::native_shell_dtos::BrowserTagState;

/// Native runtime browser playback-age row bucket model.
pub type NativePlaybackAgeBucket = self::native_shell_dtos::PlaybackAgeBucket;

/// Native runtime browser playback-age toolbar action payload.
pub type NativePlaybackAgeFilterChip = self::native_shell_dtos::PlaybackAgeFilterChip;

/// Native runtime browser row model.
pub type NativeBrowserRowModel = self::native_shell_dtos::BrowserRowModel;

/// Native runtime browser row processing state.
pub type NativeBrowserRowProcessingState = self::native_shell_dtos::BrowserRowProcessingState;

/// Native runtime retained vector used by Sempal-owned projection DTOs.
pub type NativeRetainedVec<T> = self::native_shell_dtos::RetainedVec<T>;

/// Native runtime automation node identifier.
pub type NativeAutomationNodeId = self::native_shell_dtos::AutomationNodeId;

/// Native runtime automation node role.
pub type NativeAutomationRole = self::native_shell_dtos::AutomationRole;

/// Native runtime automation node bounds.
pub type NativeAutomationBounds = self::native_shell_dtos::AutomationBounds;

/// Native runtime automation node snapshot.
pub type NativeAutomationNodeSnapshot = self::native_shell_dtos::AutomationNodeSnapshot;

/// Native runtime GUI automation snapshot.
pub type NativeGuiAutomationSnapshot = self::native_shell_dtos::GuiAutomationSnapshot;

/// Native runtime browser tag target used by keyboard and pointer triage actions.
pub type NativeBrowserTagTarget = self::native_shell_actions::BrowserTagTarget;

/// Native runtime UI action payload.
pub type NativeUiAction = self::native_shell_actions::UiAction;

/// Native runtime projected app model.
pub type NativeAppModel = self::native_shell_dtos::AppModel;

/// Native runtime audio-engine chip state.
pub type NativeAudioEngineChipStateModel = self::native_shell_dtos::AudioEngineChipStateModel;

/// Native runtime audio-engine projection model.
pub type NativeAudioEngineModel = self::native_shell_dtos::AudioEngineModel;

/// Native runtime audio-field summary model.
pub type NativeAudioFieldModel = self::native_shell_dtos::AudioFieldModel;

/// Native runtime audio picker option item.
pub type NativeAudioOptionItemModel = self::native_shell_dtos::AudioOptionItemModel;

/// Native runtime audio picker option raw value.
pub type NativeAudioOptionValueModel = self::native_shell_dtos::AudioOptionValueModel;

/// Native runtime audio picker target.
pub type NativeAudioPickerTargetModel = self::native_shell_dtos::AudioPickerTargetModel;

/// Native runtime focus-context model used for contextual input routing.
pub type NativeFocusContextModel = self::native_shell_dtos::FocusContextModel;

/// Native runtime frame build result payload.
pub type NativeFrameBuildResult = self::native_shell_dtos::FrameBuildResult;

/// Native runtime projection-dirty segment mask.
pub type NativeDirtySegments = self::native_shell_dtos::DirtySegments;

/// Native runtime static-segment revision snapshot.
pub type NativeSegmentRevisions = self::native_shell_dtos::SegmentRevisions;

/// Native runtime motion-only model payload.
pub type NativeMotionModel = self::native_shell_dtos::NativeMotionModel;

/// Native runtime table column summary model.
pub type NativeColumnModel = self::native_shell_dtos::ColumnModel;

/// Native runtime confirm prompt kind descriptor.
pub type NativeConfirmPromptKind = self::native_shell_dtos::ConfirmPromptKind;

/// Native runtime confirm prompt model.
pub type NativeConfirmPromptModel = self::native_shell_dtos::ConfirmPromptModel;

/// Native runtime drag overlay model.
pub type NativeDragOverlayModel = self::native_shell_dtos::DragOverlayModel;

/// Native runtime folder actions model.
pub type NativeFolderActionsModel = self::native_shell_dtos::FolderActionsModel;

/// Native runtime folder pane identifier.
pub type NativeFolderPaneIdModel = self::native_shell_dtos::FolderPaneIdModel;

/// Native runtime folder pane model.
pub type NativeFolderPaneModel = self::native_shell_dtos::FolderPaneModel;

/// Native runtime folder recovery model.
pub type NativeFolderRecoveryModel = self::native_shell_dtos::FolderRecoveryModel;

/// Native runtime folder row model.
pub type NativeFolderRowModel = self::native_shell_dtos::FolderRowModel;

/// Native runtime folder row kind.
pub type NativeFolderRowKind = self::native_shell_dtos::FolderRowKind;

/// Native runtime map panel model.
pub type NativeMapPanelModel = self::native_shell_dtos::MapPanelModel;

/// Native runtime map point model.
pub type NativeMapPointModel = self::native_shell_dtos::MapPointModel;

/// Native runtime map render mode model.
pub type NativeMapRenderModeModel = self::native_shell_dtos::MapRenderModeModel;

/// Native runtime options panel model.
pub type NativeOptionsPanelModel = self::native_shell_dtos::OptionsPanelModel;

/// Native runtime normalized range model.
pub type NativeNormalizedRangeModel = self::native_shell_dtos::NormalizedRangeModel;

/// Native runtime progress overlay model.
pub type NativeProgressOverlayModel = self::native_shell_dtos::ProgressOverlayModel;

/// Native runtime source row model.
pub type NativeSourceRowModel = self::native_shell_dtos::SourceRowModel;

/// Native runtime sources panel model.
pub type NativeSourcesPanelModel = self::native_shell_dtos::SourcesPanelModel;

/// Native runtime status bar model.
pub type NativeStatusBarModel = self::native_shell_dtos::StatusBarModel;

/// Native runtime update panel model.
pub type NativeUpdatePanelModel = self::native_shell_dtos::UpdatePanelModel;

/// Native runtime update status indicator model.
pub type NativeUpdateStatusModel = self::native_shell_dtos::UpdateStatusModel;

/// Native runtime waveform chrome model.
pub type NativeWaveformChromeModel = self::native_shell_dtos::WaveformChromeModel;

/// Native runtime waveform panel model.
pub type NativeWaveformPanelModel = self::native_shell_dtos::WaveformPanelModel;

/// Native runtime waveform channel-view model.
pub type NativeWaveformChannelViewModel = self::native_shell_dtos::WaveformChannelViewModel;

/// Native runtime waveform slice preview model.
pub type NativeWaveformSlicePreviewModel = self::native_shell_dtos::WaveformSlicePreviewModel;

/// Extract the numeric BPM token from a native waveform tempo label.
pub use self::native_shell_dtos::parse_waveform_tempo_number_text;

/// Native runtime bridge trait used by host launchers.
pub use radiant::compat::sempal_shell::NativeAppBridge;

#[cfg(test)]
mod tests;
