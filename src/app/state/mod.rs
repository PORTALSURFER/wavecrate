//! Shared state types for the application UI layer.
//! These types are consumed by runtime-integrated renderers and migration-facing
//! projection glue.

mod audio;
mod browser;
mod controls;
mod compare;
mod drag;
mod feedback_issue;
mod focus;
mod geometry;
mod hotkeys;
mod loop_crossfade;
mod map;
mod options_panel;
mod progress;
/// Canonical projection invalidation revision counters.
mod revisions;
mod sources;
mod status;
mod update;
mod waveform;

pub use audio::*;
pub use browser::*;
pub use controls::*;
pub use compare::*;
pub use drag::*;
pub use feedback_issue::*;
pub use focus::*;
pub use geometry::*;
pub use hotkeys::*;
pub use loop_crossfade::*;
pub use map::*;
pub use options_panel::*;
pub use progress::*;
pub use revisions::*;
pub use sources::*;
pub use status::*;
pub use update::*;
pub use waveform::*;

use std::path::PathBuf;

/// Top-level UI model consumed by host and migration-facing projection surfaces.
#[derive(Clone, Debug)]
pub struct UiState {
    /// Status bar and status log state.
    pub status: StatusBarState,
    /// Source list and folder browser state.
    pub sources: SourcePanelState,
    /// Sample browser state.
    pub browser: SampleBrowserState,
    /// Waveform rendering and selection state.
    pub waveform: WaveformState,
    /// Drag-and-drop state.
    pub drag: DragState,
    /// Overlay for long-running tasks.
    pub progress: ProgressOverlayState,
    /// Tracks which UI region currently owns keyboard focus.
    pub focus: UiFocusState,
    /// UI state for contextual hotkey affordances.
    pub hotkeys: HotkeyUiState,
    /// Feedback prompt state for filing GitHub issues.
    pub feedback_issue: FeedbackIssueUiState,
    /// Audio device/options UI state.
    pub audio: AudioOptionsState,
    /// 2D map explorer state.
    pub map: MapUiState,
    /// Interaction and navigation tuning options.
    pub controls: InteractionOptionsState,
    /// Native-shell options panel state.
    pub options_panel: OptionsPanelState,
    /// Active compare-anchor metadata used by transport compare affordances.
    pub compare_anchor: Option<CompareAnchorState>,
    /// Pending loop crossfade prompt state.
    pub loop_crossfade_prompt: Option<LoopCrossfadePrompt>,
    /// Master output volume (0.0-1.0).
    pub volume: f32,
    /// Release update status / notification state.
    pub update: UpdateUiState,
    /// Currently loaded wav path, if any.
    pub loaded_wav: Option<PathBuf>,
    /// Canonical revision counters for projection invalidation.
    pub projection_revisions: UiProjectionRevisions,
    /// Optional trash folder path configured by the user.
    pub trash_folder: Option<PathBuf>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            status: StatusBarState::idle(),
            sources: SourcePanelState::default(),
            browser: SampleBrowserState::default(),
            waveform: WaveformState::default(),
            drag: DragState::default(),
            progress: ProgressOverlayState::default(),
            focus: UiFocusState::default(),
            hotkeys: HotkeyUiState::default(),
            feedback_issue: FeedbackIssueUiState::default(),
            audio: AudioOptionsState::default(),
            map: MapUiState::default(),
            controls: InteractionOptionsState::default(),
            options_panel: OptionsPanelState::default(),
            compare_anchor: None,
            loop_crossfade_prompt: None,
            volume: 1.0,
            update: UpdateUiState::default(),
            loaded_wav: None,
            projection_revisions: UiProjectionRevisions::default(),
            trash_folder: None,
        }
    }
}
