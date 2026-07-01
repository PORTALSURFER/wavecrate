use super::super::cells::{
    COLLECTION_MARKER_RIGHT_INSET, LOCKED_KEEP_RATING_COLOR, LOCKED_KEEP_RATING_MARKER_SIDE,
    RATING_MARKER_SIDE, SIMILARITY_ASPECT_DISABLED_TRACK, SIMILARITY_SCORE_FILL,
    muted_sample_file_cell, sample_collection_cell, sample_file_cell, sample_playback_type_cell,
    sample_rating_cell, sample_similarity_cell,
};
use super::super::row_widgets::RatingIndicator;
use super::super::similarity_aspect_color;
use super::*;
use crate::native_app::sample_library::folder_browser::{FolderBrowserState, model::FileEntry};
use radiant::{layout::Vector2, prelude::IntoView, theme::ThemeTokens};
use wavecrate::sample_sources::{Rating, SampleCollection};

/// Builds a representative file entry for row rendering tests.
fn file_entry() -> FileEntry {
    FileEntry {
        id: String::from("C:\\Samples\\portal_SS_kick_003.wav"),
        name: String::from("portal_SS_kick_003.wav"),
        stem: String::from("portal_SS_kick_003"),
        extension: String::from("wav"),
        kind: String::from("Audio"),
        size: String::from("1 KB"),
        size_bytes: 1024,
        modified: String::from("today"),
        modified_rank: 1,
        rating: Rating::NEUTRAL,
        rating_locked: false,
        last_curated_at: None,
        collection: None,
        collections: Vec::new(),
    }
}

#[test]
/// Verifies rating strength maps to the visible indicator count.
fn rating_indicator_count_reflects_rating_strength() {
    assert_eq!(RatingIndicator::new(Rating::NEUTRAL, false).count(), 0);
    assert_eq!(RatingIndicator::new(Rating::KEEP_1, false).count(), 1);
    assert_eq!(RatingIndicator::new(Rating::new(2), false).count(), 2);
    assert_eq!(RatingIndicator::new(Rating::TRASH_3, false).count(), 3);
    assert_eq!(RatingIndicator::new(Rating::KEEP_3, true).count(), 3);
}

#[test]
/// Verifies locked keep ratings use the dedicated keep-marker affordance.
fn locked_keep_rating_uses_locked_keep_marker() {
    assert!(RatingIndicator::new(Rating::KEEP_3, true).shows_locked_keep_marker());
    assert!(!RatingIndicator::new(Rating::KEEP_3, false).shows_locked_keep_marker());
    assert!(!RatingIndicator::new(Rating::TRASH_3, true).shows_locked_keep_marker());
}

#[test]
/// Verifies locked keep rows paint a golden marker instead of a text label.
fn locked_keep_rating_cell_paints_gold_marker_without_text() {
    let mut file = file_entry();
    file.rating = Rating::KEEP_3;
    file.rating_locked = true;
    let theme = ThemeTokens::default();
    let frame = sample_rating_cell(RatingIndicator::new(file.rating, file.rating_locked), 64.0)
        .view_frame_at_size(Vector2::new(64.0, 20.0), &theme);

    assert!(
        !frame.paint_plan.text_runs().any(|run| run.text == "KEEP"),
        "locked keep ratings should no longer paint the KEEP text label"
    );

    let marker = frame
        .paint_plan
        .fill_rects()
        .find(|fill| fill.color == LOCKED_KEEP_RATING_COLOR)
        .expect("locked keep ratings should paint the golden marker");

    assert_eq!(marker.rect.width(), LOCKED_KEEP_RATING_MARKER_SIDE as f32);
    assert_eq!(marker.rect.height(), LOCKED_KEEP_RATING_MARKER_SIDE as f32);
    assert!(
        marker.rect.width() > RATING_MARKER_SIDE as f32,
        "locked keep marker should be larger than normal rating markers"
    );
}

#[test]
/// Verifies available sample names use primary text color without cache-state styling.
fn sample_text_uses_primary_theme_color() {
    let theme = ThemeTokens::default();
    let frame = sample_file_cell(String::from("kick_deep"), 120.0)
        .view_frame_at_size(Vector2::new(120.0, 20.0), &theme);

    assert!(
        frame
            .paint_plan
            .text_runs()
            .any(|run| run.text == "kick_deep" && run.color == theme.text_primary),
        "sample rows should not express loaded/cache state through text color"
    );
}

#[test]
/// Verifies finished harvest rows can demote passive text without heavy row coloring.
fn muted_sample_file_cell_uses_muted_theme_color() {
    let theme = ThemeTokens::default();
    let frame = muted_sample_file_cell(String::from("done_kick"), 120.0)
        .view_frame_at_size(Vector2::new(120.0, 20.0), &theme);

    assert!(
        frame
            .paint_plan
            .text_runs()
            .any(|run| run.text == "done_kick" && run.color == theme.text_muted),
        "done/ignored harvest rows should be able to use subtle muted text styling"
    );
}

#[test]
/// Verifies the empty folder message starts where the item rows would start.
fn empty_folder_message_paints_at_top_of_list_body() {
    let theme = ThemeTokens::default();
    let frame =
        empty_sample_browser_rows(false).view_frame_at_size(Vector2::new(480.0, 240.0), &theme);

    let message = frame
        .paint_plan
        .text_runs()
        .find(|run| run.text == "No audio files in selected folder")
        .expect("empty folder message should paint");

    assert!(
        message.rect.max.y <= SAMPLE_BROWSER_ROW_HEIGHT + 1.0,
        "empty folder message should stay in the first list row, rect={:?}",
        message.rect
    );
}

#[test]
/// Verifies curation mode uses a completion-oriented empty list message.
fn empty_curation_message_says_no_files_are_left_to_curate() {
    let theme = ThemeTokens::default();
    let frame =
        empty_sample_browser_rows(true).view_frame_at_size(Vector2::new(480.0, 240.0), &theme);

    assert!(
        frame
            .paint_plan
            .text_runs()
            .any(|run| run.text == "No files left to curate"),
        "curation empty state should explain that the current curation scope is complete"
    );
}

#[test]
/// Verifies the collection column paints one marker for each collection membership.
fn collection_cell_paints_each_collection_membership_color() {
    let first = SampleCollection::new(0).expect("collection");
    let third = SampleCollection::new(2).expect("collection");
    let mut file = file_entry();
    file.collections = vec![third, first];
    let theme = ThemeTokens::default();
    let folder_browser = FolderBrowserState::load_default();
    let collection_colors = file
        .collection_memberships()
        .into_iter()
        .filter_map(|collection| folder_browser.collection_color(collection))
        .collect::<Vec<_>>();
    let frame = sample_collection_cell(collection_colors, 64.0)
        .view_frame_at_size(Vector2::new(64.0, 20.0), &theme);

    let colors = frame
        .paint_plan
        .fill_rects()
        .map(|fill| fill.color)
        .collect::<Vec<_>>();

    assert!(
        colors.contains(
            &folder_browser
                .collection_color(first)
                .expect("first collection color")
        ),
        "collection column should paint the first collection color"
    );
    assert!(
        colors.contains(
            &folder_browser
                .collection_color(third)
                .expect("third collection color")
        ),
        "collection column should paint the third collection color"
    );
}

#[test]
/// Verifies collection markers remain clipped to their own column cell.
fn collection_cell_keeps_markers_inside_narrow_column_bounds() {
    let collections = [
        SampleCollection::new(0).expect("collection"),
        SampleCollection::new(1).expect("collection"),
        SampleCollection::new(2).expect("collection"),
    ];
    let theme = ThemeTokens::default();
    let folder_browser = FolderBrowserState::load_default();
    let collection_colors = collections
        .into_iter()
        .filter_map(|collection| folder_browser.collection_color(collection))
        .collect::<Vec<_>>();
    let column_width = 24.0;
    let frame = sample_collection_cell(collection_colors.clone(), column_width)
        .view_frame_at_size(Vector2::new(column_width, 20.0), &theme);

    let marker_rects = frame
        .paint_plan
        .fill_rects()
        .filter(|fill| collection_colors.contains(&fill.color))
        .map(|fill| fill.rect)
        .collect::<Vec<_>>();

    assert!(
        !marker_rects.is_empty(),
        "collection cell should paint visible collection markers"
    );
    assert!(
        marker_rects.iter().all(|rect| {
            rect.min.x >= 0.0
                && rect.max.x <= column_width
                && rect.min.y >= 0.0
                && rect.max.y <= 20.0
        }),
        "collection markers should stay inside the collection column bounds: {marker_rects:?}"
    );
}

#[test]
/// Verifies collection markers reserve the header divider gutter.
fn collection_cell_keeps_markers_left_of_header_divider_gutter() {
    let collections = [
        SampleCollection::new(0).expect("collection"),
        SampleCollection::new(1).expect("collection"),
        SampleCollection::new(2).expect("collection"),
    ];
    let theme = ThemeTokens::default();
    let folder_browser = FolderBrowserState::load_default();
    let collection_colors = collections
        .into_iter()
        .filter_map(|collection| folder_browser.collection_color(collection))
        .collect::<Vec<_>>();
    let column_width = 58.0;
    let frame = sample_collection_cell(collection_colors.clone(), column_width)
        .view_frame_at_size(Vector2::new(column_width, 20.0), &theme);

    let max_marker_x = frame
        .paint_plan
        .fill_rects()
        .filter(|fill| collection_colors.contains(&fill.color))
        .map(|fill| fill.rect.max.x)
        .max_by(f32::total_cmp)
        .expect("collection cell should paint visible collection markers");

    assert!(
        max_marker_x <= column_width - COLLECTION_MARKER_RIGHT_INSET as f32,
        "collection markers should end left of the header divider gutter: max_marker_x={max_marker_x}"
    );
}

#[test]
/// Verifies playback-type cells paint compact type labels.
fn playback_type_cell_paints_loop_label() {
    let theme = ThemeTokens::default();
    let frame = sample_playback_type_cell(Some("Loop"), 76.0)
        .view_frame_at_size(Vector2::new(76.0, 20.0), &theme);

    assert!(
        frame.paint_plan.text_runs().any(|run| run.text == "Loop"),
        "loop playback type should paint a compact Loop label"
    );
}

#[test]
/// Verifies unknown playback type stays visually quiet.
fn missing_playback_type_cell_paints_muted_dash() {
    let theme = ThemeTokens::default();
    let frame =
        sample_playback_type_cell(None, 76.0).view_frame_at_size(Vector2::new(76.0, 20.0), &theme);

    assert!(
        frame
            .paint_plan
            .text_runs()
            .any(|run| run.text == "-" && run.color == theme.text_muted),
        "unknown playback type should paint a muted dash"
    );
}

#[test]
/// Verifies similarity scores render as compact progress bars.
fn similarity_score_cell_paints_progress_fill() {
    let theme = ThemeTokens::default();
    let mut aspects =
        crate::native_app::sample_library::folder_browser::model::EMPTY_SIMILARITY_ASPECT_STRENGTHS;
    aspects[wavecrate_analysis::aspects::SimilarityAspect::Spectrum.index()] = Some(0.8);
    let frame = sample_similarity_cell(
        Some(0.65),
        aspects,
        [true; wavecrate_analysis::aspects::ASPECT_COUNT],
        190.0,
    )
    .view_frame_at_size(Vector2::new(190.0, 20.0), &theme);

    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == SIMILARITY_SCORE_FILL && fill.rect.width() > 20.0),
        "available similarity scores should paint a progress fill"
    );
    assert!(
        frame.paint_plan.fill_rects().any(|fill| fill.color
            == similarity_aspect_color(wavecrate_analysis::aspects::SimilarityAspect::Spectrum)
            && fill.rect.width() > 4.0),
        "available aspect scores should paint their compact aspect fill"
    );
}

#[test]
/// Verifies unavailable similarity scores are explicit instead of showing a zero bar.
fn missing_similarity_score_cell_paints_na_label() {
    let theme = ThemeTokens::default();
    let frame = sample_similarity_cell(
        None,
        crate::native_app::sample_library::folder_browser::model::EMPTY_SIMILARITY_ASPECT_STRENGTHS,
        [true; wavecrate_analysis::aspects::ASPECT_COUNT],
        190.0,
    )
    .view_frame_at_size(Vector2::new(190.0, 20.0), &theme);

    assert!(
        frame
            .paint_plan
            .text_runs()
            .any(|run| run.text == "N/A" && run.color == theme.text_muted),
        "missing similarity scores should paint a muted N/A label"
    );
}

#[test]
/// Verifies disabled aspects use the explicit disabled track color instead of an active fill.
fn disabled_similarity_aspect_paints_disabled_track() {
    let theme = ThemeTokens::default();
    let mut aspects =
        crate::native_app::sample_library::folder_browser::model::EMPTY_SIMILARITY_ASPECT_STRENGTHS;
    aspects[wavecrate_analysis::aspects::SimilarityAspect::Pitch.index()] = Some(0.9);
    let mut enabled = [true; wavecrate_analysis::aspects::ASPECT_COUNT];
    enabled[wavecrate_analysis::aspects::SimilarityAspect::Pitch.index()] = false;

    let frame = sample_similarity_cell(Some(0.65), aspects, enabled, 190.0)
        .view_frame_at_size(Vector2::new(190.0, 20.0), &theme);

    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == SIMILARITY_ASPECT_DISABLED_TRACK),
        "disabled aspects should paint the disabled track"
    );
    assert!(
        !frame.paint_plan.fill_rects().any(|fill| fill.color
            == similarity_aspect_color(wavecrate_analysis::aspects::SimilarityAspect::Pitch)
            && fill.rect.width() > 4.0),
        "disabled aspects should not paint their active fill"
    );
}
