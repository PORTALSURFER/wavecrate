//! Backend-neutral state aliases for migration consumers.
//!
//! These aliases keep host-facing migration code (`app_core` projections and
//! native bridge action routing) independent from direct `egui_app::state`
//! module paths while the legacy controller internals are incrementally
//! extracted.

/// Alias for the legacy UI state root during migration.
pub type UiState = crate::egui_app::state::UiState;
/// Alias for map query bounds in similarity map projection.
pub type MapQueryBounds = crate::egui_app::state::MapQueryBounds;
/// Alias for browser tab selection state.
pub type SampleBrowserTab = crate::egui_app::state::SampleBrowserTab;
/// Alias for browser triage columns.
pub type TriageFlagColumn = crate::egui_app::state::TriageFlagColumn;
/// Alias for update status state.
pub type UpdateStatus = crate::egui_app::state::UpdateStatus;
/// Alias for map render mode state.
pub type MapRenderMode = crate::egui_app::state::MapRenderMode;
/// Alias for browser inline action prompts.
pub type SampleBrowserActionPrompt = crate::egui_app::state::SampleBrowserActionPrompt;
/// Alias for folder action prompts.
pub type FolderActionPrompt = crate::egui_app::state::FolderActionPrompt;
/// Alias for drag-and-drop target state.
pub type DragTarget = crate::egui_app::state::DragTarget;
/// Alias for browser sort state.
pub type SampleBrowserSort = crate::egui_app::state::SampleBrowserSort;
/// Alias for visible-row projection state.
pub type VisibleRows = crate::egui_app::state::VisibleRows;
/// Alias for destructive edit prompt state.
pub type DestructiveEditPrompt = crate::egui_app::state::DestructiveEditPrompt;
/// Alias for destructive waveform edit enum.
pub type DestructiveSelectionEdit = crate::egui_app::state::DestructiveSelectionEdit;
/// Alias for inline folder creation state.
pub type InlineFolderCreation = crate::egui_app::state::InlineFolderCreation;
/// Alias for folder row projection state.
pub type FolderRowView = crate::egui_app::state::FolderRowView;
/// Alias for folder delete recovery entry state.
pub type FolderDeleteRecoveryEntry = crate::egui_app::state::FolderDeleteRecoveryEntry;
/// Alias for folder delete recovery action state.
pub type FolderDeleteRecoveryAction = crate::egui_app::state::FolderDeleteRecoveryAction;
/// Alias for folder delete recovery status state.
pub type FolderDeleteRecoveryStatus = crate::egui_app::state::FolderDeleteRecoveryStatus;
