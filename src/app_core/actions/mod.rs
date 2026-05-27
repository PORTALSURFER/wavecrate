//! Migration-facing aliases and catalog helpers for native runtime action/model types.
//!
//! These aliases centralize runtime-facing type dependencies in `app_core`,
//! so bridge/controller glue does not import concrete runtime types directly.
//! Wavecrate-owned projection DTOs live here while Radiant-specific compatibility
//! types remain isolated to the runtime boundary.

mod catalog;
mod ui_projection_actions;
mod ui_projection_bridge;
pub(crate) mod ui_projection_dtos;

pub use self::catalog::{
    GUI_ACTION_CATALOG, GuiActionCatalogEntry, GuiActionKind, GuiCoverageLayer, GuiDispatchPolicy,
    GuiEffectClass, GuiHistoryPolicy, GuiSurface, action_catalog_entry, action_catalog_entry_by_id,
    action_kind, representative_action_for_kind,
};
pub use self::ui_projection_dtos::folder_row_model as native_folder_row_model;

/// Native runtime browser action metadata model.
pub type NativeBrowserActionsModel = self::ui_projection_dtos::BrowserActionsModel;

/// Native runtime browser chrome model.
pub type NativeBrowserChromeModel = self::ui_projection_dtos::BrowserChromeModel;

/// Native runtime browser panel model.
pub type NativeBrowserPanelModel = self::ui_projection_dtos::BrowserPanelModel;

/// Native runtime browser tag-sidebar model.
pub type NativeBrowserTagSidebarModel = self::ui_projection_dtos::BrowserTagSidebarModel;

/// Native runtime browser tag pill model.
pub type NativeBrowserTagPillModel = self::ui_projection_dtos::BrowserTagPillModel;

/// Native runtime browser tag pill state.
pub type NativeBrowserTagState = self::ui_projection_dtos::BrowserTagState;

/// Native runtime browser playback-age row bucket model.
pub type NativePlaybackAgeBucket = self::ui_projection_dtos::PlaybackAgeBucket;

/// Native runtime browser playback-age toolbar action payload.
pub type NativePlaybackAgeFilterChip = self::ui_projection_dtos::PlaybackAgeFilterChip;

/// Native runtime browser row model.
pub type NativeBrowserRowModel = self::ui_projection_dtos::BrowserRowModel;

/// Native runtime browser row processing state.
pub type NativeBrowserRowProcessingState = self::ui_projection_dtos::BrowserRowProcessingState;

/// Native runtime retained vector used by Wavecrate-owned projection DTOs.
pub type NativeRetainedVec<T> = self::ui_projection_dtos::RetainedVec<T>;

/// Native runtime automation node identifier.
pub type NativeAutomationNodeId = self::ui_projection_dtos::AutomationNodeId;

/// Native runtime automation node role.
pub type NativeAutomationRole = self::ui_projection_dtos::AutomationRole;

/// Native runtime automation node bounds.
pub type NativeAutomationBounds = self::ui_projection_dtos::AutomationBounds;

/// Native runtime automation node snapshot.
pub type NativeAutomationNodeSnapshot = self::ui_projection_dtos::AutomationNodeSnapshot;

/// Native runtime GUI automation snapshot.
pub type NativeGuiAutomationSnapshot = self::ui_projection_dtos::GuiAutomationSnapshot;

/// Native runtime browser tag target used by keyboard and pointer triage actions.
pub type NativeBrowserTagTarget = self::ui_projection_actions::BrowserTagTarget;

/// Native runtime UI action payload.
pub type NativeUiAction = self::ui_projection_actions::UiAction;

/// Native runtime projected app model.
pub type NativeAppModel = self::ui_projection_dtos::AppModel;

/// Native runtime audio-engine chip state.
pub type NativeAudioEngineChipStateModel = self::ui_projection_dtos::AudioEngineChipStateModel;

/// Native runtime audio-engine projection model.
pub type NativeAudioEngineModel = self::ui_projection_dtos::AudioEngineModel;

/// Native runtime audio-field summary model.
pub type NativeAudioFieldModel = self::ui_projection_dtos::AudioFieldModel;

/// Native runtime audio picker option item.
pub type NativeAudioOptionItemModel = self::ui_projection_dtos::AudioOptionItemModel;

/// Native runtime audio picker option raw value.
pub type NativeAudioOptionValueModel = self::ui_projection_dtos::AudioOptionValueModel;

/// Native runtime audio picker target.
pub type NativeAudioPickerTargetModel = self::ui_projection_dtos::AudioPickerTargetModel;

/// Native runtime focus-context model used for contextual input routing.
pub type NativeFocusContextModel = self::ui_projection_dtos::FocusContextModel;

/// Native runtime frame build result payload.
pub type NativeFrameBuildResult = self::ui_projection_dtos::FrameBuildResult;

/// Native runtime projection-dirty segment mask.
pub type NativeDirtySegments = self::ui_projection_dtos::DirtySegments;

/// Native runtime static-segment revision snapshot.
pub type NativeSegmentRevisions = self::ui_projection_dtos::SegmentRevisions;

/// Native runtime motion-only model payload.
pub type NativeMotionModel = self::ui_projection_dtos::NativeMotionModel;

/// Native runtime table column summary model.
pub type NativeColumnModel = self::ui_projection_dtos::ColumnModel;

/// Native runtime confirm prompt kind descriptor.
pub type NativeConfirmPromptKind = self::ui_projection_dtos::ConfirmPromptKind;

/// Native runtime confirm prompt model.
pub type NativeConfirmPromptModel = self::ui_projection_dtos::ConfirmPromptModel;

/// Native runtime drag overlay model.
pub type NativeDragOverlayModel = self::ui_projection_dtos::DragOverlayModel;

/// Native runtime folder actions model.
pub type NativeFolderActionsModel = self::ui_projection_dtos::FolderActionsModel;

/// Native runtime folder pane identifier.
pub type NativeFolderPaneIdModel = self::ui_projection_dtos::FolderPaneIdModel;

/// Native runtime folder pane model.
pub type NativeFolderPaneModel = self::ui_projection_dtos::FolderPaneModel;

/// Native runtime folder recovery model.
pub type NativeFolderRecoveryModel = self::ui_projection_dtos::FolderRecoveryModel;

/// Native runtime folder row model.
pub type NativeFolderRowModel = self::ui_projection_dtos::FolderRowModel;

/// Native runtime folder row kind.
pub type NativeFolderRowKind = self::ui_projection_dtos::FolderRowKind;

/// Native runtime map panel model.
pub type NativeMapPanelModel = self::ui_projection_dtos::MapPanelModel;

/// Native runtime map point model.
pub type NativeMapPointModel = self::ui_projection_dtos::MapPointModel;

/// Native runtime map render mode model.
pub type NativeMapRenderModeModel = self::ui_projection_dtos::MapRenderModeModel;

/// Native runtime options panel model.
pub type NativeOptionsPanelModel = self::ui_projection_dtos::OptionsPanelModel;

/// Native runtime normalized range model.
pub type NativeNormalizedRangeModel = self::ui_projection_dtos::NormalizedRangeModel;

/// Native runtime progress overlay model.
pub type NativeProgressOverlayModel = self::ui_projection_dtos::ProgressOverlayModel;

/// Native runtime source row model.
pub type NativeSourceRowModel = self::ui_projection_dtos::SourceRowModel;

/// Native runtime sources panel model.
pub type NativeSourcesPanelModel = self::ui_projection_dtos::SourcesPanelModel;

/// Native runtime status bar model.
pub type NativeStatusBarModel = self::ui_projection_dtos::StatusBarModel;

/// Native runtime update panel model.
pub type NativeUpdatePanelModel = self::ui_projection_dtos::UpdatePanelModel;

/// Native runtime update status indicator model.
pub type NativeUpdateStatusModel = self::ui_projection_dtos::UpdateStatusModel;

/// Native runtime waveform chrome model.
pub type NativeWaveformChromeModel = self::ui_projection_dtos::WaveformChromeModel;

/// Native runtime waveform panel model.
pub type NativeWaveformPanelModel = self::ui_projection_dtos::WaveformPanelModel;

/// Native runtime waveform channel-view model.
pub type NativeWaveformChannelViewModel = self::ui_projection_dtos::WaveformChannelViewModel;

/// Native runtime waveform slice preview model.
pub type NativeWaveformSlicePreviewModel = self::ui_projection_dtos::WaveformSlicePreviewModel;

/// Native runtime waveform tool state model.
pub type NativeWaveformToolStateModel = self::ui_projection_dtos::WaveformToolStateModel;

/// Native runtime waveform motion aggregate model.
pub type NativeWaveformMotionModel = self::ui_projection_dtos::WaveformMotionModel;

/// Extract the numeric BPM token from a native waveform tempo label.
pub use self::ui_projection_dtos::parse_waveform_tempo_number_text;

/// Native runtime bridge trait and native file-drop payloads used by host launchers.
pub use self::ui_projection_bridge::{NativeAppBridge, NativeFileDropEvent, NativeFileDropPhase};

#[cfg(test)]
mod tests;
