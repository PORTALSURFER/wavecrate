use super::*;
use crate::app::state::FocusedSimilarity;
use crate::app::view_model;
use std::path::PathBuf;

mod apply;
mod background;
mod query;
mod resolve;

const DEFAULT_SIMILAR_COUNT: usize = 40;
const SIMILAR_RE_RANK_CANDIDATES: usize = 200;
const EMBED_WEIGHT: f32 = 0.8;
const DSP_WEIGHT: f32 = 0.2;
const DUPLICATE_SCORE_THRESHOLD: f32 = 0.995;
const DUPLICATE_RMS_MIN: f32 = 1.0e-4;
const FEATURE_RMS_INDEX: usize = 2;
const MISSING_SIMILARITY_SCORE: f32 = -2.0;

pub(crate) fn find_similar_for_visible_row(
    controller: &mut AppController,
    visible_row: usize,
) -> Result<(), String> {
    let (sample_id, entry_index) =
        resolve::resolve_sample_id_for_visible_row(controller, visible_row)?;
    apply_similarity_for_sample_id(
        controller,
        &sample_id,
        None,
        view_model::sample_display_label,
        Some(entry_index),
        "No similar samples found in the current source",
    )
}

pub(crate) fn find_duplicates_for_visible_row(
    controller: &mut AppController,
    visible_row: usize,
) -> Result<(), String> {
    let (sample_id, entry_index) =
        resolve::resolve_sample_id_for_visible_row(controller, visible_row)?;
    apply_similarity_for_sample_id(
        controller,
        &sample_id,
        Some(DUPLICATE_SCORE_THRESHOLD),
        |path| format!("Duplicates of {}", view_model::sample_display_label(path)),
        Some(entry_index),
        "No duplicates found in the current source",
    )
}

pub(crate) fn find_similar_for_sample_id(
    controller: &mut AppController,
    sample_id: &str,
) -> Result<(), String> {
    apply_similarity_for_sample_id(
        controller,
        sample_id,
        None,
        view_model::sample_display_label,
        None,
        "No similar samples found in the current source",
    )
}

pub(crate) fn clear_similar_filter(controller: &mut AppController) {
    apply::clear_similar_filter(controller);
}

pub(crate) fn queue_focused_similarity_highlight_refresh(
    controller: &mut AppController,
    pending: crate::app::controller::state::runtime::PendingFocusedSimilarityRefresh,
) {
    background::queue_focused_similarity_highlight_refresh(controller, pending);
}

pub(crate) fn queue_loaded_similarity_query_refresh(
    controller: &mut AppController,
) -> Result<(), String> {
    background::queue_loaded_similarity_query_refresh(controller)
}

fn apply_similarity_for_sample_id(
    controller: &mut AppController,
    sample_id: &str,
    score_cutoff: Option<f32>,
    label_builder: impl FnOnce(&Path) -> String,
    anchor_override: Option<usize>,
    empty_error: &str,
) -> Result<(), String> {
    let query = query::build_similar_query_for_sample_id(
        controller,
        sample_id,
        score_cutoff,
        label_builder,
        anchor_override,
        empty_error,
    )?;
    apply::apply_similarity_query(controller, query);
    Ok(())
}

pub(crate) fn find_similar_for_audio_path(
    controller: &mut AppController,
    path: &Path,
) -> Result<(), String> {
    let query = query::build_similarity_query_for_audio_path(controller, path)?;
    apply::apply_similarity_query(controller, query);
    Ok(())
}

pub(crate) fn enable_loaded_similarity_sort(controller: &mut AppController) -> Result<(), String> {
    controller.ui.browser.search.similarity_sort_follow_loaded = true;
    background::queue_loaded_similarity_query_refresh(controller)
}

pub(crate) fn disable_similarity_sort(controller: &mut AppController) {
    apply::disable_similarity_sort(controller);
}

pub(crate) fn focused_similarity_from_paths(
    sample_id: String,
    paths: Vec<PathBuf>,
    scores: Vec<f32>,
    anchor_index: Option<usize>,
    mut resolve_index: impl FnMut(&Path) -> Option<usize>,
) -> Option<FocusedSimilarity> {
    let mut indices = Vec::new();
    let mut mapped_scores = Vec::new();
    for (path, score) in paths.into_iter().zip(scores.into_iter()) {
        if let Some(index) = resolve_index(&path) {
            if anchor_index == Some(index) {
                continue;
            }
            indices.push(index);
            mapped_scores.push(score);
        }
    }
    if indices.is_empty() {
        return None;
    }
    Some(FocusedSimilarity {
        sample_id,
        indices,
        scores: mapped_scores,
        anchor_index,
    })
}

fn focused_similarity_from_resolved(
    resolved: resolve::ResolvedSimilarity,
    anchor_index: Option<usize>,
) -> Option<FocusedSimilarity> {
    let mut indices = Vec::new();
    let mut scores = Vec::new();
    for (index, score) in resolved
        .indices
        .into_iter()
        .zip(resolved.scores.into_iter())
    {
        if anchor_index == Some(index) {
            continue;
        }
        indices.push(index);
        scores.push(score);
    }
    if indices.is_empty() {
        return None;
    }
    Some(FocusedSimilarity {
        sample_id: resolved.sample_id,
        indices,
        scores,
        anchor_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn focused_similarity_from_resolved_skips_anchor() {
        let resolved = resolve::ResolvedSimilarity {
            sample_id: "source::a.wav".to_string(),
            relative_path: PathBuf::from("a.wav"),
            indices: vec![1, 2, 3],
            scores: vec![0.99, 0.98, 0.97],
        };
        let highlight = focused_similarity_from_resolved(resolved, Some(2)).expect("highlight");
        assert_eq!(highlight.indices, vec![1, 3]);
        assert_eq!(highlight.scores, vec![0.99, 0.97]);
        assert_eq!(highlight.anchor_index, Some(2));
    }

    #[test]
    fn focused_similarity_from_resolved_returns_none_when_empty() {
        let resolved = resolve::ResolvedSimilarity {
            sample_id: "source::a.wav".to_string(),
            relative_path: PathBuf::from("a.wav"),
            indices: vec![4],
            scores: vec![0.99],
        };
        let highlight = focused_similarity_from_resolved(resolved, Some(4));
        assert!(highlight.is_none());
    }
}
