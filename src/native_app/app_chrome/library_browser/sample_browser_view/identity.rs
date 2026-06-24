use wavecrate_analysis::aspects::SimilarityAspect;

pub(super) fn sample_row_key(file_id: &str) -> String {
    format!("sample-row-{file_id}")
}

pub(super) fn similarity_anchor_key(file_id: &str) -> String {
    format!("sample-similarity-anchor-{file_id}")
}

pub(super) fn playback_type_key(file_id: &str) -> String {
    format!("sample-playback-type-{file_id}")
}

pub(super) fn collection_key(file_id: &str) -> String {
    format!("sample-collection-{file_id}")
}

pub(super) fn similarity_score_key(file_id: &str) -> String {
    format!("sample-similarity-score-{file_id}")
}

pub(super) fn missing_similarity_score_key(file_id: &str) -> String {
    format!("sample-similarity-score-missing-{file_id}")
}

pub(super) fn similarity_aspect_key(aspect: SimilarityAspect, file_id: &str) -> String {
    format!("sample-similarity-aspect-{}-{file_id}", aspect.index())
}

pub(super) fn rating_key(file_id: &str) -> String {
    format!("sample-rating-{file_id}")
}

pub(super) fn text_cell_key(file_id: &str, column_id: &str) -> String {
    format!("sample-{file_id}-{column_id}")
}
