/// Modal progress indicator for slow tasks.
/// Identifies the long-running task responsible for updating the progress overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProgressTaskKind {
    /// Moving files to trash.
    TrashMove,
    /// Loading a waveform from disk.
    WavLoad,
    /// Scanning a sample source.
    Scan,
    /// Running background analysis jobs.
    Analysis,
    /// Normalizing audio samples.
    Normalization,
    /// Exporting waveform slice batches in the background.
    SelectionExport,
    /// Copying, moving, or restoring files in the background.
    FileOps,
    /// Filtering or rebuilding browser results in the background.
    Search,
}

use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone, Debug)]
struct ProgressTaskState {
    modal: bool,
    title: String,
    detail: Option<String>,
    completed: usize,
    total: usize,
    cancelable: bool,
    cancel_requested: bool,
    last_update_at: Option<Instant>,
    last_progress_at: Option<Instant>,
    analysis: Option<AnalysisProgressSnapshot>,
    started_at: Instant,
}

impl ProgressTaskState {
    fn new(modal: bool, title: impl Into<String>, total: usize, cancelable: bool) -> Self {
        let now = Instant::now();
        Self {
            modal,
            title: title.into(),
            detail: None,
            completed: 0,
            total,
            cancelable,
            cancel_requested: false,
            last_update_at: Some(now),
            last_progress_at: Some(now),
            analysis: None,
            started_at: now,
        }
    }
}

/// UI state for the progress overlay and its counters.
#[derive(Clone, Debug, Default)]
pub struct ProgressOverlayState {
    /// Whether the overlay is visible.
    pub visible: bool,
    /// When true, the modal overlay is rendered (otherwise progress is status-bar only).
    pub modal: bool,
    /// The task currently driving the progress overlay (when visible).
    pub task: Option<ProgressTaskKind>,
    /// Title text for the overlay.
    pub title: String,
    /// Optional detail text for the overlay.
    pub detail: Option<String>,
    /// Completed steps.
    pub completed: usize,
    /// Total steps.
    pub total: usize,
    /// Whether cancel is allowed.
    pub cancelable: bool,
    /// Whether the user requested cancellation.
    pub cancel_requested: bool,
    /// Last time the overlay was updated.
    pub last_update_at: Option<Instant>,
    /// Last time progress advanced.
    pub last_progress_at: Option<Instant>,
    /// Optional analysis progress snapshot.
    pub analysis: Option<AnalysisProgressSnapshot>,
    /// Active footer-progress contenders across background task families.
    task_states: HashMap<ProgressTaskKind, ProgressTaskState>,
}

/// Snapshot of analysis progress for display.
#[derive(Clone, Debug)]
pub struct AnalysisProgressSnapshot {
    /// Number of pending jobs.
    pub pending: usize,
    /// Number of running jobs.
    pub running: usize,
    /// Number of failed jobs.
    pub failed: usize,
    /// Completed samples count.
    pub samples_completed: usize,
    /// Total samples to process.
    pub samples_total: usize,
    /// Snapshot of running jobs.
    pub running_jobs: Vec<RunningJobSnapshot>,
    /// Staleness threshold in seconds.
    pub stale_after_secs: Option<i64>,
}

/// Summary of a running job heartbeat for display.
#[derive(Clone, Debug)]
pub struct RunningJobSnapshot {
    /// Human-readable job label.
    pub label: String,
    /// Last heartbeat timestamp, epoch seconds.
    pub last_heartbeat_at: Option<i64>,
    /// Whether the job appears stalled.
    pub possibly_stalled: bool,
}

impl RunningJobSnapshot {
    /// Build a snapshot and mark it stalled when the heartbeat is stale.
    pub fn from_heartbeat(
        label: String,
        last_heartbeat_at: Option<i64>,
        stale_after_secs: Option<i64>,
        now_epoch: Option<i64>,
    ) -> Self {
        let possibly_stalled = match (last_heartbeat_at, stale_after_secs, now_epoch) {
            (Some(heartbeat), Some(stale_after), Some(now)) => {
                now.saturating_sub(heartbeat) >= stale_after
            }
            _ => false,
        };
        Self {
            label,
            last_heartbeat_at,
            possibly_stalled,
        }
    }
}

impl ProgressOverlayState {
    /// Create and show a progress overlay with the provided title and total step count.
    pub fn new(
        task: ProgressTaskKind,
        title: impl Into<String>,
        total: usize,
        cancelable: bool,
    ) -> Self {
        let mut state = Self::default();
        state.show_task(task, true, title, total, cancelable);
        state
    }

    /// Reset the overlay back to its default (hidden) state.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Return whether a task currently participates in footer arbitration.
    pub fn has_task(&self, task: ProgressTaskKind) -> bool {
        self.task_states.contains_key(&task)
    }

    /// Return the current total for one task when it participates in footer arbitration.
    pub fn task_total(&self, task: ProgressTaskKind) -> Option<usize> {
        self.task_states.get(&task).map(|slot| slot.total)
    }

    /// Return the current completed count for one task when it participates in footer arbitration.
    pub fn task_completed(&self, task: ProgressTaskKind) -> Option<usize> {
        self.task_states.get(&task).map(|slot| slot.completed)
    }

    /// Return the current detail label for one task when it participates in footer arbitration.
    pub fn task_detail(&self, task: ProgressTaskKind) -> Option<&str> {
        self.task_states
            .get(&task)
            .and_then(|slot| slot.detail.as_deref())
    }

    /// Show or refresh one progress task and then recompute the visible footer owner.
    pub fn show_task(
        &mut self,
        task: ProgressTaskKind,
        modal: bool,
        title: impl Into<String>,
        total: usize,
        cancelable: bool,
    ) {
        let title = title.into();
        let slot = self
            .task_states
            .entry(task)
            .or_insert_with(|| ProgressTaskState::new(modal, title.clone(), total, cancelable));
        slot.modal = modal;
        slot.title = title;
        slot.total = total;
        slot.cancelable = cancelable;
        slot.last_update_at = Some(Instant::now());
        self.recompute_visible_task();
    }

    /// Remove one task from footer arbitration and recompute the visible footer owner.
    pub fn clear_task(&mut self, task: ProgressTaskKind) {
        self.task_states.remove(&task);
        self.recompute_visible_task();
    }

    /// Update one task's detail text and refresh the timestamp.
    pub fn set_task_detail(&mut self, task: ProgressTaskKind, detail: Option<String>) {
        if let Some(slot) = self.task_states.get_mut(&task) {
            slot.detail = detail;
            slot.last_update_at = Some(Instant::now());
            self.recompute_visible_task();
        }
    }

    /// Update one task's title text and refresh the timestamp.
    pub fn set_task_title(&mut self, task: ProgressTaskKind, title: impl Into<String>) {
        if let Some(slot) = self.task_states.get_mut(&task) {
            slot.title = title.into();
            slot.last_update_at = Some(Instant::now());
            self.recompute_visible_task();
        }
    }

    /// Update one task's total/completed counts and refresh timestamps.
    pub fn set_task_counts(&mut self, task: ProgressTaskKind, total: usize, completed: usize) {
        if let Some(slot) = self.task_states.get_mut(&task) {
            if slot.total != total || slot.completed != completed {
                slot.last_progress_at = Some(Instant::now());
            }
            slot.total = total;
            slot.completed = completed;
            slot.last_update_at = Some(Instant::now());
            self.recompute_visible_task();
        }
    }

    /// Update one task's cancelability and refresh the timestamp.
    pub fn set_task_cancelable(&mut self, task: ProgressTaskKind, cancelable: bool) {
        if let Some(slot) = self.task_states.get_mut(&task) {
            slot.cancelable = cancelable;
            slot.last_update_at = Some(Instant::now());
            self.recompute_visible_task();
        }
    }

    /// Update one task's analysis snapshot.
    pub fn set_task_analysis_snapshot(
        &mut self,
        task: ProgressTaskKind,
        snapshot: Option<AnalysisProgressSnapshot>,
    ) {
        if let Some(slot) = self.task_states.get_mut(&task) {
            slot.analysis = snapshot;
            slot.last_update_at = Some(Instant::now());
            self.recompute_visible_task();
        }
    }

    /// Request cancellation for one task and recompute the visible footer owner.
    pub fn request_task_cancel(&mut self, task: ProgressTaskKind) {
        if let Some(slot) = self.task_states.get_mut(&task)
            && slot.cancelable
        {
            slot.cancel_requested = true;
            slot.last_update_at = Some(Instant::now());
            self.recompute_visible_task();
        }
    }

    /// Update the detail text and refresh the timestamp.
    pub fn set_detail(&mut self, detail: Option<String>) {
        if let Some(task) = self.task {
            self.set_task_detail(task, detail);
        }
    }

    /// Update the title text and refresh the timestamp.
    pub fn set_title(&mut self, title: impl Into<String>) {
        if let Some(task) = self.task {
            self.set_task_title(task, title);
        }
    }

    /// Update total/completed counts and refresh timestamps.
    pub fn set_counts(&mut self, total: usize, completed: usize) {
        if let Some(task) = self.task {
            self.set_task_counts(task, total, completed);
        }
    }

    /// Update the analysis progress snapshot.
    pub fn set_analysis_snapshot(&mut self, snapshot: Option<AnalysisProgressSnapshot>) {
        if let Some(task) = self.task {
            self.set_task_analysis_snapshot(task, snapshot);
        }
    }

    /// Return completion in the range `[0.0, 1.0]`.
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.completed as f32 / self.total as f32).clamp(0.0, 1.0)
        }
    }

    fn recompute_visible_task(&mut self) {
        let previous = self.task;
        let selected = self.select_visible_task(previous);
        let Some(task) = selected else {
            self.visible = false;
            self.modal = false;
            self.task = None;
            self.title.clear();
            self.detail = None;
            self.completed = 0;
            self.total = 0;
            self.cancelable = false;
            self.cancel_requested = false;
            self.last_update_at = None;
            self.last_progress_at = None;
            self.analysis = None;
            return;
        };
        let slot = self
            .task_states
            .get(&task)
            .expect("selected progress task should exist");
        self.visible = true;
        self.modal = slot.modal;
        self.task = Some(task);
        self.title = slot.title.clone();
        self.detail = slot.detail.clone();
        self.completed = slot.completed;
        self.total = slot.total;
        self.cancelable = slot.cancelable;
        self.cancel_requested = slot.cancel_requested;
        self.last_update_at = slot.last_update_at;
        self.last_progress_at = slot.last_progress_at;
        self.analysis = slot.analysis.clone();
    }

    fn select_visible_task(&self, previous: Option<ProgressTaskKind>) -> Option<ProgressTaskKind> {
        let mut best: Option<(ProgressTaskKind, &ProgressTaskState)> = None;
        for (&task, state) in &self.task_states {
            match best {
                None => best = Some((task, state)),
                Some((best_task, best_state)) => {
                    let best_priority = task_priority(best_task);
                    let candidate_priority = task_priority(task);
                    if candidate_priority > best_priority
                        || (candidate_priority == best_priority
                            && Some(task) == previous
                            && Some(best_task) != previous)
                        || (candidate_priority == best_priority
                            && Some(best_task) != previous
                            && state.started_at < best_state.started_at)
                    {
                        best = Some((task, state));
                    }
                }
            }
        }
        best.map(|(task, _)| task)
    }
}

fn task_priority(task: ProgressTaskKind) -> u8 {
    match task {
        ProgressTaskKind::TrashMove => 100,
        ProgressTaskKind::Scan => 90,
        ProgressTaskKind::Analysis => 80,
        ProgressTaskKind::FileOps => 70,
        ProgressTaskKind::Normalization => 60,
        ProgressTaskKind::SelectionExport => 50,
        ProgressTaskKind::WavLoad => 20,
        ProgressTaskKind::Search => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::{ProgressOverlayState, ProgressTaskKind, RunningJobSnapshot};

    #[test]
    fn progress_fraction_handles_zero_total() {
        let progress = ProgressOverlayState::new(ProgressTaskKind::TrashMove, "Task", 0, false);
        assert_eq!(progress.fraction(), 0.0);
    }

    #[test]
    fn progress_reset_clears_visibility() {
        let mut progress = ProgressOverlayState::new(ProgressTaskKind::TrashMove, "Task", 2, true);
        progress.completed = 3;
        assert!(progress.fraction() <= 1.0);
        progress.reset();
        assert!(!progress.visible);
        assert_eq!(progress.task, None);
        assert_eq!(progress.completed, 0);
        assert_eq!(progress.total, 0);
    }

    #[test]
    fn running_job_marks_stale_heartbeat() {
        let snapshot =
            RunningJobSnapshot::from_heartbeat("job".to_string(), Some(10), Some(5), Some(20));
        assert!(snapshot.possibly_stalled);

        let snapshot =
            RunningJobSnapshot::from_heartbeat("job".to_string(), Some(18), Some(5), Some(20));
        assert!(!snapshot.possibly_stalled);
    }

    #[test]
    fn higher_priority_background_task_wins_footer_lane() {
        let mut progress = ProgressOverlayState::default();
        progress.show_task(ProgressTaskKind::Analysis, false, "Analyzing", 10, true);
        progress.show_task(
            ProgressTaskKind::WavLoad,
            false,
            "Loading samples",
            0,
            false,
        );

        assert_eq!(progress.task, Some(ProgressTaskKind::Analysis));
        assert_eq!(progress.title, "Analyzing");
    }

    #[test]
    fn clearing_visible_task_reveals_next_contender() {
        let mut progress = ProgressOverlayState::default();
        progress.show_task(ProgressTaskKind::Analysis, false, "Analyzing", 10, true);
        progress.show_task(
            ProgressTaskKind::WavLoad,
            false,
            "Loading samples",
            0,
            false,
        );

        progress.clear_task(ProgressTaskKind::Analysis);

        assert_eq!(progress.task, Some(ProgressTaskKind::WavLoad));
        assert_eq!(progress.title, "Loading samples");
    }
}
