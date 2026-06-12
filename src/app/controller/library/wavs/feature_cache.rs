use super::*;
use crate::app::controller::controller_state::{
    AnalysisJobStatus, FeatureCache, FeatureCacheKey, FeatureStatus,
};
use crate::app::controller::jobs::{BrowserFeatureCacheRefreshResult, JobMessage};
use crate::app::controller::state::runtime::PendingBrowserFeatureCacheRefresh;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod repository;

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
    let source_rows = repository::FeatureCacheRepository::new(&conn)
        .load_source_rows(source_id)
        .map_err(|err| err.to_string())?;
    let rows = repository::align_rows_to_entries(entry_paths, fallback_rows, source_rows);
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
        &mut self,
        entry_index: usize,
    ) -> Option<&FeatureStatus> {
        let source_id = self.selection_state.ctx.selected_source.clone()?;
        let current_key = self.current_browser_feature_cache_key()?;
        self.ui_cache
            .browser
            .features
            .get(&source_id)
            .filter(|cache| cache.key == current_key)
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
            .browser
            .pending_feature_cache_refresh
            .as_ref()
            .is_some_and(|pending| {
                pending.request_id == message.request_id
                    && pending.source_id == message.source_id
                    && pending.key == message.key
            })
        {
            return;
        }
        self.runtime.browser.pending_feature_cache_refresh = None;
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
            .browser
            .pending_feature_cache_refresh
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
        self.runtime.browser.pending_feature_cache_refresh =
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

    #[cfg(test)]
    /// Clear one pending browser feature-cache refresh for focused regression tests.
    pub(crate) fn clear_pending_browser_feature_cache_refresh_for_tests(&mut self) {
        self.runtime.browser.pending_feature_cache_refresh = None;
    }

    #[cfg(test)]
    /// Return whether one browser feature-cache refresh is currently queued.
    pub(crate) fn has_pending_browser_feature_cache_refresh_for_tests(&self) -> bool {
        self.runtime.browser.pending_feature_cache_refresh.is_some()
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
