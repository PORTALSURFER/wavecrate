use crate::native_app::ui::ids as widget_ids;
use radiant::widgets::stable_widget_id;
use wavecrate_analysis::aspects::SimilarityAspect;

/// Retained key for the similarity-anchor button inside each keyed sample row.
pub(super) const RETAINED_SIMILARITY_ANCHOR_BUTTON_KEY: &str = "sample-similarity-anchor";
/// Scope for retained sample-row input identity.
pub(super) const RETAINED_SAMPLE_ROW_INPUT_SCOPE: u64 = widget_ids::RETAINED_SAMPLE_ROW_INPUT_SCOPE;

/// Automation-facing id for the similarity weighting toggle.
pub(super) fn automation_similarity_weighting_toggle_id() -> u64 {
    widget_ids::AUTOMATION_SAMPLE_SIMILARITY_WEIGHTING_TOGGLE_ID
}

/// Automation-facing id for the random navigation toggle.
pub(super) fn automation_random_navigation_toggle_id() -> u64 {
    widget_ids::AUTOMATION_SAMPLE_RANDOM_NAVIGATION_TOGGLE_ID
}

/// Retained id for a sortable, draggable, resizable sample header cell.
pub(super) fn retained_sample_header_cell_id(column_id: &str) -> u64 {
    stable_widget_id(widget_ids::RETAINED_SAMPLE_HEADER_CELL_ID, column_id)
}

#[cfg(test)]
/// Radiant-derived child ids for a retained sample header cell.
pub(super) fn retained_sample_header_child_ids(
    column_id: &str,
) -> radiant::prelude::CompactDetailsHeaderCellIds {
    radiant::prelude::CompactDetailsHeaderCellIds::from_cell_id(retained_sample_header_cell_id(
        column_id,
    ))
}

/// Automation-facing id for one similarity aspect enabled toggle.
pub(super) fn automation_similarity_aspect_toggle_id(aspect: SimilarityAspect) -> u64 {
    stable_widget_id(
        widget_ids::AUTOMATION_SAMPLE_SIMILARITY_ASPECT_TOGGLE_SCOPE,
        similarity_aspect_control_key(aspect),
    )
}

/// Automation-facing id for one similarity aspect weight slider.
pub(super) fn automation_similarity_aspect_weight_id(aspect: SimilarityAspect) -> u64 {
    stable_widget_id(
        widget_ids::AUTOMATION_SAMPLE_SIMILARITY_ASPECT_WEIGHT_SCOPE,
        similarity_aspect_control_key(aspect),
    )
}

/// Retained row key for dynamic sample-row view subtrees.
pub(super) fn retained_sample_row_key(file_id: &str) -> String {
    format!("sample-row-{file_id}")
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
        assert_eq!(
            retained_sample_header_cell_id("name"),
            retained_sample_header_cell_id("name")
        );
        assert_ne!(
            retained_sample_header_cell_id("name"),
            retained_sample_header_cell_id("size")
        );
    }

    #[test]
    /// Verifies header-cell child controls remain derived by Radiant.
    fn sample_header_child_ids_are_derived_by_radiant() {
        let cell_id = retained_sample_header_cell_id("name");
        let child_ids = retained_sample_header_child_ids("name");

        assert_eq!(
            child_ids.sort_drag,
            Some(radiant::prelude::compact_details_header_sort_drag_id(
                cell_id
            ))
        );
        assert_eq!(
            child_ids.resize,
            Some(radiant::prelude::compact_details_header_resize_id(cell_id))
        );
    }

    #[test]
    fn similarity_aspect_control_ids_are_distinct_by_role_and_aspect() {
        let toggle_ids = SimilarityAspect::ORDER
            .iter()
            .map(|aspect| automation_similarity_aspect_toggle_id(*aspect))
            .collect::<BTreeSet<_>>();
        let weight_ids = SimilarityAspect::ORDER
            .iter()
            .map(|aspect| automation_similarity_aspect_weight_id(*aspect))
            .collect::<BTreeSet<_>>();

        assert_eq!(toggle_ids.len(), SimilarityAspect::ORDER.len());
        assert_eq!(weight_ids.len(), SimilarityAspect::ORDER.len());
        assert!(toggle_ids.is_disjoint(&weight_ids));
    }
}
