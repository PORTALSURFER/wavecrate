use crate::app::controller::FeatureStatus;
use crate::sample_sources::SourceId;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use super::{normalize_relative_key, parse_job_status};

const ANALYSIS_JOB_TYPE: &str = "readiness_analysis_features_v1";

#[derive(Debug)]
pub(super) enum FeatureCacheRepositoryError {
    Prepare(rusqlite::Error),
    Query(rusqlite::Error),
    Row(rusqlite::Error),
}

impl fmt::Display for FeatureCacheRepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prepare(err) => write!(formatter, "Prepare feature cache query failed: {err}"),
            Self::Query(err) => write!(formatter, "Query feature cache failed: {err}"),
            Self::Row(err) => write!(formatter, "Read feature cache row failed: {err}"),
        }
    }
}

pub(super) struct FeatureCacheRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> FeatureCacheRepository<'conn> {
    pub(super) fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    pub(super) fn load_source_rows(
        &self,
        source_id: &SourceId,
    ) -> Result<HashMap<String, FeatureStatus>, FeatureCacheRepositoryError> {
        let prefix = format!("{}::", source_id.as_str());
        let prefix_end = format!("{prefix}\u{10FFFF}");
        let mut stmt = self
            .conn
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
                 LEFT JOIN analysis_jobs j
                    ON j.sample_id = s.sample_id
                   AND j.job_type = ?1
                   AND j.readiness_managed = 1
                 WHERE s.sample_id >= ?3 AND s.sample_id < ?4",
            )
            .map_err(FeatureCacheRepositoryError::Prepare)?;
        let mut rows = stmt
            .query(params![
                ANALYSIS_JOB_TYPE,
                wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                prefix,
                prefix_end
            ])
            .map_err(FeatureCacheRepositoryError::Query)?;
        let mut source_rows = HashMap::new();
        while let Some(row) = rows.next().map_err(FeatureCacheRepositoryError::Query)? {
            let sample_id: String = row.get(0).map_err(FeatureCacheRepositoryError::Row)?;
            let Some(relative_path) = sample_id.split_once("::").map(|(_, path)| path) else {
                continue;
            };
            source_rows.insert(
                normalize_relative_key(relative_path),
                FeatureStatus {
                    duration_seconds: row
                        .get::<_, Option<f64>>(1)
                        .map_err(FeatureCacheRepositoryError::Row)?
                        .map(|seconds| seconds as f32),
                    sr_used: row
                        .get::<_, Option<i64>>(2)
                        .map_err(FeatureCacheRepositoryError::Row)?,
                    long_sample_mark: row
                        .get::<_, Option<i64>>(3)
                        .map_err(FeatureCacheRepositoryError::Row)?
                        .map(|value| value != 0),
                    has_features_v1: row
                        .get::<_, i64>(4)
                        .map_err(FeatureCacheRepositoryError::Row)?
                        != 0,
                    has_embedding: row
                        .get::<_, i64>(5)
                        .map_err(FeatureCacheRepositoryError::Row)?
                        != 0,
                    analysis_status: row
                        .get::<_, Option<String>>(6)
                        .map_err(FeatureCacheRepositoryError::Row)?
                        .as_deref()
                        .and_then(parse_job_status),
                },
            );
        }
        Ok(source_rows)
    }
}

pub(super) fn align_rows_to_entries(
    entry_paths: &[PathBuf],
    fallback_rows: &[Option<FeatureStatus>],
    mut source_rows: HashMap<String, FeatureStatus>,
) -> Vec<Option<FeatureStatus>> {
    entry_paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let key = normalize_relative_key(&path.to_string_lossy());
            Some(merge_fallback_row(
                source_rows.remove(&key).unwrap_or_else(empty_status),
                fallback_rows.get(index).and_then(|row| row.as_ref()),
            ))
        })
        .collect()
}

fn empty_status() -> FeatureStatus {
    FeatureStatus {
        has_features_v1: false,
        has_embedding: false,
        duration_seconds: None,
        sr_used: None,
        long_sample_mark: None,
        analysis_status: None,
    }
}

fn merge_fallback_row(
    mut status: FeatureStatus,
    fallback: Option<&FeatureStatus>,
) -> FeatureStatus {
    if status.duration_seconds.is_none()
        && let Some(fallback) = fallback
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
        && let Some(fallback) = fallback
    {
        status.long_sample_mark = fallback.long_sample_mark;
    }
    status
}

#[cfg(test)]
mod tests;
