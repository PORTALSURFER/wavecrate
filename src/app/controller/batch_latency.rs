//! Test-only timing hooks for large browser metadata batches.

use std::cell::RefCell;
use std::time::Duration;

/// Targeted large-selection controller budget for prepare/dispatch phases.
pub(crate) const LARGE_BROWSER_BATCH_CONTROLLER_BUDGET: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BatchLatencyPhase {
    TagSidebarTargetResolution,
    TagSidebarOptimisticTag,
    BpmPreload,
    AutoRenamePrepare,
    AutoRenameDispatch,
    AutoRenameWorker,
    MetadataMutationQueue,
}

#[derive(Clone, Debug)]
pub(crate) struct BatchLatencySample {
    pub(crate) phase: BatchLatencyPhase,
    pub(crate) item_count: usize,
    pub(crate) elapsed: Duration,
    pub(crate) detail_count: usize,
    pub(crate) queue_depth_before: Option<usize>,
    pub(crate) queue_depth_after: Option<usize>,
}

impl BatchLatencySample {
    pub(crate) fn new(phase: BatchLatencyPhase, item_count: usize, elapsed: Duration) -> Self {
        Self {
            phase,
            item_count,
            elapsed,
            detail_count: 0,
            queue_depth_before: None,
            queue_depth_after: None,
        }
    }

    pub(crate) fn with_detail_count(mut self, detail_count: usize) -> Self {
        self.detail_count = detail_count;
        self
    }

    pub(crate) fn with_queue_depths(mut self, before: usize, after: usize) -> Self {
        self.queue_depth_before = Some(before);
        self.queue_depth_after = Some(after);
        self
    }
}

thread_local! {
    static SAMPLES: RefCell<Vec<BatchLatencySample>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn clear() {
    SAMPLES.with(|samples| samples.borrow_mut().clear());
}

pub(crate) fn record(sample: BatchLatencySample) {
    SAMPLES.with(|samples| samples.borrow_mut().push(sample));
}

pub(crate) fn snapshot() -> Vec<BatchLatencySample> {
    SAMPLES.with(|samples| samples.borrow().clone())
}
