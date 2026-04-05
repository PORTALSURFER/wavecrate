//! Background similarity-query helpers used by automatic UI refresh paths.

use super::resolve::{
    is_effectively_silent, load_embedding_for_sample, load_light_dsp_for_sample,
    load_rms_for_sample, rerank_with_dsp,
};
use super::*;
use crate::app::controller::jobs::{FocusedSimilarityPaths, JobMessage};
use crate::app::controller::state::runtime::{
    PendingFocusedSimilarityQuery, PendingFocusedSimilarityRefresh, PendingLoadedSimilarityQuery,
};
use crate::sample_sources::SourceId;
use rusqlite::{OptionalExtension, params};
use std::{path::PathBuf, sync::Arc};

/// Background request to refresh focused near-duplicate highlights.
#[derive(Clone, Debug)]
pub(crate) struct FocusedSimilarityJob {
    /// Monotonic request identifier used to drop stale async results.
    pub(crate) request_id: u64,
    /// Source that owns the selected sample.
    pub(crate) source_id: SourceId,
    /// Root path used to open the source database.
    pub(crate) source_root: PathBuf,
    /// Stable sample identifier for the focused sample.
    pub(crate) sample_id: String,
    /// Relative path expected to remain selected on apply.
    pub(crate) relative_path: PathBuf,
    /// Focused browser entry index captured at queue time.
    pub(crate) anchor_index: Option<usize>,
    /// Whether fast similarity-prep mode is enabled.
    pub(crate) fast_mode_enabled: bool,
    /// Fast-prep sample rate used to detect partial analysis rows.
    pub(crate) fast_sample_rate: u32,
}

/// Background request to rebuild follow-loaded similarity ordering.
#[derive(Clone, Debug)]
pub(crate) struct LoadedSimilarityQueryJob {
    /// Monotonic request identifier used to drop stale async results.
    pub(crate) request_id: u64,
    /// Source that owns the loaded sample.
    pub(crate) source_id: SourceId,
    /// Root path used to open the source database.
    pub(crate) source_root: PathBuf,
    /// Relative path of the loaded sample.
    pub(crate) relative_path: PathBuf,
    /// Browser snapshot key the query indices must still match.
    pub(crate) key: crate::app::controller::FeatureCacheKey,
    /// Snapshot of current wav-entry paths used to map scores back to browser indices.
    pub(crate) entry_paths: Arc<[PathBuf]>,
}

pub(crate) fn queue_focused_similarity_highlight_refresh(
    controller: &mut AppController,
    pending: PendingFocusedSimilarityRefresh,
) {
    let Some(source) = controller.current_source() else {
        controller.clear_focused_similarity_highlight();
        return;
    };
    let request_id = controller.runtime.jobs.next_similarity_request_id();
    controller.runtime.pending_focused_similarity_query = Some(PendingFocusedSimilarityQuery {
        request_id,
        source_id: source.id.clone(),
        relative_path: pending.relative_path.clone(),
    });
    let job = FocusedSimilarityJob {
        request_id,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        sample_id: pending.sample_id.clone(),
        relative_path: pending.relative_path.clone(),
        anchor_index: pending.anchor_index,
        fast_mode_enabled: controller.similarity_prep_fast_mode_enabled(),
        fast_sample_rate: controller.similarity_prep_fast_sample_rate(),
    };
    controller.runtime.jobs.spawn_one_shot_job(
        true,
        move || {
            let result = compute_focused_similarity(job.clone());
            crate::app::controller::jobs::FocusedSimilarityResult {
                request_id: job.request_id,
                source_id: job.source_id,
                relative_path: job.relative_path,
                result,
            }
        },
        JobMessage::FocusedSimilarityLoaded,
    );
}

pub(crate) fn queue_loaded_similarity_query_refresh(
    controller: &mut AppController,
) -> Result<(), String> {
    if !controller.ui.browser.search.similarity_sort_follow_loaded {
        return Ok(());
    }
    if controller.ui.browser.search.sort != SampleBrowserSort::Similarity {
        return Ok(());
    }
    if controller.ui.browser.search.similar_query.is_some() {
        return Ok(());
    }
    let loaded_audio = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .ok_or_else(|| "Load a sample to sort by similarity".to_string())?;
    let source_id = loaded_audio.source_id.clone();
    let loaded_relative_path = loaded_audio.relative_path.clone();
    if controller.selection_state.ctx.selected_source.as_ref() != Some(&source_id) {
        return Err("Select the loaded sample's source to sort by similarity".to_string());
    }
    let Some(source) = controller.current_source() else {
        return Err("Source not found".to_string());
    };
    let snapshot = controller
        .current_browser_feature_cache_snapshot()
        .ok_or_else(|| "Similarity data unavailable for the current source".to_string())?;
    let request = loaded::build_loaded_similarity_request(
        &source.id,
        &loaded_relative_path,
        snapshot.key,
        snapshot.entry_paths.as_ref(),
    );
    if let Some(query) = loaded::cached_loaded_similarity_query(
        controller.runtime.loaded_similarity_query_cache.as_ref(),
        &request,
    ) {
        controller.runtime.pending_loaded_similarity_query = None;
        controller.ui.browser.search.search_busy = false;
        controller.ui.browser.search.similar_query = Some(query);
        if controller.should_dispatch_browser_search_async() {
            controller.dispatch_search_job();
        } else {
            controller.rebuild_browser_lists();
        }
        return Ok(());
    }
    let request_id = controller.runtime.jobs.next_similarity_request_id();
    controller.runtime.pending_loaded_similarity_query = Some(PendingLoadedSimilarityQuery {
        request_id,
        source_id: source.id.clone(),
        relative_path: loaded_relative_path.clone(),
        key: snapshot.key,
    });
    controller.ui.browser.search.search_busy = true;
    controller.mark_browser_search_projection_revision_dirty();
    let job = LoadedSimilarityQueryJob {
        request_id,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        relative_path: loaded_relative_path,
        key: snapshot.key,
        entry_paths: snapshot.entry_paths,
    };
    controller.runtime.jobs.spawn_one_shot_job(
        true,
        move || {
            let result = compute_loaded_similarity_query(job.clone());
            crate::app::controller::jobs::LoadedSimilarityQueryResult {
                request_id: job.request_id,
                source_id: job.source_id,
                relative_path: job.relative_path,
                key: job.key,
                result,
            }
        },
        JobMessage::LoadedSimilarityQueryBuilt,
    );
    Ok(())
}

/// Compute focused near-duplicate highlights without touching controller state.
pub(crate) fn compute_focused_similarity(
    job: FocusedSimilarityJob,
) -> Result<Option<FocusedSimilarityPaths>, String> {
    let mut conn =
        crate::app::controller::library::analysis_jobs::open_source_db(&job.source_root)?;
    maybe_enqueue_full_analysis_for_request(
        &mut conn,
        &job.sample_id,
        job.fast_mode_enabled,
        job.fast_sample_rate,
    )?;
    if let Some(rms) = load_rms_for_sample(&conn, &job.sample_id)?
        && is_effectively_silent(rms)
    {
        return Err("Selected sample is effectively silent".to_string());
    }
    let neighbours = crate::analysis::ann_index::find_similar(
        &conn,
        &job.sample_id,
        SIMILAR_RE_RANK_CANDIDATES,
    )?;
    let query_embedding = load_embedding_for_sample(&conn, &job.sample_id)?;
    let query_dsp = load_light_dsp_for_sample(&conn, &job.sample_id)?;
    let ranked = rerank_with_dsp(
        &conn,
        neighbours,
        query_embedding.as_deref(),
        query_dsp.as_deref(),
    )?;
    let (paths, scores) = filter_ranked_candidate_paths(
        &conn,
        ranked,
        &job.source_id,
        Some(DUPLICATE_SCORE_THRESHOLD),
    )?;
    if paths.is_empty() {
        return Ok(None);
    }
    Ok(Some(FocusedSimilarityPaths {
        sample_id: job.sample_id,
        paths,
        scores,
        anchor_index: job.anchor_index,
    }))
}

/// Compute follow-loaded similarity ordering without touching controller state.
pub(crate) fn compute_loaded_similarity_query(
    job: LoadedSimilarityQueryJob,
) -> Result<crate::app::controller::state::runtime::LoadedSimilarityQueryData, String> {
    let conn = crate::app::controller::library::analysis_jobs::open_source_db(&job.source_root)?;
    let request = loaded::build_loaded_similarity_request(
        &job.source_id,
        &job.relative_path,
        job.key,
        &job.entry_paths,
    );
    loaded::build_loaded_similarity_query_data_with_cache(&conn, &request)
}

fn maybe_enqueue_full_analysis_for_request(
    conn: &mut rusqlite::Connection,
    sample_id: &str,
    fast_mode_enabled: bool,
    fast_sample_rate: u32,
) -> Result<(), String> {
    if !fast_mode_enabled {
        return Ok(());
    }
    let row: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT content_hash, analysis_version FROM samples WHERE sample_id = ?1",
            params![sample_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|err| format!("Load analysis version failed: {err}"))?;
    let Some((content_hash, analysis_version)) = row else {
        return Ok(());
    };
    if content_hash.trim().is_empty() {
        return Ok(());
    }
    let fast_version = crate::analysis::version::analysis_version_for_sample_rate(fast_sample_rate);
    if analysis_version.as_deref() != Some(fast_version.as_str()) {
        return Ok(());
    }
    let active: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM analysis_jobs
             WHERE sample_id = ?1 AND job_type = ?2 AND status IN ('pending','running')",
            params![sample_id, "wav_metadata_v1"],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if active > 0 {
        return Ok(());
    }
    let (source_id, relative_path) =
        crate::app::controller::library::analysis_jobs::parse_sample_id(sample_id)?;
    let created_at = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    };
    conn.execute(
        "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', 0, ?6)
         ON CONFLICT(sample_id, job_type) DO UPDATE SET
            source_id = excluded.source_id,
            relative_path = excluded.relative_path,
            content_hash = excluded.content_hash,
            status = 'pending',
            attempts = 0,
            created_at = excluded.created_at,
            last_error = NULL",
        params![
            sample_id,
            source_id,
            relative_path.to_string_lossy().replace('\\', "/"),
            "wav_metadata_v1",
            content_hash,
            created_at
        ],
    )
    .map_err(|err| format!("Enqueue analysis job failed: {err}"))?;
    Ok(())
}

fn filter_ranked_candidate_paths(
    conn: &rusqlite::Connection,
    ranked: impl IntoIterator<Item = (String, f32)>,
    source_id: &SourceId,
    score_cutoff: Option<f32>,
) -> Result<(Vec<PathBuf>, Vec<f32>), String> {
    let mut paths = Vec::new();
    let mut scores = Vec::new();
    let apply_duplicate_filters = score_cutoff.is_some();
    for (candidate_id, score) in ranked {
        if let Some(cutoff) = score_cutoff
            && score < cutoff
        {
            break;
        }
        let (candidate_source, relative_path) =
            crate::app::controller::library::analysis_jobs::parse_sample_id(&candidate_id)?;
        if candidate_source.as_str() != source_id.as_str() {
            continue;
        }
        if apply_duplicate_filters
            && let Some(rms) = load_rms_for_sample(conn, &candidate_id)?
            && is_effectively_silent(rms)
        {
            continue;
        }
        paths.push(relative_path);
        scores.push(score);
        if paths.len() >= DEFAULT_SIMILAR_COUNT {
            break;
        }
    }
    Ok((paths, scores))
}
