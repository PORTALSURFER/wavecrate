use radiant::prelude as ui;

const COOPERATIVE_YIELD_INTERVAL: usize = 4_096;
const COOPERATIVE_SLEEP_INTERVAL: usize = 262_144;
const PROGRESS_REPORT_INTERVAL: usize = 16_384;

pub(in crate::gui_app::waveform) fn report_phase_progress_throttled(
    start: f32,
    end: f32,
    completed: usize,
    total: usize,
    progress: &impl Fn(f32),
) {
    cooperate_with_ui(completed);
    if completed == total || completed.is_multiple_of(PROGRESS_REPORT_INTERVAL) {
        report_phase_progress(start, end, completed, total, progress);
    }
}

pub(in crate::gui_app::waveform) fn cooperate_with_ui(completed: usize) {
    if completed == 0 {
        return;
    }
    if completed.is_multiple_of(COOPERATIVE_SLEEP_INTERVAL) {
        std::thread::sleep(std::time::Duration::from_millis(1));
    } else if completed.is_multiple_of(COOPERATIVE_YIELD_INTERVAL) {
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
    ui::ProgressPhase::new(start, end).report(completed, total, progress);
}
