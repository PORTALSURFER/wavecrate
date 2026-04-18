//! Migration-facing aliases and catalog helpers for native runtime action/model types.
//!
//! These aliases centralize runtime-facing type dependencies in `app_core`,
//! so bridge/controller glue does not import concrete runtime types directly.
//! They intentionally point at `radiant::compat::sempal_shell` because the
//! current shell contract is compatibility infrastructure, not the preferred
//! generic Radiant API.

mod catalog;

pub use self::catalog::{
    GUI_ACTION_CATALOG, GuiActionCatalogEntry, GuiActionKind, GuiCoverageLayer, GuiDispatchPolicy,
    GuiEffectClass, GuiHistoryPolicy, GuiSurface, action_catalog_entry, action_catalog_entry_by_id,
    action_kind, representative_action_for_kind,
};

/// Native runtime browser action metadata model.
pub type NativeBrowserActionsModel = radiant::compat::sempal_shell::BrowserActionsModel;

/// Native runtime browser chrome model.
pub type NativeBrowserChromeModel = radiant::compat::sempal_shell::BrowserChromeModel;

/// Native runtime browser panel model.
pub type NativeBrowserPanelModel = radiant::compat::sempal_shell::BrowserPanelModel;

/// Native runtime browser tag-sidebar model.
pub type NativeBrowserTagSidebarModel = radiant::compat::sempal_shell::BrowserTagSidebarModel;

/// Native runtime browser playback-age row bucket model.
pub type NativePlaybackAgeBucket = radiant::compat::sempal_shell::PlaybackAgeBucket;

/// Native runtime browser playback-age toolbar chip model.
pub type NativePlaybackAgeFilterChip = radiant::compat::sempal_shell::PlaybackAgeFilterChip;

/// Native runtime browser row model.
pub type NativeBrowserRowModel = radiant::compat::sempal_shell::BrowserRowModel;

/// Native runtime automation node identifier.
pub type NativeAutomationNodeId = radiant::compat::sempal_shell::AutomationNodeId;

/// Native runtime automation node role.
pub type NativeAutomationRole = radiant::compat::sempal_shell::AutomationRole;

/// Native runtime automation node snapshot.
pub type NativeAutomationNodeSnapshot = radiant::compat::sempal_shell::AutomationNodeSnapshot;

/// Native runtime GUI automation snapshot.
pub type NativeGuiAutomationSnapshot = radiant::compat::sempal_shell::GuiAutomationSnapshot;

/// Native runtime browser tag target used by keyboard and pointer triage actions.
pub type NativeBrowserTagTarget = radiant::compat::sempal_shell::BrowserTagTarget;

/// Native runtime UI action payload.
pub type NativeUiAction = radiant::compat::sempal_shell::UiAction;

/// Native runtime projected app model.
pub type NativeAppModel = radiant::compat::sempal_shell::AppModel;

/// Native runtime audio-engine chip state.
pub type NativeAudioEngineChipStateModel = radiant::compat::sempal_shell::AudioEngineChipStateModel;

/// Native runtime audio-engine projection model.
pub type NativeAudioEngineModel = radiant::compat::sempal_shell::AudioEngineModel;

/// Native runtime audio-field summary model.
pub type NativeAudioFieldModel = radiant::compat::sempal_shell::AudioFieldModel;

/// Native runtime audio picker option item.
pub type NativeAudioOptionItemModel = radiant::compat::sempal_shell::AudioOptionItemModel;

/// Native runtime audio picker option raw value.
pub type NativeAudioOptionValueModel = radiant::compat::sempal_shell::AudioOptionValueModel;

/// Native runtime audio picker target.
pub type NativeAudioPickerTargetModel = radiant::compat::sempal_shell::AudioPickerTargetModel;

/// Native runtime focus-context model used for contextual input routing.
pub type NativeFocusContextModel = radiant::compat::sempal_shell::FocusContextModel;

/// Native runtime frame build result payload.
pub type NativeFrameBuildResult = radiant::compat::sempal_shell::FrameBuildResult;

/// Native runtime projection-dirty segment mask.
pub type NativeDirtySegments = radiant::compat::sempal_shell::DirtySegments;

/// Native runtime static-segment revision snapshot.
pub type NativeSegmentRevisions = radiant::compat::sempal_shell::SegmentRevisions;

/// Native runtime motion-only model payload.
pub type NativeMotionModel = radiant::compat::sempal_shell::NativeMotionModel;

/// Native runtime table column summary model.
pub type NativeColumnModel = radiant::compat::sempal_shell::ColumnModel;

/// Native runtime confirm prompt kind descriptor.
pub type NativeConfirmPromptKind = radiant::compat::sempal_shell::ConfirmPromptKind;

/// Native runtime confirm prompt model.
pub type NativeConfirmPromptModel = radiant::compat::sempal_shell::ConfirmPromptModel;

/// Native runtime drag overlay model.
pub type NativeDragOverlayModel = radiant::compat::sempal_shell::DragOverlayModel;

/// Native runtime folder actions model.
pub type NativeFolderActionsModel = radiant::compat::sempal_shell::FolderActionsModel;

/// Native runtime folder recovery model.
pub type NativeFolderRecoveryModel = radiant::compat::sempal_shell::FolderRecoveryModel;

/// Native runtime folder row model.
pub type NativeFolderRowModel = radiant::compat::sempal_shell::FolderRowModel;

/// Native runtime folder row kind.
pub type NativeFolderRowKind = radiant::compat::sempal_shell::FolderRowKind;

/// Native runtime map panel model.
pub type NativeMapPanelModel = radiant::compat::sempal_shell::MapPanelModel;

/// Native runtime map point model.
pub type NativeMapPointModel = radiant::compat::sempal_shell::MapPointModel;

/// Native runtime map render mode model.
pub type NativeMapRenderModeModel = radiant::compat::sempal_shell::MapRenderModeModel;

/// Native runtime options panel model.
pub type NativeOptionsPanelModel = radiant::compat::sempal_shell::OptionsPanelModel;

/// Native runtime normalized range model.
pub type NativeNormalizedRangeModel = radiant::compat::sempal_shell::NormalizedRangeModel;

/// Native runtime progress overlay model.
pub type NativeProgressOverlayModel = radiant::compat::sempal_shell::ProgressOverlayModel;

/// Native runtime source row model.
pub type NativeSourceRowModel = radiant::compat::sempal_shell::SourceRowModel;

/// Native runtime sources panel model.
pub type NativeSourcesPanelModel = radiant::compat::sempal_shell::SourcesPanelModel;

/// Native runtime status bar model.
pub type NativeStatusBarModel = radiant::compat::sempal_shell::StatusBarModel;

/// Native runtime update panel model.
pub type NativeUpdatePanelModel = radiant::compat::sempal_shell::UpdatePanelModel;

/// Native runtime update status indicator model.
pub type NativeUpdateStatusModel = radiant::compat::sempal_shell::UpdateStatusModel;

/// Native runtime waveform chrome model.
pub type NativeWaveformChromeModel = radiant::compat::sempal_shell::WaveformChromeModel;

/// Native runtime waveform panel model.
pub type NativeWaveformPanelModel = radiant::compat::sempal_shell::WaveformPanelModel;

/// Native runtime bridge trait used by host launchers.
pub use radiant::compat::sempal_shell::NativeAppBridge;

#[cfg(test)]
mod tests;
