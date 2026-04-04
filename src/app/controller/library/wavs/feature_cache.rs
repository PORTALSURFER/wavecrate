use super::*;
use crate::app::controller::controller_state::{
    AnalysisJobStatus, FeatureCache, FeatureCacheKey, FeatureStatus,
};
use crate::app::controller::jobs::{BrowserFeatureCacheRefreshResult, JobMessage};
use crate::app::controller::state::runtime::PendingBrowserFeatureCacheRefresh;
use rusqlite::params;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const ANALYSIS_JOB_TYPE: &str = "wav_metadata_v1";

/// Build one stable browser feature-cache key from ordered wav-entry paths.
pub(crate) fn feature_cache_key_for_paths(entry_paths: &[PathBuf]) -> FeatureCacheKey {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for path in entry_paths {
        normalize_relative_key(&path.to_string_lossy()).hash(&mut hasher);
    }
    FeatureCacheKey {
        entries_len: entry_paths.len(),
        entries_hash: hasher.finish(),
    }
}

/// Load browser feature metadata aligned to one ordered wav-entry path snapshot.
pub(crate) fn build_feature_cache_for_paths(
    source_id: &SourceId,
    source_root: &Path,
    entry_paths: &[PathBuf],
    fallback_rows: &[Option<FeatureStatus>],
) -> Result<FeatureCache, String> {
    let key = feature_cache_key_for_paths(entry_paths);
    let conn = analysis_jobs::open_source_db(source_root)?;
    let mut rows = vec![None; key.entries_len];

    let prefix = format!("{}::", source_id.as_str());
    let prefix_end = format!("{prefix}\u{10FFFF}");

    let mut sample_map: HashMap<String, FeatureStatus> = HashMap::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT s.sample_id,
                        s.duration_seconds,
                        s.sr_used,
                        s.long_sample_mark,
                        CASE WHEN f.sample_id IS NULL THEN 0 ELSE 1 END AS has_features_v1,
                        CASE WHEN e.sample_id IS NULL THEN 0 ELSE 1 END AS has_embedding,
                        j.status
                 FROM samples s
                 LEFT JOIN features f ON f.sample_id = s.sample_id AND f.feat_version = 1
                 LEFT JOIN embeddings e ON e.sample_id = s.sample_id AND e.model_id = ?2
                 LEFT JOIN analysis_jobs j ON j.sample_id = s.sample_id AND j.job_type = ?1
                 WHERE s.sample_id >= ?3 AND s.sample_id < ?4",
            )
            .map_err(|err| format!("Prepare feature cache query failed: {err}"))?;
        let mut query_rows = stmt
            .query(params![
                ANALYSIS_JOB_TYPE,
                crate::analysis::similarity::SIMILARITY_MODEL_ID,
                prefix,
                prefix_end
            ])
            .map_err(|err| format!("Query feature cache failed: {err}"))?;
        while let Some(row) = query_rows
            .next()
            .map_err(|err| format!("Query feature cache failed: {err}"))?
        {
            let sample_id: String = row.get::<_, String>(0).map_err(|err| err.to_string())?;
            let duration_seconds: Option<f64> = row
                .get::<_, Option<f64>>(1)
                .map_err(|err| err.to_string())?;
            let sr_used: Option<i64> = row
                .get::<_, Option<i64>>(2)
                .map_err(|err| err.to_string())?;
            let long_sample_mark: Option<i64> = row
                .get::<_, Option<i64>>(3)
                .map_err(|err| err.to_string())?;
            let has_features_v1: i64 = row.get::<_, i64>(4).map_err(|err| err.to_string())?;
            let has_embedding: i64 = row.get::<_, i64>(5).map_err(|err| err.to_string())?;
            let status: Option<String> = row
                .get::<_, Option<String>>(6)
                .map_err(|err| err.to_string())?;
            let analysis_status = status.as_deref().and_then(parse_job_status);
            let Some(relative_path) = sample_id.split_once("::").map(|(_, path)| path) else {
                continue;
            };
            sample_map.insert(
                normalize_relative_key(relative_path),
                FeatureStatus {
                    has_features_v1: has_features_v1 != 0,
                    has_embedding: has_embedding != 0,
                    duration_seconds: duration_seconds.map(|seconds| seconds as f32),
                    sr_used,
                    long_sample_mark: long_sample_mark.map(|value| value != 0),
                    analysis_status,
                },
            );
        }
    }

    for (index, row_slot) in rows.iter_mut().enumerate() {
        let key = entry_paths
            .get(index)
            .map(|path| normalize_relative_key(&path.to_string_lossy()))
            .unwrap_or_default();
        let mut status = sample_map.remove(&key).unwrap_or(FeatureStatus {
            has_features_v1: false,
            has_embedding: false,
            duration_seconds: None,
            sr_used: None,
            long_sample_mark: None,
            analysis_status: None,
        });
        if status.duration_seconds.is_none()
            && let Some(fallback) = fallback_rows.get(index).and_then(|row| row.as_ref())
            && let Some(duration) = fallback
                .duration_seconds
                .filter(|value| value.is_finite() && *value > 0.0)
        {
            status.duration_seconds = Some(duration);
            if status.sr_used.is_none() {
                status.sr_used = fallback.sr_used;
            }
        }
        if status.long_sample_mark.is_none()
            && let Some(fallback) = fallback_rows.get(index).and_then(|row| row.as_ref())
        {
            status.long_sample_mark = fallback.long_sample_mark;
        }
        *row_slot = Some(status);
    }

    Ok(FeatureCache {
        key,
        rows: rows.into(),
    })
}

impl AppController {
    /// Queue one async browser feature-cache refresh when the current snapshot is missing or stale.
    pub(crate) fn queue_feature_cache_refresh_for_browser(&mut self) {
        self.queue_feature_cache_refresh_for_browser_with_force(false);
    }

    /// Queue one async browser feature-cache refresh even when stale-safe rows still exist.
    pub(crate) fn force_feature_cache_refresh_for_browser(&mut self) {
        self.queue_feature_cache_refresh_for_browser_with_force(true);
    }

    /// Return cached browser feature metadata for one entry when the selected-source cache is safe.
    pub(crate) fn cached_feature_status_for_entry(
        &self,
        entry_index: usize,
    ) -> Option<&FeatureStatus> {
        let source_id = self.selection_state.ctx.selected_source.as_ref()?;
        self.ui_cache
            .browser
            .features
            .get(source_id)
            .filter(|cache| cache.rows.len() == self.wav_entries_len())
            .and_then(|cache| cache.rows.get(entry_index))
            .and_then(|row| row.as_ref())
    }

    /// Apply one completed async browser feature-cache refresh.
    pub(crate) fn handle_feature_cache_refreshed_message(
        &mut self,
        message: BrowserFeatureCacheRefreshResult,
    ) {
        if !self
            .runtime
            .pending_browser_feature_cache_refresh
            .as_ref()
            .is_some_and(|pending| {
                pending.request_id == message.request_id
                    && pending.source_id == message.source_id
                    && pending.key == message.key
            })
        {
            return;
        }
        self.runtime.pending_browser_feature_cache_refresh = None;
        if self.current_browser_feature_cache_key() != Some(message.key)
            || self.selected_source_id().as_ref() != Some(&message.source_id)
        {
            return;
        }
        match message.result {
            Ok(cache) => self.install_feature_cache_snapshot(message.source_id, cache),
            Err(err) => {
                self.ui_cache
                    .browser
                    .features
                    .entry(message.source_id)
                    .or_insert_with(|| FeatureCache::empty(message.key));
                self.set_status(
                    format!("Failed to load analysis metadata: {err}"),
                    crate::app::controller::StatusTone::Error,
                );
            }
        }
    }

    /// Patch cached duration metadata for a sample if the feature cache is live.
    pub(crate) fn update_cached_duration_for_path(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
        duration_seconds: f32,
        sample_rate: u32,
    ) {
        if duration_seconds.is_finite() && duration_seconds > 0.0 {
            self.ui_cache
                .browser
                .durations
                .entry(source_id.clone())
                .or_default()
                .insert(relative_path.to_path_buf(), duration_seconds);
        }
        let Some(cache) = self.ui_cache.browser.features.get_mut(source_id) else {
            return;
        };
        let normalized = relative_path.to_string_lossy().replace('\\', "/");
        let Some(index) = self.wav_entries.lookup.get(Path::new(&normalized)).copied() else {
            return;
        };
        let rows = Arc::make_mut(&mut cache.rows);
        if let Some(slot) = rows.get_mut(index) {
            let status = slot.get_or_insert(FeatureStatus {
                has_features_v1: false,
                has_embedding: false,
                duration_seconds: None,
                sr_used: None,
                long_sample_mark: None,
                analysis_status: None,
            });
            status.duration_seconds = Some(duration_seconds);
            status.sr_used = Some(sample_rate as i64);
        }
    }

    /// Install one browser feature-cache snapshot and dirty row metadata for repaint.
    pub(crate) fn install_feature_cache_snapshot(
        &mut self,
        source_id: SourceId,
        cache: FeatureCache,
    ) {
        self.ui_cache
            .browser
            .features
            .insert(source_id.clone(), cache);
        if self.selection_state.ctx.selected_source.as_ref() == Some(&source_id) {
            self.mark_browser_row_metadata_projection_revision_dirty();
        }
    }

    fn queue_feature_cache_refresh_for_browser_with_force(&mut self, force: bool) {
        let Some(source) = self.current_source() else {
            return;
        };
        let Some(snapshot) = self.current_browser_feature_cache_snapshot() else {
            return;
        };
        let key = snapshot.key;
        if !force
            && self
                .ui_cache
                .browser
                .features
                .get(&source.id)
                .is_some_and(|cache| cache.key == key)
        {
            return;
        }
        if self
            .runtime
            .pending_browser_feature_cache_refresh
            .as_ref()
            .is_some_and(|pending| pending.source_id == source.id && pending.key == key)
        {
            return;
        }
        let fallback_rows = self
            .ui_cache
            .browser
            .features
            .get(&source.id)
            .filter(|cache| cache.key == key)
            .map(|cache| cache.rows.clone())
            .unwrap_or_else(|| Arc::from([]));
        let request_id = self.runtime.jobs.next_feature_cache_request_id();
        self.runtime.pending_browser_feature_cache_refresh =
            Some(PendingBrowserFeatureCacheRefresh {
                request_id,
                source_id: source.id.clone(),
                key,
            });
        let source_id = source.id;
        let source_root = source.root;
        self.runtime.jobs.spawn_one_shot_job(
            true,
            move || BrowserFeatureCacheRefreshResult {
                request_id,
                source_id: source_id.clone(),
                key,
                result: build_feature_cache_for_paths(
                    &source_id,
                    &source_root,
                    snapshot.entry_paths.as_ref(),
                    fallback_rows.as_ref(),
                ),
            },
            JobMessage::BrowserFeatureCacheRefreshed,
        );
    }

    fn current_browser_feature_cache_key(&mut self) -> Option<FeatureCacheKey> {
        self.current_browser_feature_cache_snapshot()
            .map(|snapshot| snapshot.key)
    }
}

impl AppController {
    /// Update the cached long-sample marker for a sample if the feature cache is live.
    pub(crate) fn update_cached_long_mark_for_path(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
        long_sample_mark: bool,
    ) {
        let Some(cache) = self.ui_cache.browser.features.get_mut(source_id) else {
            return;
        };
        let normalized = relative_path.to_string_lossy().replace('\\', "/");
        let Some(index) = self.wav_entries.lookup.get(Path::new(&normalized)).copied() else {
            return;
        };
        let rows = Arc::make_mut(&mut cache.rows);
        if let Some(slot) = rows.get_mut(index) {
            let status = slot.get_or_insert(FeatureStatus {
                has_features_v1: false,
                has_embedding: false,
                duration_seconds: None,
                sr_used: None,
                long_sample_mark: None,
                analysis_status: None,
            });
            status.long_sample_mark = Some(long_sample_mark);
        }
    }
}

fn parse_job_status(status: &str) -> Option<AnalysisJobStatus> {
    match status {
        "pending" => Some(AnalysisJobStatus::Pending),
        "running" => Some(AnalysisJobStatus::Running),
        "done" => Some(AnalysisJobStatus::Done),
        "failed" => Some(AnalysisJobStatus::Failed),
        "canceled" => Some(AnalysisJobStatus::Canceled),
        _ => None,
    }
}

fn normalize_relative_key(relative_path: &str) -> String {
    relative_path
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests;
