use super::SampleBrowserHeaderBar;
use super::projection::{
    RandomNavigationButtonProjection, STARMAP_VIEW_TOOLTIP, SampleBrowserHeaderProjection,
    SampleNameViewModeButtonProjection, SampleSimilarityControlsProjection,
    SampleSimilarityHeaderProjection, StarmapViewButtonProjection, projected_header_columns,
};
use crate::native_app::app::SampleNameViewMode;
use crate::native_app::sample_library::folder_browser::model::FileColumn;
use crate::native_app::sample_library::folder_browser::projection::FileColumnDragFeedback;
use radiant::prelude as ui;
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
    let map_view = StarmapViewButtonProjection::new(false);
    let name_mode = SampleNameViewModeButtonProjection::from_mode(SampleNameViewMode::DiskFilename);

    assert!(random_navigation.active);
    assert_eq!(
        random_navigation.tooltip,
        "Random audition within the selected folder or active filter."
    );
    assert!(!map_view.active);
    assert_eq!(map_view.tooltip, STARMAP_VIEW_TOOLTIP);
    assert_eq!(
        name_mode.tooltip,
        "Switch sample names between disk filenames and metadata labels."
    );
}

#[test]
fn starmap_view_button_projection_tracks_mode_switch_state() {
    let inactive = StarmapViewButtonProjection::new(false);
    let active = StarmapViewButtonProjection::new(true);

    assert!(!inactive.active);
    assert!(active.active);
    assert_eq!(inactive.tooltip, "Switch between list and Starmap views.");
    assert_eq!(active.tooltip, inactive.tooltip);
}

#[test]
fn header_column_projection_inserts_similarity_after_name_column_only() {
    let name = FileColumn::for_tests("name", "Name", 240.0);
    let size = FileColumn::for_tests("size", "Size", 78.0);
    let columns = [&name, &size];

    let active = projected_header_columns(&columns, true, false);
    assert_eq!(active.len(), 2);
    assert_eq!(active[0].column.id, "name");
    assert!(active[0].show_similarity_after);
    assert_eq!(active[1].column.id, "size");
    assert!(!active[1].show_similarity_after);

    let inactive = projected_header_columns(&columns, false, false);
    assert!(inactive.iter().all(|column| !column.show_similarity_after));

    let map_mode = projected_header_columns(&columns, true, true);
    assert!(
        map_mode.is_empty(),
        "map mode should hide list-table column headers"
    );
}

#[test]
fn header_bar_projection_collects_controls_columns_and_drag_marker() {
    let name = FileColumn::for_tests("name", "Name", 240.0);
    let size = FileColumn::for_tests("size", "Size", 78.0);
    let columns = [&name, &size];
    let sort = ui::DetailsSort::new("name", ui::SortDirection::Ascending);
    let drag_feedback = FileColumnDragFeedback {
        label: "Name".to_string(),
        pointer: ui::Point::new(42.0, 9.0),
        width: 240.0,
        marker_x: 138.0,
    };
    let settings = SimilarityAspectSettings::default();

    let projection = SampleBrowserHeaderProjection::from_model(SampleBrowserHeaderBar {
        columns: &columns,
        sort: &sort,
        drag_feedback: Some(&drag_feedback),
        mode: SampleNameViewMode::MetadataLabel,
        random_navigation_enabled: true,
        map_view_active: false,
        similarity_mode_active: true,
        similarity_controls: &settings,
        help_tooltips_enabled: true,
    });

    assert_eq!(projection.sort.column_id, "name");
    assert_eq!(projection.drag_marker_x, Some(138.0));
    assert!(projection.random_navigation.active);
    assert!(!projection.map_view.active);
    assert_eq!(projection.name_view_mode.label, "Label");
    assert!(projection.help_tooltips_enabled);
    assert_eq!(projection.columns.len(), 2);
    assert!(projection.columns[0].show_similarity_after);
    assert!(!projection.columns[1].show_similarity_after);
    assert_eq!(projection.similarity_header.score_label, "Sim");
}

#[test]
fn header_bar_projection_hides_list_columns_in_map_mode() {
    let name = FileColumn::for_tests("name", "Name", 240.0);
    let size = FileColumn::for_tests("size", "Size", 78.0);
    let columns = [&name, &size];
    let sort = ui::DetailsSort::new("name", ui::SortDirection::Ascending);
    let drag_feedback = FileColumnDragFeedback {
        label: "Name".to_string(),
        pointer: ui::Point::new(42.0, 9.0),
        width: 240.0,
        marker_x: 138.0,
    };
    let settings = SimilarityAspectSettings::default();

    let projection = SampleBrowserHeaderProjection::from_model(SampleBrowserHeaderBar {
        columns: &columns,
        sort: &sort,
        drag_feedback: Some(&drag_feedback),
        mode: SampleNameViewMode::DiskFilename,
        random_navigation_enabled: false,
        map_view_active: true,
        similarity_mode_active: true,
        similarity_controls: &settings,
        help_tooltips_enabled: false,
    });

    assert!(projection.columns.is_empty());
    assert_eq!(projection.drag_marker_x, None);
    assert!(projection.map_view.active);
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
