use super::projection::{
    RandomNavigationButtonProjection, SampleNameViewModeButtonProjection,
    SampleSimilarityControlsProjection, SampleSimilarityHeaderProjection, projected_header_columns,
};
use crate::native_app::app::SampleNameViewMode;
use crate::native_app::sample_library::folder_browser::model::FileColumn;
use wavecrate::sample_sources::config::SimilarityAspectSettings;
use wavecrate_analysis::aspects::SimilarityAspect;

#[test]
fn sample_name_view_mode_projection_names_current_mode() {
    assert_eq!(
        SampleNameViewModeButtonProjection::from_mode(SampleNameViewMode::DiskFilename).label,
        "Disk"
    );
    assert_eq!(
        SampleNameViewModeButtonProjection::from_mode(SampleNameViewMode::MetadataLabel).label,
        "Label"
    );
}

#[test]
fn header_button_projections_keep_product_tooltips() {
    let random_navigation = RandomNavigationButtonProjection::new(true);
    let name_mode = SampleNameViewModeButtonProjection::from_mode(SampleNameViewMode::DiskFilename);

    assert!(random_navigation.active);
    assert_eq!(
        random_navigation.tooltip,
        "Random audition within the selected folder or active filter."
    );
    assert_eq!(
        name_mode.tooltip,
        "Switch sample names between disk filenames and metadata labels."
    );
}

#[test]
fn header_column_projection_inserts_similarity_after_name_column_only() {
    let name = FileColumn::for_tests("name", "Name", 240.0);
    let size = FileColumn::for_tests("size", "Size", 78.0);
    let columns = [&name, &size];

    let active = projected_header_columns(&columns, true);
    assert_eq!(active.len(), 2);
    assert_eq!(active[0].column.id, "name");
    assert!(active[0].show_similarity_after);
    assert_eq!(active[1].column.id, "size");
    assert!(!active[1].show_similarity_after);

    let inactive = projected_header_columns(&columns, false);
    assert!(inactive.iter().all(|column| !column.show_similarity_after));
}

#[test]
fn similarity_control_projection_preserves_order_labels_and_state() {
    let mut settings = SimilarityAspectSettings::default();
    settings.set_weighting_enabled(true);
    settings.set_aspect_enabled(SimilarityAspect::Pitch, false);
    settings.set_aspect_weight(SimilarityAspect::Spectrum, 0.35);

    let projection = SampleSimilarityControlsProjection::from_settings(&settings);

    assert_eq!(projection.weighting_label, "Weight");
    assert!(projection.weighting_enabled);
    assert_eq!(
        projection
            .aspects
            .iter()
            .map(|control| control.label)
            .collect::<Vec<_>>(),
        ["O", "S", "T", "P", "A"]
    );
    assert_eq!(
        projection.aspects[SimilarityAspect::Pitch.index()].aspect,
        SimilarityAspect::Pitch
    );
    assert!(!projection.aspects[SimilarityAspect::Pitch.index()].enabled);
    assert_eq!(
        projection.aspects[SimilarityAspect::Spectrum.index()].weight,
        0.35
    );
}

#[test]
fn similarity_header_projection_marks_disabled_aspects() {
    let mut settings = SimilarityAspectSettings::default();
    settings.set_aspect_enabled(SimilarityAspect::Amplitude, false);

    let projection = SampleSimilarityHeaderProjection::from_settings(&settings);

    assert_eq!(projection.score_label, "Sim");
    assert_eq!(
        projection
            .aspects
            .iter()
            .map(|aspect| aspect.label)
            .collect::<Vec<_>>(),
        ["O", "S", "T", "P", "A"]
    );
    assert!(!projection.aspects[SimilarityAspect::Amplitude.index()].enabled);
}
