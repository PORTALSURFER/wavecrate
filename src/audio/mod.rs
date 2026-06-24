//! Compatibility exports for Wavecrate's reusable audio foundation.
//!
//! Generic audio infrastructure lives in the `reson` crate. This module keeps
//! existing Wavecrate imports stable and owns conversion from Wavecrate-specific
//! selection state into neutral realtime audio fade ranges.

pub use reson::{
    AudioDeviceSummary, AudioHostSummary, AudioInputConfig, AudioInputError, AudioOutputConfig,
    AudioOutputError, AudioPlayer, AudioRecorder, EditFadeRange, FadeParams, InputMonitor,
    PlaybackMetronomeConfig, PlaybackRequestId, PlaybackRuntime, PlaybackRuntimeCancellation,
    PlaybackRuntimeConfig, PlaybackRuntimeEvent, PlaybackRuntimeHandle, PlaybackRuntimeMode,
    PlaybackRuntimeProgress, PlaybackRuntimeRequest, PlaybackRuntimeSource, PlaybackRuntimeStarted,
    PlaybackRuntimeSubmitError, RecordingOutcome, ResolvedInput, ResolvedInputConfig,
    ResolvedOutput, SamplesBuffer, Source, Wsola, available_devices, available_hosts,
    available_input_channel_count, available_input_devices, available_input_hosts, decoder, input,
    open_output_stream, output, recording, resolve_input_stream_config,
    supported_input_sample_rates, supported_sample_rates, wav_sanitize,
};

use crate::selection::SelectionRange;

/// Convert a Wavecrate waveform selection into a reusable `reson` edit-fade range.
pub fn edit_fade_range_from_selection(range: Option<SelectionRange>) -> Option<EditFadeRange> {
    range.map(|range| {
        EditFadeRange::new(
            range.start(),
            range.end(),
            range.gain(),
            range.fade_in().map(|fade| {
                FadeParams::with_outer_gain(fade.length, fade.curve, fade.mute, fade.outer_gain)
            }),
            range.fade_out().map(|fade| {
                FadeParams::with_outer_gain(fade.length, fade.curve, fade.mute, fade.outer_gain)
            }),
        )
    })
}
