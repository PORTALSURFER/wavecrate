use super::*;
use radiant::{
    gui::types::{Point, Rect},
    layout::Vector2,
    prelude::IntoView,
    theme::ThemeTokens,
};
use std::collections::HashMap;
use wavecrate::sample_sources::Rating;

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
        collection: None,
    }
}

#[test]
/// Verifies disk-filename mode shows the file stem.
fn disk_filename_view_uses_file_stem() {
    assert_eq!(
        sample_name_cell_value(
            &file_entry(),
            SampleNameViewMode::DiskFilename,
            &HashMap::new()
        ),
        "portal_SS_kick_003"
    );
}

#[test]
/// Verifies metadata-label mode uses joined metadata tags.
fn metadata_label_view_uses_file_metadata_tag_stem_without_extension() {
    let file = file_entry();
    let metadata_tags_by_file = HashMap::from([(
        file.id.clone(),
        vec![String::from("kick"), String::from("warm")],
    )]);

    assert_eq!(
        sample_name_cell_value(
            &file,
            SampleNameViewMode::MetadataLabel,
            &metadata_tags_by_file
        ),
        "kick_warm"
    );
}

#[test]
/// Verifies metadata-label mode falls back to the file stem.
fn metadata_label_view_falls_back_to_file_stem_without_file_tags() {
    let metadata_tags_by_file = HashMap::from([(
        String::from("C:\\Samples\\other.wav"),
        vec![String::from("kick")],
    )]);

    assert_eq!(
        sample_name_cell_value(
            &file_entry(),
            SampleNameViewMode::MetadataLabel,
            &metadata_tags_by_file
        ),
        "portal_SS_kick_003"
    );
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
/// Verifies locked keep ratings use the keep badge affordance.
fn locked_keep_rating_uses_keep_badge() {
    assert!(RatingIndicator::new(Rating::KEEP_3, true).shows_keep_badge());
    assert!(!RatingIndicator::new(Rating::KEEP_3, false).shows_keep_badge());
    assert!(!RatingIndicator::new(Rating::TRASH_3, true).shows_keep_badge());
}

#[test]
/// Verifies locked keep rows paint the keep badge label.
fn locked_keep_rating_cell_paints_keep_badge_text() {
    let mut file = file_entry();
    file.rating = Rating::KEEP_3;
    file.rating_locked = true;
    let theme = ThemeTokens::default();
    let frame = radiant::runtime::UiSurface::new(sample_rating_cell(&file, 64.0).into_node())
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(64.0, 20.0)),
            &theme,
        );

    assert!(
        frame.paint_plan.text_runs().any(|run| run.text == "KEEP"),
        "locked keep ratings should paint the KEEP badge label"
    );
}

#[test]
/// Verifies unloaded sample names use muted text color.
fn unloaded_sample_text_uses_muted_theme_color() {
    let theme = ThemeTokens::default();
    let frame = radiant::runtime::UiSurface::new(
        sample_file_cell(
            &file_entry(),
            String::from("kick_deep"),
            120.0,
            "name",
            false,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 20.0)),
        &theme,
    );

    assert!(
        frame
            .paint_plan
            .text_runs()
            .any(|run| run.text == "kick_deep" && run.color == theme.text_muted),
        "unloaded sample rows should paint text with the muted theme color"
    );
}

#[test]
/// Verifies loaded sample names use primary text color.
fn loaded_sample_text_uses_primary_theme_color() {
    let theme = ThemeTokens::default();
    let frame = radiant::runtime::UiSurface::new(
        sample_file_cell(
            &file_entry(),
            String::from("kick_deep"),
            120.0,
            "name",
            true,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 20.0)),
        &theme,
    );

    assert!(
        frame
            .paint_plan
            .text_runs()
            .any(|run| run.text == "kick_deep" && run.color == theme.text_primary),
        "loaded sample rows should paint text with the primary theme color"
    );
}
