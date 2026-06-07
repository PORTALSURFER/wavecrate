//! Shared native-app names for the current flat module layout.
//!
//! Keep this file limited to names that are used across several native-app
//! modules. As feature areas move into focused submodules, prefer importing
//! from the owning module directly and shrinking this scope.

pub(super) use super::state::{
    ActiveFolderCacheWarmResult, AppSettingsTab, AudioSettingsDropdown, GuiMessage, NativeAppState,
    NativeFileDropHover, NormalizationProgress, NormalizationResult, PendingPlaybackStart,
    PendingSamplePlayback, SampleLoadResult, SampleNameViewMode, SamplePlaybackReady,
    WaveformCacheEntry, WaveformCacheIndicatorRefreshResult, WaveformCacheWarmResult,
};
pub(super) use super::waveform::{
    WaveformActiveDragKind, WaveformInteraction, WaveformSelectionKind, WaveformState,
};
pub(super) use crate::native_app::audio::audio_settings::format_sample_rate_label;
pub(super) use crate::native_app::audio::sample_load_actions::{
    NormalizedWaveformReload, WaveformPlaybackResume,
};
pub(super) use crate::native_app::browser::file_actions::sample_path_label;
pub(super) use crate::native_app::browser::folder_browser::{
    FileMoveConflictResolution, FolderBrowserMessage, FolderBrowserState, FolderScanProgress,
};
pub(super) use crate::native_app::chrome::layout::view;
pub(super) use crate::native_app::shell::emit_gui_action;
pub(super) use crate::native_app::shell::shortcuts::default_gui_shortcut_resolution;
pub(super) use wavecrate::logging;
