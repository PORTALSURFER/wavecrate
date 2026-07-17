pub(super) const WORKER_PROGRESS_OVERALL_KEY: &str = "bottom-status-progress-overall";
pub(super) const WORKER_PROGRESS_ACTIVE_KEY: &str = "bottom-status-progress-active";
pub(super) const WORKER_PROGRESS_ACTIVITY_HIGHLIGHT_KEY: &str =
    "bottom-status-progress-activity-highlight";
pub(super) const WORKER_PROGRESS_CURRENT_FILE_KEY: &str = "bottom-status-progress-current-file";
pub(super) const JOB_DETAILS_POPOVER_KEY: &str = "bottom-job-details-popover";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_progress_layer_keys_remain_distinct() {
        assert_ne!(WORKER_PROGRESS_OVERALL_KEY, WORKER_PROGRESS_ACTIVE_KEY);
        assert_ne!(
            WORKER_PROGRESS_ACTIVITY_HIGHLIGHT_KEY,
            WORKER_PROGRESS_CURRENT_FILE_KEY
        );
    }
}
