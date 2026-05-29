pub(in crate::gui_app::waveform) fn downmix_to_mono_with_progress_and_cancel(
    samples: &[f32],
    channels: usize,
    frames: usize,
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<f32>, String> {
    let channels = channels.max(1);
    let mut mono = Vec::with_capacity(frames);
    for frame in 0..frames {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        let sample_start = frame * channels;
        let mut peak = 0.0_f32;
        for sample in samples[sample_start..sample_start + channels]
            .iter()
            .copied()
        {
            if sample.abs() > peak.abs() {
                peak = sample;
            }
        }
        mono.push(peak.clamp(-1.0, 1.0));
        super::report_phase_progress_throttled(start, end, frame + 1, frames, progress);
    }
    progress(end);
    Ok(mono)
}

#[cfg(test)]
pub(in crate::gui_app::waveform) fn downmix_to_mono(
    samples: &[f32],
    channels: usize,
    frames: usize,
) -> Vec<f32> {
    downmix_to_mono_with_progress_and_cancel(samples, channels, frames, 0.0, 1.0, &|_| {}, &|| {
        false
    })
    .expect("non-cancellable downmix cannot be cancelled")
}
