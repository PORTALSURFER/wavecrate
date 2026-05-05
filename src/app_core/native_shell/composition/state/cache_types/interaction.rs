use super::*;

/// Ephemeral sidebar source-menu state tracked by the runtime.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct SourceContextMenuState {
    /// Pane containing the source row that opened the menu.
    pub pane: crate::compat_app_contract::FolderPaneIdModel,
    /// Source row index the menu actions target.
    pub row_index: usize,
    /// Pointer anchor used to place the floating menu panel.
    pub anchor: Point,
}

/// Ephemeral browser row context-menu state tracked by the runtime.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserContextMenuState {
    /// Browser visible-row index the menu actions target.
    pub visible_row: usize,
    /// Pointer anchor used to place the floating menu panel.
    pub anchor: Point,
}

/// Invalidation key for the retained browser scrollbar interaction geometry.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserScrollbarCacheKey {
    /// The resolved browser-row cache key the scrollbar is derived from.
    pub rows_key: BrowserRowsCacheKey,
}

/// One retained playhead x-position point used to build ghost-line trails.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct PlayheadTrailPoint {
    /// Normalized x-position in `0.0..=1.0`.
    pub ratio: f32,
    /// Monotonic animation clock value when this point was captured.
    pub captured_at_seconds: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::gui::native_shell::state) struct NativeAnimationReasons {
    pub transport_running: bool,
    pub startup_frame_tick: bool,
    pub playhead_trail_active: bool,
    pub waveform_toolbar_flash_active: bool,
    pub waveform_selection_flash_active: bool,
    pub waveform_edit_selection_flash_active: bool,
    pub source_add_button_flash_active: bool,
    pub status_options_button_flash_active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui::native_shell::state) struct WaveformToolbarFlash {
    pub hint: WaveformToolbarHoverHint,
    pub ticks_remaining: u8,
}

impl NativeAnimationReasons {
    pub(in crate::gui::native_shell::state) fn needs_animation(self) -> bool {
        self.transport_running
            || self.startup_frame_tick
            || self.playhead_trail_active
            || self.waveform_toolbar_flash_active
            || self.waveform_selection_flash_active
            || self.waveform_edit_selection_flash_active
            || self.source_add_button_flash_active
            || self.status_options_button_flash_active
    }
}
