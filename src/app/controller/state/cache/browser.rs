//! Browser-list caches for labels, feature metadata, and staged search output.

use crate::app::controller::library::wavs;
use crate::sample_sources::{SourceId, db::SourceTag};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) struct BrowserCacheState {
    pub(crate) labels: HashMap<SourceId, BrowserLabelCacheEntry>,
    pub(crate) analysis_failures: HashMap<SourceId, HashMap<PathBuf, String>>,
    pub(crate) analysis_failures_pending: HashSet<SourceId>,
    /// Retained staged browser pipeline outputs keyed by revision fingerprints.
    pub(crate) pipeline: wavs::BrowserPipelineCache,
    pub(crate) search: wavs::BrowserSearchCache,
    pub(crate) features: HashMap<SourceId, FeatureCache>,
    pub(crate) bpm_values: HashMap<SourceId, HashMap<PathBuf, Option<f32>>>,
    pub(crate) durations: HashMap<SourceId, HashMap<PathBuf, f32>>,
    pub(crate) normal_tags: HashMap<SourceId, HashMap<PathBuf, Vec<SourceTag>>>,
}

impl BrowserCacheState {
    pub(crate) fn new() -> Self {
        Self {
            labels: HashMap::new(),
            analysis_failures: HashMap::new(),
            analysis_failures_pending: HashSet::new(),
            pipeline: wavs::BrowserPipelineCache::default(),
            search: wavs::BrowserSearchCache::default(),
            features: HashMap::new(),
            bpm_values: HashMap::new(),
            durations: HashMap::new(),
            normal_tags: HashMap::new(),
        }
    }
}

/// Retained browser labels aligned to one ordered wav-entry snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowserLabelCacheEntry {
    /// Stable hash of the ordered relative-path list backing the cached labels.
    pub(crate) path_fingerprint: u64,
    /// Display labels aligned to absolute wav-entry indices.
    pub(crate) labels: Vec<String>,
}

impl BrowserLabelCacheEntry {
    /// Build an empty label cache aligned to one ordered-path snapshot.
    pub(crate) fn new(path_fingerprint: u64, entries_len: usize) -> Self {
        Self {
            path_fingerprint,
            labels: vec![String::new(); entries_len],
        }
    }
}

/// Stable snapshot key for browser feature-cache rows aligned to the current wav list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FeatureCacheKey {
    /// Number of wav entries the cache rows are aligned to.
    pub(crate) entries_len: usize,
    /// Stable hash of the ordered relative-path list backing the cache rows.
    pub(crate) entries_hash: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnalysisJobStatus {
    Pending,
    Running,
    Done,
    Failed,
    Canceled,
}

#[derive(Clone, Debug)]
pub(crate) struct FeatureStatus {
    pub(crate) has_features_v1: bool,
    pub(crate) has_embedding: bool,
    pub(crate) duration_seconds: Option<f32>,
    pub(crate) sr_used: Option<i64>,
    pub(crate) long_sample_mark: Option<bool>,
    pub(crate) analysis_status: Option<AnalysisJobStatus>,
}

/// Cached per-entry browser feature metadata aligned to one wav-entry snapshot.
#[derive(Clone, Debug)]
pub(crate) struct FeatureCache {
    pub(crate) key: FeatureCacheKey,
    pub(crate) rows: Arc<[Option<FeatureStatus>]>,
}

impl FeatureCache {
    /// Build an empty placeholder cache for one wav-entry snapshot key.
    pub(crate) fn empty(key: FeatureCacheKey) -> Self {
        Self {
            rows: vec![None; key.entries_len].into(),
            key,
        }
    }
}
