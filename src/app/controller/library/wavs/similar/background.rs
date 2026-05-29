//! Background similarity-query helpers used by automatic UI refresh paths.

mod compute;

use super::*;
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::state::runtime::{
    PendingFocusedSimilarityQuery, PendingFocusedSimilarityRefresh, PendingLoadedSimilarityQuery,
};
use crate::app::state::ProgressTaskKind;

pub(crate) use compute::{
    FocusedSimilarityJob, LoadedSimilarityQueryJob, compute_focused_similarity,
    compute_loaded_similarity_query,
};

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
        controller.clear_progress_task(ProgressTaskKind::Search);
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
    controller.show_status_progress(ProgressTaskKind::Search, "Filtering samples", 0, false);
    controller.update_progress_detail_for_task(
        ProgressTaskKind::Search,
        format!(
            "Refreshing similarity ordering for {}",
            loaded_relative_path.display()
        ),
    );
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
