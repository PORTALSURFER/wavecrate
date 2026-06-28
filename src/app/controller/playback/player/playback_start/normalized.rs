use super::*;
use crate::app::controller::playback::audio_samples::decode_samples_from_bytes;

pub(super) fn normalized_audition_gain(controller: &AppController, start: f32, end: f32) -> f32 {
    if !controller.ui.waveform.normalized_audition_enabled {
        return 1.0;
    }
    let Some(peak) = normalized_audition_peak(controller, start, end) else {
        return 1.0;
    };
    crate::audio::normalized_gain_from_peak(peak)
}

/// Resolve the peak amplitude used for normalized audition over one playback span.
///
/// The retained decoded waveform is the fast path, but it is not guaranteed to
/// be present for every loaded sample. Plain transport playback should still
/// honor normalized audition in that state, so fall back to the loaded audio
/// bytes when the waveform decode cache is unavailable.
fn normalized_audition_peak(controller: &AppController, start: f32, end: f32) -> Option<f32> {
    if let Some(decoded) = controller.sample_view.waveform.decoded.as_ref() {
        return decoded.max_abs_in_span(start, end);
    }
    let loaded = controller.sample_view.wav.loaded_audio.as_ref()?;
    let decoded = decode_samples_from_bytes(&loaded.bytes).ok()?;
    crate::audio::peak_for_interleaved_span(
        &decoded.samples,
        decoded.channels.max(1) as usize,
        start,
        end,
    )
}
