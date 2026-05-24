use super::AnalysisProgressSnapshot;
use std::time::Instant;

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

#[derive(Clone, Debug)]
pub(super) struct ProgressTaskState {
    pub(super) modal: bool,
    pub(super) title: String,
    pub(super) detail: Option<String>,
    pub(super) completed: usize,
    pub(super) total: usize,
    pub(super) cancelable: bool,
    pub(super) cancel_requested: bool,
    pub(super) last_update_at: Option<Instant>,
    pub(super) last_progress_at: Option<Instant>,
    pub(super) analysis: Option<AnalysisProgressSnapshot>,
    pub(super) started_at: Instant,
}

impl ProgressTaskState {
    pub(super) fn new(
        modal: bool,
        title: impl Into<String>,
        total: usize,
        cancelable: bool,
    ) -> Self {
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

pub(super) fn task_priority(task: ProgressTaskKind) -> u8 {
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
