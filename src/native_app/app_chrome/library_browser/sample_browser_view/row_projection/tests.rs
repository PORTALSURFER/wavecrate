use super::*;
use crate::native_app::sample_library::folder_browser::model::EMPTY_SIMILARITY_ASPECT_STRENGTHS;
use crate::native_app::sample_library::folder_browser::projection::VisibleSampleRow;
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
        collections: Vec::new(),
    }
}

fn visible_row(file: &FileEntry) -> VisibleSampleRow<'_> {
    VisibleSampleRow {
        file,
        selected: false,
        copy_flash: false,
        drag_active: false,
        drag_source: false,
        cached: false,
        missing: false,
        rename: None,
        similarity_anchor: false,
        similarity_strength: None,
        similarity_aspect_strengths: EMPTY_SIMILARITY_ASPECT_STRENGTHS,
        collection_colors: Vec::new(),
        source_folder_path: String::from("drums/kicks"),
    }
}

fn column_display<'a>(
    file: &'a FileEntry,
    row: &VisibleSampleRow<'_>,
    column: &'a FileColumn,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
) -> SampleColumnDisplay<'a> {
    sample_column_display(
        file,
        row,
        column,
        SampleNameViewMode::DiskFilename,
        metadata_tags_by_file,
    )
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
fn sample_collection_projection_uses_collection_colors() {
    let file = file_entry();
    let mut row = visible_row(&file);
    row.copy_flash = true;
    row.collection_colors = vec![ui::Rgba8::new(1, 2, 3, 255), ui::Rgba8::new(4, 5, 6, 255)];
    let column = FileColumn::for_tests("collection", "Collection", 80.0);

    let display = column_display(&file, &row, &column, &HashMap::new());

    assert!(
        sample_row_display(
            &row,
            &[&column],
            false,
            [true; wavecrate_analysis::aspects::ASPECT_COUNT],
            SampleNameViewMode::DiskFilename,
            &HashMap::new(),
        )
        .copy_flash
    );
    assert!(matches!(
        display.content,
        SampleColumnContent::Collection(colors)
            if colors == vec![ui::Rgba8::new(1, 2, 3, 255), ui::Rgba8::new(4, 5, 6, 255)]
    ));
}

#[test]
fn sample_source_folder_projection_uses_row_folder_path_without_cache_state() {
    let file = file_entry();
    let mut row = visible_row(&file);
    row.cached = true;
    let column = FileColumn::for_tests("source_folder", "Folder", 160.0);

    let display = column_display(&file, &row, &column, &HashMap::new());
    let row_display = sample_row_display(
        &row,
        &[&column],
        false,
        [true; wavecrate_analysis::aspects::ASPECT_COUNT],
        SampleNameViewMode::DiskFilename,
        &HashMap::new(),
    );

    assert!(matches!(display.content, SampleColumnContent::Text(value) if value == "drums/kicks"));
    assert!(
        row_display.cached,
        "loaded/cache state belongs to the row hit-target projection, not text cells"
    );
}

#[test]
fn sample_playback_type_projection_uses_metadata_tags() {
    let file = file_entry();
    let row = visible_row(&file);
    let column = FileColumn::for_tests("playback_type", "Type", 76.0);
    let metadata_tags_by_file = HashMap::from([(file.id.clone(), vec![String::from("one-shot")])]);

    let display = column_display(&file, &row, &column, &metadata_tags_by_file);

    assert!(matches!(
        display.content,
        SampleColumnContent::PlaybackType(Some("One-shot"))
    ));
}

#[test]
fn sample_playback_type_projection_handles_unknown_tags() {
    let file = file_entry();
    let row = visible_row(&file);
    let column = FileColumn::for_tests("playback_type", "Type", 76.0);

    let display = column_display(&file, &row, &column, &HashMap::new());

    assert!(matches!(
        display.content,
        SampleColumnContent::PlaybackType(None)
    ));
}
