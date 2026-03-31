use super::resolve::{ResolvedSimilarity, normalize_l2, open_source_db_for_id, rerank_with_dsp};
use super::*;
use crate::app::state::SimilarQuery;
use std::path::PathBuf;

pub(crate) fn build_similar_query_for_sample_id(
    controller: &mut AppController,
    sample_id: &str,
    score_cutoff: Option<f32>,
    label_builder: impl FnOnce(&Path) -> String,
    anchor_override: Option<usize>,
    empty_error: &str,
) -> Result<SimilarQuery, String> {
    let resolved = resolve::resolve_similarity_for_sample_id(controller, sample_id, score_cutoff)?;
    if resolved.indices.is_empty() {
        return Err(empty_error.to_string());
    }
    Ok(build_similar_query_from_resolved(
        controller,
        resolved,
        label_builder,
        anchor_override,
    ))
}

pub(crate) fn build_similarity_query_for_loaded_sample(
    controller: &mut AppController,
) -> Result<SimilarQuery, String> {
    let loaded_audio = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .cloned()
        .ok_or_else(|| "Load a sample to sort by similarity".to_string())?;
    let source_id: crate::sample_sources::SourceId = loaded_audio.source_id.clone();
    if controller.selection_state.ctx.selected_source.as_ref() != Some(&source_id) {
        return Err("Select the loaded sample's source to sort by similarity".to_string());
    }
    let conn = open_source_db_for_id(controller, &source_id)?;
    let entry_paths = collect_wav_entry_paths(controller)?;
    let request = loaded::loaded_audio_request(&loaded_audio, &entry_paths);
    loaded::build_loaded_similarity_query(&conn, request)
}

pub(crate) fn build_similarity_query_for_audio_path(
    controller: &mut AppController,
    path: &Path,
) -> Result<SimilarQuery, String> {
    let source_id = controller
        .selection_state
        .ctx
        .selected_source
        .clone()
        .ok_or_else(|| "No active source selected".to_string())?;
    let features = crate::analysis::compute_feature_vector_v1_for_path(path)?;
    let embedding = crate::analysis::similarity::embedding_from_features(&features)?;
    let query_dsp = crate::analysis::light_dsp_from_features_v1(&features).map(normalize_l2);
    let conn = open_source_db_for_id(controller, &source_id)?;
    let neighbours = crate::analysis::ann_index::find_similar_for_embedding(
        &conn,
        &embedding,
        SIMILAR_RE_RANK_CANDIDATES,
    )?;
    let ranked = rerank_with_dsp(&conn, neighbours, Some(&embedding), query_dsp.as_deref())?;

    let mut indices = Vec::new();
    let mut scores = Vec::new();
    for (candidate_id, score) in ranked {
        let (candidate_source, relative_path) =
            super::analysis_jobs::parse_sample_id(&candidate_id)?;
        if candidate_source.as_str() != source_id.as_str() {
            continue;
        }
        if let Some(index) = controller.wav_index_for_path(&relative_path) {
            indices.push(index);
            scores.push(score);
            if indices.len() >= DEFAULT_SIMILAR_COUNT {
                break;
            }
        }
    }
    if indices.is_empty() {
        return Err("No similar samples found in the current source".to_string());
    }
    let label = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("Clip: {name}"))
        .unwrap_or_else(|| "Clip".to_string());
    Ok(SimilarQuery {
        sample_id: format!("clip::{}", path.display()),
        label,
        indices,
        scores,
        anchor_index: None,
    })
}

fn build_similar_query_from_resolved(
    controller: &mut AppController,
    resolved: ResolvedSimilarity,
    label_builder: impl FnOnce(&Path) -> String,
    anchor_override: Option<usize>,
) -> SimilarQuery {
    let anchor_index = resolve_anchor_index(controller, &resolved.relative_path, anchor_override);
    let (indices, scores) =
        ensure_anchor_similarity_result(resolved.indices, resolved.scores, anchor_index);
    SimilarQuery {
        sample_id: resolved.sample_id,
        label: label_builder(&resolved.relative_path),
        indices,
        scores,
        anchor_index,
    }
}

fn resolve_anchor_index(
    controller: &mut AppController,
    relative_path: &Path,
    anchor_override: Option<usize>,
) -> Option<usize> {
    anchor_override.or_else(|| controller.wav_index_for_path(relative_path))
}

fn collect_wav_entry_paths(controller: &mut AppController) -> Result<Vec<PathBuf>, String> {
    let mut entry_paths = Vec::with_capacity(controller.wav_entries_len());
    controller.for_each_wav_entry(|_, entry| {
        entry_paths.push(entry.relative_path.clone());
    })?;
    Ok(entry_paths)
}

/// Ensure the anchor sample remains present in similarity results with self-similarity.
pub(super) fn ensure_anchor_similarity_result(
    mut indices: Vec<usize>,
    mut scores: Vec<f32>,
    anchor_index: Option<usize>,
) -> (Vec<usize>, Vec<f32>) {
    let Some(anchor_index) = anchor_index else {
        return (indices, scores);
    };
    let Some(existing_position) = indices.iter().position(|index| *index == anchor_index) else {
        indices.insert(0, anchor_index);
        scores.insert(0, 1.0);
        return (indices, scores);
    };
    if existing_position != 0 {
        indices.remove(existing_position);
        scores.remove(existing_position);
        indices.insert(0, anchor_index);
        scores.insert(0, 1.0);
    } else if let Some(anchor_score) = scores.get_mut(0) {
        *anchor_score = 1.0;
    }
    (indices, scores)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};

    #[test]
    fn resolve_anchor_index_prefers_override() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![sample_entry(
            "a.wav",
            crate::sample_sources::Rating::NEUTRAL,
        )]);
        let anchor = resolve_anchor_index(&mut controller, Path::new("a.wav"), Some(7));
        assert_eq!(anchor, Some(7));
    }

    #[test]
    fn ensure_anchor_similarity_result_inserts_missing_anchor_at_front() {
        let (indices, scores) =
            ensure_anchor_similarity_result(vec![3, 5], vec![0.7, 0.4], Some(2));
        assert_eq!(indices, vec![2, 3, 5]);
        assert_eq!(scores, vec![1.0, 0.7, 0.4]);
    }

    #[test]
    fn ensure_anchor_similarity_result_moves_existing_anchor_to_front() {
        let (indices, scores) =
            ensure_anchor_similarity_result(vec![3, 2, 5], vec![0.7, 0.98, 0.4], Some(2));
        assert_eq!(indices, vec![2, 3, 5]);
        assert_eq!(scores, vec![1.0, 0.7, 0.4]);
    }
}
