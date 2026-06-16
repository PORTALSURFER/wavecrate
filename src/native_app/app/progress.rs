use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct NormalizationProgress {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) completed: usize,
    pub(in crate::native_app) total: usize,
    pub(in crate::native_app) work_completed: usize,
    pub(in crate::native_app) work_total: usize,
    pub(in crate::native_app) queued: usize,
    pub(in crate::native_app) detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct NormalizationQueueItem {
    pub(in crate::native_app) paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct NormalizationResult {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) loaded_path: PathBuf,
    pub(in crate::native_app) normalizing_loaded: bool,
    pub(in crate::native_app) was_playing: bool,
    pub(in crate::native_app) restart_ratio: f32,
    pub(in crate::native_app) restart_span: Option<(f32, f32)>,
    pub(in crate::native_app) normalized: Vec<PathBuf>,
    pub(in crate::native_app) skipped: Vec<PathBuf>,
    pub(in crate::native_app) last_error: Option<String>,
}
