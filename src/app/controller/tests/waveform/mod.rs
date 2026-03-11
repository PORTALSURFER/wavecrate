//! Waveform controller regression tests grouped by behavior.

/// Direct destructive edit and confirmation behavior.
mod destructive_edits;
/// Waveform hotkey routing and normalization behavior.
mod hotkeys_normalize;
/// Waveform loading and reset behavior.
mod loading_reset;
/// Playback state preservation during waveform edits.
mod playback;
