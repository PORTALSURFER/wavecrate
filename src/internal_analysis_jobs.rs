//! Hidden bridge for native-runtime readiness stage execution.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use rusqlite::Connection;

use crate::app::controller::library::analysis_jobs;

pub use crate::app::controller::library::analysis_jobs::ReadinessStageError;

/// Produce current feature artifacts for one readiness-owned file target.
pub fn run_readiness_feature_stage(
    conn: &mut Connection,
    source_root: &Path,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<bool, ReadinessStageError> {
    analysis_jobs::run_readiness_feature_stage(
        conn,
        source_root,
        source_id,
        relative_path,
        content_hash,
        analysis_version,
        cancel,
    )
}

/// Produce current embedding, aspect, and ANN artifacts for one readiness-owned file target.
pub fn run_readiness_embedding_stage(
    conn: &mut Connection,
    source_root: &Path,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    analysis_jobs::run_readiness_embedding_stage(
        conn,
        source_root,
        source_id,
        relative_path,
        content_hash,
        analysis_version,
        cancel,
    )
}
