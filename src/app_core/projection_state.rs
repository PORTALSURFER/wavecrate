//! Projection, prompt, drag/drop, map, audio, status, and waveform state contracts owned by app-core.
//!
//! The legacy controller still stores these backing state values during the
//! runtime-facade migration. This module keeps the remaining projection-facing
//! aliases grouped behind a focused app-core ownership point instead of the
//! broad legacy state bridge.

// Map projection state.

/// Normalized map query bounds shared with map projection helpers.
pub type MapQueryBounds = crate::app::state::MapQueryBounds;

/// Cached map bounds used by migration-facing map projections.
pub type MapBounds = crate::app::state::MapBounds;

/// Cached projected map point payload used by migration-facing map projections.
pub type MapPoint = crate::app::state::MapPoint;

/// Map rendering mode exposed by migration-facing projections.
pub type MapRenderMode = crate::app::state::MapRenderMode;

// Options/audio status state.

/// Active audio-picker target shown in options flows.
pub type AudioPickerTarget = crate::app::state::AudioPickerTarget;

/// Active audio output description.
#[cfg(test)]
pub(crate) type ActiveAudioOutput = crate::app::state::ActiveAudioOutput;

/// Audio host option description.
#[cfg(test)]
pub(crate) type AudioHostView = crate::app::state::AudioHostView;

/// Audio device option description.
#[cfg(test)]
pub(crate) type AudioDeviceView = crate::app::state::AudioDeviceView;

/// Update status exposed by migration-facing projections.
pub type UpdateStatus = crate::app::state::UpdateStatus;

/// UI status tone used for app-level status messages.
pub type StatusTone = crate::app::state::StatusTone;

/// Progress task kind used by progress overlays.
#[cfg(test)]
pub(crate) type ProgressTaskKind = crate::app::state::ProgressTaskKind;

/// Progress overlay state used by controller action tests.
#[cfg(test)]
pub(crate) type ProgressOverlayState = crate::app::state::ProgressOverlayState;

/// Issue-token status used by controller action tests.
#[cfg(test)]
pub(crate) type IssueTokenStatus = crate::app::state::IssueTokenStatus;

// Prompt, drag/drop, and waveform state.

/// Pending modal folder action prompt.
pub type FolderActionPrompt = crate::app::state::FolderActionPrompt;

/// Pending options-panel confirmation prompt.
pub type OptionsPanelPrompt = crate::app::state::OptionsPanelPrompt;

/// Destructive edit action exposed by prompt surfaces.
pub type DestructiveSelectionEdit = crate::app::state::DestructiveSelectionEdit;

/// Prompt model for destructive edits.
pub type DestructiveEditPrompt = crate::app::state::DestructiveEditPrompt;

/// Unified drag target used by migration-facing drag/drop projections.
pub type DragTarget = crate::app::state::DragTarget;

/// Active drag payload.
pub type DragPayload = crate::app::state::DragPayload;

/// Drag source used by waveform and browser actions.
pub type DragSource = crate::app::state::DragSource;

/// Focus context shared by controller action routing.
pub type FocusContext = crate::app::state::FocusContext;

/// UI-space point for drag/drop and waveform interactions.
pub type UiPoint = crate::app::state::UiPoint;

/// Waveform comparison-anchor state.
#[cfg(test)]
pub(crate) type CompareAnchorState = crate::app::state::CompareAnchorState;

/// Waveform view state projected through migration-facing APIs.
#[cfg(test)]
pub(crate) type WaveformView = crate::app::state::WaveformView;

/// Waveform slice batch profile used by waveform projection and options tests.
pub type WaveformSliceBatchProfile = crate::app::state::WaveformSliceBatchProfile;

/// Waveform slice-review state used by options tests.
#[cfg(test)]
pub(crate) type WaveformSliceReviewState = crate::app::state::WaveformSliceReviewState;

/// Waveform duplicate-cleanup state used by options tests.
#[cfg(test)]
pub(crate) type WaveformDuplicateCleanupState = crate::app::state::WaveformDuplicateCleanupState;

/// Waveform duplicate-cleanup preview used by options tests.
#[cfg(test)]
pub(crate) type WaveformDuplicateCleanupPreview =
    crate::app::state::WaveformDuplicateCleanupPreview;
