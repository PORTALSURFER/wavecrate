//! Shared backfill planning, execution, and persistence models.

use std::path::PathBuf;

/// Work item that requires decoding and embedding computation.
pub(super) struct EmbeddingWork {
    pub(super) content_hash: String,
    pub(super) absolute_path: PathBuf,
    pub(super) sample_ids: Vec<String>,
}

/// Result of a completed embedding computation before it is expanded per sample.
pub(super) struct EmbeddingComputation {
    pub(super) content_hash: String,
    pub(super) sample_ids: Vec<String>,
    pub(super) embedding: Vec<f32>,
    pub(super) created_at: i64,
}

/// Persistable embedding row for a specific sample id.
pub(super) struct EmbeddingResult {
    pub(super) sample_id: String,
    pub(super) content_hash: String,
    pub(super) embedding: Vec<f32>,
    pub(super) created_at: i64,
}

/// Embedding payload reused from caches or derived features.
#[derive(Clone)]
pub(super) struct EmbeddingData {
    pub(super) embedding: Vec<f32>,
    pub(super) created_at: i64,
}

/// Planned backfill work split between already-ready rows and decode work.
pub(super) struct BackfillPlan {
    pub(super) ready: Vec<EmbeddingResult>,
    pub(super) work: Vec<EmbeddingWork>,
}

pub(super) struct WorkEntry {
    pub(super) absolute_path: PathBuf,
    pub(super) sample_ids: Vec<String>,
}

impl WorkEntry {
    pub(super) fn new(absolute_path: PathBuf) -> Self {
        Self {
            absolute_path,
            sample_ids: Vec::new(),
        }
    }
}
