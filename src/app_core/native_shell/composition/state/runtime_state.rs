//! Small runtime-state types owned by the native shell state façade.

use super::{CachedFolderRow, FolderRowsCacheKey};

/// Color mode used for the transient waveform selection export flash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaveformSelectionFlashTone {
    /// Optimistic submit feedback shown as soon as the export is queued.
    Optimistic,
    /// Error feedback shown when an async export later fails.
    Error,
}

/// Cached runtime state for one sidebar folder pane.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::app_core::native_shell::composition::state) struct FolderPaneRuntimeState {
    pub(in crate::app_core::native_shell::composition::state) rows: Vec<CachedFolderRow>,
    pub(in crate::app_core::native_shell::composition::state) window_start: usize,
    pub(in crate::app_core::native_shell::composition::state) autoscroll: bool,
    pub(in crate::app_core::native_shell::composition::state) last_focused_row: Option<usize>,
    pub(in crate::app_core::native_shell::composition::state) cache_key: Option<FolderRowsCacheKey>,
}

impl Default for FolderPaneRuntimeState {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            window_start: 0,
            autoscroll: true,
            last_focused_row: None,
            cache_key: None,
        }
    }
}
