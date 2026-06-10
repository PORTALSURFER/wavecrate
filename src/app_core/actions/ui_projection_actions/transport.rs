use serde::{Deserialize, Serialize};

/// Transport and global playback actions emitted by the UI runtime input layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransportAction {
    /// Toggle transport playback state.
    ToggleTransport,
    /// Replay the stored compare-anchor sample without changing browser focus.
    PlayCompareAnchor,
    /// Start playback from the beginning of the active sample.
    PlayFromStart,
    /// Start playback from the current playhead or cursor position.
    PlayFromCurrentPlayhead,
    /// Start playback from the current waveform cursor position.
    ///
    /// Plain waveform click-release uses this action so playback starts from
    /// the exact clicked point instead of reusing any older visible playhead
    /// position.
    PlayFromWaveformCursor,
    /// Start playback immediately from one exact waveform position.
    ///
    /// Plain waveform click-release uses this direct action so the host can
    /// seek and start playback from the clicked point in one step without
    /// inferring intent from the cursor or visible playhead state.
    PlayWaveformAtPrecise {
        /// Normalized nanounit playback target (`0..=1_000_000_000`).
        position_nanos: u32,
    },
    /// Handle Escape key behavior for playback, selection, and cursor cleanup.
    HandleEscape,
}
