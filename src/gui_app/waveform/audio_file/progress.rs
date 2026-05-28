const COOPERATIVE_YIELD_INTERVAL: usize = 4_096;
const COOPERATIVE_SLEEP_INTERVAL: usize = 262_144;

pub(in crate::gui_app::waveform) fn report_phase_progress_throttled(
    start: f32,
    end: f32,
    completed: usize,
    total: usize,
    progress: &impl Fn(f32),
) {
    cooperate_with_ui(completed);
    if completed == total || completed % 16_384 == 0 {
        report_phase_progress(start, end, completed, total, progress);
    }
}

pub(in crate::gui_app::waveform) fn cooperate_with_ui(completed: usize) {
    if completed == 0 {
        return;
    }
    if completed % COOPERATIVE_SLEEP_INTERVAL == 0 {
        std::thread::sleep(std::time::Duration::from_millis(1));
    } else if completed % COOPERATIVE_YIELD_INTERVAL == 0 {
        std::thread::yield_now();
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
