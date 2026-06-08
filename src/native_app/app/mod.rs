mod cache;
mod drop;
mod loading;
mod message;
mod progress;
mod settings;
mod state;

pub(in crate::native_app) use cache::{
    ActiveFolderCacheWarmResult, WaveformCacheEntry, WaveformCacheIndicatorRefreshResult,
    WaveformCacheWarmResult,
};
pub(in crate::native_app) use drop::NativeFileDropHover;
pub(in crate::native_app) use loading::{
    PendingPlaybackStart, PendingSamplePlayback, SampleLoadResult, SamplePlaybackReady,
};
pub(in crate::native_app) use message::GuiMessage;
pub(in crate::native_app) use progress::{NormalizationProgress, NormalizationResult};
pub(in crate::native_app) use settings::{
    AppSettingsTab, AudioSettingsDropdown, SampleNameViewMode,
};
#[cfg(test)]
pub(in crate::native_app) use state::DEFAULT_VOLUME;
pub(in crate::native_app) use state::NativeAppState;

pub(super) use crate::native_app::app_chrome::layers::view;
pub(super) use crate::native_app::audio::sample_load_actions::{
    NormalizedWaveformReload, WaveformPlaybackResume,
};
pub(super) use crate::native_app::sample_library::file_actions::sample_path_label;
pub(super) use crate::native_app::sample_library::folder_browser::{
    FileMoveConflictResolution, FolderBrowserMessage, FolderBrowserState, FolderScanProgress,
};
pub(super) use crate::native_app::shell::emit_gui_action;
pub(super) use crate::native_app::shell::shortcuts::default_gui_shortcuts;
pub(super) use crate::native_app::ui::display::format_sample_rate_label;
pub(super) use crate::native_app::waveform::{
    WaveformActiveDragKind, WaveformInteraction, WaveformSelectionKind, WaveformState,
};
pub(super) use wavecrate::logging;
