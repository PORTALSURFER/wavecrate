use super::resolve::{ResolvedSimilarity, normalize_l2, open_source_db_for_id, rerank_with_dsp};
use super::*;
use crate::app::state::{
    EMPTY_SIMILARITY_ASPECT_SCORE_ROW, SimilarQuery, SimilarityAspectScoreRow,
};

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
    let snapshot = controller
        .current_browser_feature_cache_snapshot()
        .ok_or_else(|| "Similarity data unavailable for the current source".to_string())?;
    let request =
        loaded::loaded_audio_request(&loaded_audio, snapshot.key, snapshot.entry_paths.as_ref());
    if let Some(query) = loaded::cached_loaded_similarity_query(
        controller.runtime.similarity.loaded_query_cache.as_ref(),
        &request,
    ) {
        return Ok(query);
    }
    let data = loaded::build_loaded_similarity_query_data_with_cache(&conn, &request)?;
    controller.runtime.similarity.loaded_query_cache =
        Some(loaded::build_loaded_similarity_query_cache(&data));
    Ok(data.query)
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
    let features = wavecrate_analysis::compute_feature_vector_v1_for_path(path)?;
    let embedding = wavecrate_analysis::similarity::embedding_from_features(&features)?;
    let query_dsp = wavecrate_analysis::light_dsp_from_features_v1(&features).map(normalize_l2);
    let query_aspects =
        wavecrate_analysis::aspects::aspect_descriptors_from_features_v1(&features)?;
    let conn = open_source_db_for_id(controller, &source_id)?;
    let neighbours = wavecrate_analysis::ann_index::find_similar_for_embedding(
        &conn,
        &embedding,
        SIMILAR_RE_RANK_CANDIDATES,
    )?;
    let ranked = rerank_with_dsp(&conn, neighbours, Some(&embedding), query_dsp.as_deref())?;

    let mut candidate_ids = Vec::new();
    let mut indices = Vec::new();
    let mut scores = Vec::new();
    for (candidate_id, score) in ranked {
        let (candidate_source, relative_path) =
            super::analysis_jobs::parse_sample_id(&candidate_id)?;
        if candidate_source.as_str() != source_id.as_str() {
            continue;
        }
        if let Some(index) = controller.wav_index_for_path(&relative_path) {
            candidate_ids.push(candidate_id);
            indices.push(index);
            scores.push(score);
            if indices.len() >= DEFAULT_SIMILAR_COUNT {
                break;
            }
        }
    }
    let aspect_scores =
        aspect_scores_for_candidate_ids(&conn, Some(&query_aspects), &candidate_ids)?;
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
        aspect_scores,
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
    let (indices, scores, aspect_scores) = ensure_anchor_similarity_result(
        resolved.indices,
        resolved.scores,
        resolved.aspect_scores,
        anchor_index,
        resolved.anchor_aspect_scores,
    );
    SimilarQuery {
        sample_id: resolved.sample_id,
        label: label_builder(&resolved.relative_path),
        indices,
        scores,
        aspect_scores,
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

/// Ensure the anchor sample remains present in similarity results with self-similarity.
pub(super) fn ensure_anchor_similarity_result(
    mut indices: Vec<usize>,
    mut scores: Vec<f32>,
    mut aspect_scores: Vec<SimilarityAspectScoreRow>,
    anchor_index: Option<usize>,
    anchor_aspect_scores: SimilarityAspectScoreRow,
) -> (Vec<usize>, Vec<f32>, Vec<SimilarityAspectScoreRow>) {
    aspect_scores.resize(scores.len(), EMPTY_SIMILARITY_ASPECT_SCORE_ROW);
    let Some(anchor_index) = anchor_index else {
        return (indices, scores, aspect_scores);
    };
    let Some(existing_position) = indices.iter().position(|index| *index == anchor_index) else {
        indices.insert(0, anchor_index);
        scores.insert(0, 1.0);
        aspect_scores.insert(0, anchor_aspect_scores);
        return (indices, scores, aspect_scores);
    };
    if existing_position != 0 {
        indices.remove(existing_position);
        scores.remove(existing_position);
        aspect_scores.remove(existing_position);
        indices.insert(0, anchor_index);
        scores.insert(0, 1.0);
        aspect_scores.insert(0, anchor_aspect_scores);
    } else if let Some(anchor_score) = scores.get_mut(0) {
        *anchor_score = 1.0;
        if let Some(row) = aspect_scores.get_mut(0) {
            *row = anchor_aspect_scores;
        }
    }
    (indices, scores, aspect_scores)
}

fn aspect_scores_for_candidate_ids(
    conn: &rusqlite::Connection,
    query_aspects: Option<&wavecrate_analysis::aspects::AspectDescriptorSet>,
    candidate_ids: &[String],
) -> Result<Vec<SimilarityAspectScoreRow>, String> {
    if query_aspects.is_none() {
        return Ok(vec![EMPTY_SIMILARITY_ASPECT_SCORE_ROW; candidate_ids.len()]);
    }
    let descriptors = super::resolve::load_aspect_descriptors_for_samples(conn, candidate_ids)?;
    Ok(candidate_ids
        .iter()
        .map(|sample_id| {
            super::resolve::similarity_aspect_score_row(query_aspects, descriptors.get(sample_id))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
    use crate::app::state::empty_similarity_aspect_score_rows;

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
        let mut anchor_aspects = EMPTY_SIMILARITY_ASPECT_SCORE_ROW;
        anchor_aspects[0] = Some(1.0);
        let (indices, scores, aspect_scores) = ensure_anchor_similarity_result(
            vec![3, 5],
            vec![0.7, 0.4],
            empty_similarity_aspect_score_rows(2),
            Some(2),
            anchor_aspects,
        );
        assert_eq!(indices, vec![2, 3, 5]);
        assert_eq!(scores, vec![1.0, 0.7, 0.4]);
        assert_eq!(aspect_scores[0], anchor_aspects);
    }

    #[test]
    fn ensure_anchor_similarity_result_moves_existing_anchor_to_front() {
        let mut anchor_aspects = EMPTY_SIMILARITY_ASPECT_SCORE_ROW;
        anchor_aspects[0] = Some(1.0);
        let mut row_for_three = EMPTY_SIMILARITY_ASPECT_SCORE_ROW;
        row_for_three[0] = Some(0.7);
        let mut row_for_two = EMPTY_SIMILARITY_ASPECT_SCORE_ROW;
        row_for_two[0] = Some(0.98);
        let mut row_for_five = EMPTY_SIMILARITY_ASPECT_SCORE_ROW;
        row_for_five[0] = Some(0.4);
        let (indices, scores, aspect_scores) = ensure_anchor_similarity_result(
            vec![3, 2, 5],
            vec![0.7, 0.98, 0.4],
            vec![row_for_three, row_for_two, row_for_five],
            Some(2),
            anchor_aspects,
        );
        assert_eq!(indices, vec![2, 3, 5]);
        assert_eq!(scores, vec![1.0, 0.7, 0.4]);
        assert_eq!(
            aspect_scores,
            vec![anchor_aspects, row_for_three, row_for_five]
        );
    }

    #[test]
    fn build_similarity_query_for_loaded_sample_reuses_matching_cache() {
        let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
            "cached.wav",
            crate::sample_sources::Rating::NEUTRAL,
        )]);
        controller.selection_state.ctx.selected_source = Some(source.id.clone());
        controller.sample_view.wav.loaded_audio =
            Some(crate::app::controller::state::audio::LoadedAudio {
                source_id: source.id.clone(),
                root: source.root.clone(),
                relative_path: PathBuf::from("cached.wav"),
                bytes: std::sync::Arc::from(Vec::<u8>::new()),
                duration_seconds: 1.0,
                sample_rate: 44_100,
                channels: 1,
            });
        let snapshot = controller
            .current_browser_feature_cache_snapshot()
            .expect("browser snapshot");
        let sample_id = crate::app::controller::library::analysis_jobs::build_sample_id(
            source.id.as_str(),
            Path::new("cached.wav"),
        );
        let expected = SimilarQuery {
            sample_id: sample_id.clone(),
            label: "Loaded: cached.wav".to_string(),
            indices: vec![0],
            scores: vec![1.0],
            aspect_scores: empty_similarity_aspect_score_rows(1),
            anchor_index: Some(0),
        };
        controller.runtime.similarity.loaded_query_cache = Some(
            crate::app::controller::state::runtime::LoadedSimilarityQueryCache {
                sample_id,
                query: expected.clone(),
                source_snapshot:
                    crate::app::controller::state::runtime::LoadedSimilaritySourceSnapshot {
                        source_id: source.id.clone(),
                        key: snapshot.key,
                        candidates: std::sync::Arc::from([]),
                    },
            },
        );

        let query =
            build_similarity_query_for_loaded_sample(&mut controller).expect("cached query");

        assert_eq!(query.indices, expected.indices);
        assert_eq!(query.scores, expected.scores);
        assert_eq!(query.anchor_index, expected.anchor_index);
        assert_eq!(query.label, expected.label);
    }
}
