pub(in crate::gui_app::waveform) fn report_phase_progress_throttled(
    start: f32,
    end: f32,
    completed: usize,
    total: usize,
    progress: &impl Fn(f32),
) {
    if completed == total || completed % 16_384 == 0 {
        report_phase_progress(start, end, completed, total, progress);
    }
}

pub(in crate::gui_app::waveform) fn report_phase_progress(
    start: f32,
    end: f32,
    completed: usize,
    total: usize,
    progress: &impl Fn(f32),
) {
    if total == 0 {
        return;
    }
    let ratio = completed as f32 / total as f32;
    progress(start + (end - start) * ratio.clamp(0.0, 1.0));
}
