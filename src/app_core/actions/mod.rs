//! Migration-facing aliases and catalog helpers for UI runtime action/model types.
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
    GUI_ACTION_CATALOG, GUI_ACTION_CATALOG_DOMAINS, GuiActionCatalogEntry, GuiActionKind,
    GuiCoverageLayer, GuiDispatchPolicy, GuiEffectClass, GuiHistoryPolicy, GuiSurface,
    action_catalog_entries_by_domain, action_catalog_entry, action_catalog_entry_by_id,
    action_kind, representative_action_for_kind,
};
pub use self::ui_projection_dtos::folder_row_model as native_folder_row_model;

/// UI runtime browser chrome model.
pub type NativeBrowserChromeModel = self::ui_projection_dtos::BrowserChromeModel;

/// UI runtime browser panel model.
pub type NativeBrowserPanelModel = self::ui_projection_dtos::BrowserPanelModel;

/// UI runtime browser tag-sidebar model.
pub type NativeBrowserTagSidebarModel = self::ui_projection_dtos::BrowserTagSidebarModel;

/// UI runtime browser tag pill state.
pub type NativeBrowserTagState = self::ui_projection_dtos::BrowserTagState;

/// UI runtime browser playback-age row bucket model.
pub type NativePlaybackAgeBucket = self::ui_projection_dtos::PlaybackAgeBucket;

/// UI runtime browser playback-age toolbar action payload.
pub type NativePlaybackAgeFilterChip = self::ui_projection_dtos::PlaybackAgeFilterChip;

/// UI runtime browser row model.
pub type NativeBrowserRowModel = self::ui_projection_dtos::BrowserRowModel;

/// UI runtime browser row processing state.
pub type NativeBrowserRowProcessingState = self::ui_projection_dtos::BrowserRowProcessingState;

/// UI runtime retained vector used by Wavecrate-owned projection DTOs.
pub type NativeRetainedVec<T> = self::ui_projection_dtos::RetainedVec<T>;

/// UI runtime automation node identifier.
pub type NativeAutomationNodeId = self::ui_projection_dtos::AutomationNodeId;

/// UI runtime automation node role.
pub type NativeAutomationRole = self::ui_projection_dtos::AutomationRole;

/// UI runtime automation node bounds.
pub type NativeAutomationBounds = self::ui_projection_dtos::AutomationBounds;

/// UI runtime automation node snapshot.
pub type NativeAutomationNodeSnapshot = self::ui_projection_dtos::AutomationNodeSnapshot;

/// UI runtime GUI automation snapshot.
pub type NativeGuiAutomationSnapshot = self::ui_projection_dtos::GuiAutomationSnapshot;

/// UI runtime browser tag target used by keyboard and pointer triage actions.
pub type NativeBrowserTagTarget = self::ui_projection_actions::BrowserTagTarget;

/// UI runtime UI action payload.
pub type NativeUiAction = self::ui_projection_actions::UiAction;

/// UI runtime or artifact input that can upgrade retained legacy action payloads.
pub type NativeRetainedUiAction = self::ui_projection_actions::RetainedUiAction;

/// UI runtime UI action domain family.
pub type NativeUiActionDomain = self::ui_projection_actions::UiActionDomain;

/// UI runtime current column triage action payload.
pub type NativeColumnTriageAction = self::ui_projection_actions::ColumnTriageAction;

/// UI runtime browser action payload.
pub type NativeBrowserAction = self::ui_projection_actions::BrowserAction;

/// UI runtime transport action payload.
pub type NativeTransportAction = self::ui_projection_actions::TransportAction;

/// UI runtime history and update action payload.
pub type NativeHistoryUpdateAction = self::ui_projection_actions::HistoryUpdateAction;

/// UI runtime prompt and edit action payload.
pub type NativePromptEditAction = self::ui_projection_actions::PromptEditAction;

/// UI runtime shell action payload.
pub type NativeShellAction = self::ui_projection_actions::ShellAction;

/// UI runtime sources and folders action payload.
pub type NativeSourcesFoldersAction = self::ui_projection_actions::SourcesFoldersAction;

/// UI runtime options and audio-settings action payload.
pub type NativeOptionsAction = self::ui_projection_actions::OptionsAction;

/// UI runtime waveform action payload.
pub type NativeWaveformAction = self::ui_projection_actions::WaveformAction;

/// UI runtime projected app model.
pub type NativeAppModel = self::ui_projection_dtos::AppModel;

/// UI runtime audio-engine chip state.
pub type NativeAudioEngineChipStateModel = self::ui_projection_dtos::AudioEngineChipStateModel;

/// UI runtime audio-engine projection model.
pub type NativeAudioEngineModel = self::ui_projection_dtos::AudioEngineModel;

/// UI runtime audio-field summary model.
pub type NativeAudioFieldModel = self::ui_projection_dtos::AudioFieldModel;

/// UI runtime audio picker option item.
pub type NativeAudioOptionItemModel = self::ui_projection_dtos::AudioOptionItemModel;

/// UI runtime audio picker option raw value.
pub type NativeAudioOptionValueModel = self::ui_projection_dtos::AudioOptionValueModel;

/// UI runtime audio picker target.
pub type NativeAudioPickerTargetModel = self::ui_projection_dtos::AudioPickerTargetModel;

/// UI runtime focus-context model used for contextual input routing.
pub type NativeFocusContextModel = self::ui_projection_dtos::FocusContextModel;

/// UI runtime frame build result payload.
pub type NativeFrameBuildResult = self::ui_projection_dtos::FrameBuildResult;

/// UI runtime projection-dirty segment mask.
pub type NativeDirtySegments = self::ui_projection_dtos::DirtySegments;

/// UI runtime static-segment revision snapshot.
pub type NativeSegmentRevisions = self::ui_projection_dtos::SegmentRevisions;

/// UI runtime motion-only model payload.
pub type NativeMotionModel = self::ui_projection_dtos::NativeMotionModel;

/// UI runtime table column summary model.
pub type NativeColumnModel = self::ui_projection_dtos::ColumnModel;

/// UI runtime confirm prompt kind descriptor.
pub type NativeConfirmPromptKind = self::ui_projection_dtos::ConfirmPromptKind;

/// UI runtime confirm prompt model.
pub type NativeConfirmPromptModel = self::ui_projection_dtos::ConfirmPromptModel;

/// UI runtime drag overlay model.
pub type NativeDragOverlayModel = self::ui_projection_dtos::DragOverlayModel;

/// UI runtime folder pane identifier.
pub type NativeFolderPaneIdModel = self::ui_projection_dtos::FolderPaneIdModel;

/// UI runtime folder row model.
pub type NativeFolderRowModel = self::ui_projection_dtos::FolderRowModel;

/// UI runtime folder row kind.
pub type NativeFolderRowKind = self::ui_projection_dtos::FolderRowKind;

/// UI runtime map panel model.
pub type NativeMapPanelModel = self::ui_projection_dtos::MapPanelModel;

/// UI runtime map point model.
pub type NativeMapPointModel = self::ui_projection_dtos::MapPointModel;

/// UI runtime options panel model.
pub type NativeOptionsPanelModel = self::ui_projection_dtos::OptionsPanelModel;

/// UI runtime normalized range model.
pub type NativeNormalizedRangeModel = self::ui_projection_dtos::NormalizedRangeModel;

/// UI runtime progress overlay model.
pub type NativeProgressOverlayModel = self::ui_projection_dtos::ProgressOverlayModel;

/// UI runtime source row model.
pub type NativeSourceRowModel = self::ui_projection_dtos::SourceRowModel;

/// UI runtime sources panel model.
pub type NativeSourcesPanelModel = self::ui_projection_dtos::SourcesPanelModel;

/// UI runtime status bar model.
pub type NativeStatusBarModel = self::ui_projection_dtos::StatusBarModel;

/// UI runtime update panel model.
pub type NativeUpdatePanelModel = self::ui_projection_dtos::UpdatePanelModel;

/// UI runtime update status indicator model.
pub type NativeUpdateStatusModel = self::ui_projection_dtos::UpdateStatusModel;

/// UI runtime waveform chrome model.
pub type NativeWaveformChromeModel = self::ui_projection_dtos::WaveformChromeModel;

/// UI runtime waveform panel model.
pub type NativeWaveformPanelModel = self::ui_projection_dtos::WaveformPanelModel;

/// UI runtime waveform channel-view model.
pub type NativeWaveformChannelViewModel = self::ui_projection_dtos::WaveformChannelViewModel;

/// UI runtime waveform slice preview model.
pub type NativeWaveformSlicePreviewModel = self::ui_projection_dtos::WaveformSlicePreviewModel;

/// Extract the numeric BPM token from a native waveform tempo label.
pub use self::ui_projection_dtos::parse_waveform_tempo_number_text;

/// UI runtime bridge trait and native file-drop payloads used by host launchers.
pub use self::ui_projection_bridge::{NativeAppBridge, NativeFileDropEvent, NativeFileDropPhase};

#[cfg(test)]
mod tests;
