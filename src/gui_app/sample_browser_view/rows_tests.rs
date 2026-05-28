
use super::*;
use radiant::{
    gui::types::{Point, Rect},
    layout::{LayoutOutput, Vector2},
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::Widget,
};
use std::collections::HashMap;
use wavecrate::sample_sources::Rating;

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
fn rating_squares_count_reflects_rating_strength() {
    assert_eq!(RatingSquares::new(Rating::NEUTRAL, false).count(), 0);
    assert_eq!(RatingSquares::new(Rating::KEEP_1, false).count(), 1);
    assert_eq!(RatingSquares::new(Rating::new(2), false).count(), 2);
    assert_eq!(RatingSquares::new(Rating::TRASH_3, false).count(), 3);
    assert_eq!(RatingSquares::new(Rating::KEEP_3, true).count(), 3);
}

#[test]
fn unloaded_sample_text_uses_muted_theme_color() {
    let theme = ThemeTokens::default();
    let mut primitives = Vec::new();
    let widget = SampleCellText::new(String::from("kick_deep"), true);

    widget.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 20.0)),
        &LayoutOutput::default(),
        &theme,
    );

    assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Text(run) if run.text == "kick_deep" && run.color == theme.text_muted)),
            "unloaded sample rows should paint text with the muted theme color"
        );
}

#[test]
fn loaded_sample_text_uses_primary_theme_color() {
    let theme = ThemeTokens::default();
    let mut primitives = Vec::new();
    let widget = SampleCellText::new(String::from("kick_deep"), false);

    widget.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 20.0)),
        &LayoutOutput::default(),
        &theme,
    );

    assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Text(run) if run.text == "kick_deep" && run.color == theme.text_primary)),
            "loaded sample rows should paint text with the primary theme color"
        );
}
