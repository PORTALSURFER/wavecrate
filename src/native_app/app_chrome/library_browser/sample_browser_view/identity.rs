use crate::native_app::ui::ids as widget_ids;
use radiant::widgets::stable_widget_id;
use wavecrate_analysis::aspects::SimilarityAspect;

pub(super) const SAMPLE_NAME_VIEW_MODE_TOGGLE_KEY: &str = "sample-name-view-mode-toggle";
pub(super) const SAMPLE_COLUMN_DROP_MARKER_KEY: &str = "sample-column-drop-marker";
pub(super) const SAMPLE_HEADER_SIMILARITY_LABEL_KEY: &str = "sample-header-similarity-label";
pub(super) const SAMPLE_HEADER_SIMILARITY_KEY: &str = "sample-header-similarity";
pub(super) const SAMPLE_ROW_INPUT_SCOPE: u64 = widget_ids::SAMPLE_ROW_INPUT_SCOPE;

pub(super) fn sample_similarity_weighting_toggle_id() -> u64 {
    widget_ids::SAMPLE_SIMILARITY_WEIGHTING_TOGGLE_ID
}

pub(super) fn random_navigation_toggle_id() -> u64 {
    widget_ids::SAMPLE_RANDOM_NAVIGATION_TOGGLE_ID
}

pub(super) fn sample_header_cell_id(column_id: &str) -> u64 {
    stable_widget_id(widget_ids::SAMPLE_HEADER_CELL_ID, column_id)
}

pub(super) fn sample_similarity_header_aspect_key(label: &str) -> String {
    format!("sample-header-similarity-aspect-{label}")
}

pub(super) fn sample_similarity_aspect_toggle_id(aspect: SimilarityAspect) -> u64 {
    stable_widget_id(
        widget_ids::SAMPLE_SIMILARITY_ASPECT_TOGGLE_SCOPE,
        similarity_aspect_control_key(aspect),
    )
}

pub(super) fn sample_similarity_aspect_weight_id(aspect: SimilarityAspect) -> u64 {
    stable_widget_id(
        widget_ids::SAMPLE_SIMILARITY_ASPECT_WEIGHT_SCOPE,
        similarity_aspect_control_key(aspect),
    )
}

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

fn similarity_aspect_control_key(aspect: SimilarityAspect) -> &'static str {
    match aspect {
        SimilarityAspect::Overall => "overall",
        SimilarityAspect::Spectrum => "spectrum",
        SimilarityAspect::Timbre => "timbre",
        SimilarityAspect::Pitch => "pitch",
        SimilarityAspect::Amplitude => "amplitude",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn sample_header_cell_ids_are_stable_per_column() {
        assert_eq!(sample_header_cell_id("name"), sample_header_cell_id("name"));
        assert_ne!(sample_header_cell_id("name"), sample_header_cell_id("size"));
    }

    #[test]
    fn similarity_aspect_control_ids_are_distinct_by_role_and_aspect() {
        let toggle_ids = SimilarityAspect::ORDER
            .iter()
            .map(|aspect| sample_similarity_aspect_toggle_id(*aspect))
            .collect::<BTreeSet<_>>();
        let weight_ids = SimilarityAspect::ORDER
            .iter()
            .map(|aspect| sample_similarity_aspect_weight_id(*aspect))
            .collect::<BTreeSet<_>>();

        assert_eq!(toggle_ids.len(), SimilarityAspect::ORDER.len());
        assert_eq!(weight_ids.len(), SimilarityAspect::ORDER.len());
        assert!(toggle_ids.is_disjoint(&weight_ids));
    }
}
