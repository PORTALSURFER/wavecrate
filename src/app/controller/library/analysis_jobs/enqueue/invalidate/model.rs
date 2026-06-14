use crate::app::controller::library::analysis_jobs::db;

pub(crate) struct BackfillPlan {
    pub(crate) sample_metadata: Vec<db::SampleMetadata>,
    pub(crate) jobs: Vec<(String, String)>,
    pub(crate) invalidate: Vec<String>,
    pub(crate) failed_requeued: usize,
}

/// Sample/job changes that should be applied after deciding which items need backfill.
pub(crate) struct BackfillUpdates {
    pub(crate) sample_metadata: Vec<db::SampleMetadata>,
    pub(crate) jobs: Vec<(String, String)>,
    pub(crate) invalidate: Vec<String>,
}

/// Temporary split of queued jobs from invalidation-only updates.
pub(super) struct QueuedBackfillJobs {
    pub(super) sample_metadata: Vec<db::SampleMetadata>,
    pub(super) jobs: Vec<(String, String)>,
}
