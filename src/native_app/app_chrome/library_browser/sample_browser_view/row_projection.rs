use radiant::prelude as ui;
use std::collections::HashMap;

use super::SAMPLE_SIMILARITY_SCORE_COLUMN_WIDTH;
use super::row_widgets::RatingIndicator;
use crate::native_app::app::SampleNameViewMode;
use crate::native_app::audio::playback::tagged_playback_mode_for_tags;
use crate::native_app::sample_library::folder_browser::commands::FileRenameView;
use crate::native_app::sample_library::folder_browser::model::{
    FileColumn, FileColumnKind, FileEntry, SimilarityAspectStrengths,
};
use crate::native_app::sample_library::folder_browser::projection::VisibleSampleRow;
use crate::native_app::sample_library::folder_browser::view_contract::collection_hotkey;

pub(super) struct SampleRowDisplay<'a> {
    pub(super) file_id: &'a str,
    pub(super) selected: bool,
    pub(super) copy_flash: bool,
    pub(super) drag_revision: u64,
    pub(super) drag_active: bool,
    pub(super) drag_source: bool,
    pub(super) cached: bool,
    pub(super) missing: bool,
    pub(super) similarity_anchor: bool,
    pub(super) similarity_strength: Option<f32>,
    pub(super) columns: Vec<SampleColumnDisplay<'a>>,
}

pub(super) struct SampleColumnDisplay<'a> {
    pub(super) file_id: &'a str,
    pub(super) id: &'a str,
    pub(super) width: f32,
    pub(super) content: SampleColumnContent,
}

pub(super) enum SampleColumnContent {
    Text {
        value: String,
        cached: bool,
    },
    Rename(FileRenameView),
    Rating(RatingIndicator),
    PlaybackType(Option<&'static str>),
    Collection(Vec<ui::Rgba8>),
    Similarity {
        overall: Option<f32>,
        aspects: SimilarityAspectStrengths,
        aspect_enabled: [bool; wavecrate_analysis::aspects::ASPECT_COUNT],
    },
}

pub(super) fn sample_row_display<'a>(
    row: &'a VisibleSampleRow<'a>,
    columns: &[&'a FileColumn],
    similarity_mode_active: bool,
    similarity_aspect_enabled: [bool; wavecrate_analysis::aspects::ASPECT_COUNT],
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
) -> SampleRowDisplay<'a> {
    let file = row.file;
    SampleRowDisplay {
        file_id: file.id.as_str(),
        selected: row.selected,
        copy_flash: row.copy_flash,
        drag_revision: row.drag_revision,
        drag_active: row.drag_active,
        drag_source: row.drag_source,
        cached: row.cached,
        missing: row.missing,
        similarity_anchor: row.similarity_anchor,
        similarity_strength: row.similarity_strength,
        columns: sample_column_displays(
            file,
            row,
            columns,
            similarity_mode_active,
            similarity_aspect_enabled,
            name_view_mode,
            metadata_tags_by_file,
        ),
    }
}

fn sample_column_displays<'a>(
    file: &'a FileEntry,
    row: &'a VisibleSampleRow<'a>,
    columns: &[&'a FileColumn],
    similarity_mode_active: bool,
    similarity_aspect_enabled: [bool; wavecrate_analysis::aspects::ASPECT_COUNT],
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
) -> Vec<SampleColumnDisplay<'a>> {
    let mut displays = Vec::with_capacity(columns.len() + 1);
    for column in columns {
        displays.push(sample_column_display(
            file,
            row,
            column,
            name_view_mode,
            metadata_tags_by_file,
        ));
        if column.kind() == FileColumnKind::Name && similarity_mode_active {
            displays.push(similarity_column_display(
                file,
                row,
                similarity_aspect_enabled,
            ));
        }
    }
    displays
}

fn similarity_column_display<'a>(
    file: &'a FileEntry,
    row: &VisibleSampleRow<'_>,
    aspect_enabled: [bool; wavecrate_analysis::aspects::ASPECT_COUNT],
) -> SampleColumnDisplay<'a> {
    SampleColumnDisplay {
        file_id: file.id.as_str(),
        id: "similarity",
        width: SAMPLE_SIMILARITY_SCORE_COLUMN_WIDTH,
        content: SampleColumnContent::Similarity {
            overall: row.similarity_strength,
            aspects: row.similarity_aspect_strengths,
            aspect_enabled,
        },
    }
}

fn sample_column_display<'a>(
    file: &'a FileEntry,
    row: &VisibleSampleRow<'_>,
    column: &'a FileColumn,
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
) -> SampleColumnDisplay<'a> {
    let content = match column.kind() {
        FileColumnKind::Name => row.rename.clone().map_or_else(
            || SampleColumnContent::Text {
                value: sample_name_cell_value(file, name_view_mode, metadata_tags_by_file),
                cached: row.cached,
            },
            SampleColumnContent::Rename,
        ),
        FileColumnKind::Rating => {
            SampleColumnContent::Rating(RatingIndicator::new(file.rating, file.rating_locked))
        }
        FileColumnKind::PlaybackType => SampleColumnContent::PlaybackType(
            tagged_playback_mode_for_tags(metadata_tags_by_file.get(&file.id).map(Vec::as_slice))
                .map(|mode| mode.label()),
        ),
        FileColumnKind::Collection => {
            SampleColumnContent::Collection(row.collection_colors.clone())
        }
        FileColumnKind::SourceFolder => SampleColumnContent::Text {
            value: row.source_folder_path.clone(),
            cached: row.cached,
        },
        kind => SampleColumnContent::Text {
            value: sample_file_column_value(file, kind),
            cached: row.cached,
        },
    };
    SampleColumnDisplay {
        file_id: file.id.as_str(),
        id: column.id.as_str(),
        width: column.width,
        content,
    }
}

pub(super) fn sample_name_cell_value(
    file: &FileEntry,
    mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
) -> String {
    match mode {
        SampleNameViewMode::DiskFilename => file.stem.clone(),
        SampleNameViewMode::MetadataLabel => {
            metadata_display_stem(file, metadata_tags_by_file.get(&file.id).map(Vec::as_slice))
        }
    }
}

fn metadata_display_stem(file: &FileEntry, metadata_tags: Option<&[String]>) -> String {
    let display = metadata_tags
        .unwrap_or(&[])
        .iter()
        .filter(|tag| !tag.is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("_");
    if display.is_empty() {
        file.stem.clone()
    } else {
        display
    }
}

fn sample_file_column_value(file: &FileEntry, kind: FileColumnKind) -> String {
    match kind {
        FileColumnKind::Extension => file.extension.clone(),
        FileColumnKind::Size => file.size.clone(),
        FileColumnKind::Modified => file.modified.clone(),
        FileColumnKind::Kind => file.kind.clone(),
        FileColumnKind::Collection => file
            .collection_memberships()
            .into_iter()
            .map(collection_hotkey)
            .map(|hotkey| hotkey.to_string())
            .collect::<Vec<_>>()
            .join(","),
        FileColumnKind::Path => file.id.clone(),
        FileColumnKind::Name
        | FileColumnKind::Rating
        | FileColumnKind::PlaybackType
        | FileColumnKind::SourceFolder
        | FileColumnKind::Similarity => file.stem.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let row = VisibleSampleRow {
            file: &file,
            selected: false,
            copy_flash: true,
            drag_revision: 0,
            drag_active: false,
            drag_source: false,
            cached: false,
            missing: false,
            rename: None,
            similarity_anchor: false,
            similarity_strength: None,
            similarity_aspect_strengths:
                crate::native_app::sample_library::folder_browser::model::EMPTY_SIMILARITY_ASPECT_STRENGTHS,
            collection_colors: vec![ui::Rgba8::new(1, 2, 3, 255), ui::Rgba8::new(4, 5, 6, 255)],
            source_folder_path: String::from("drums/kicks"),
        };
        let column = FileColumn::for_tests("collection", "Collection", 80.0);

        let display = sample_column_display(
            &file,
            &row,
            &column,
            SampleNameViewMode::DiskFilename,
            &HashMap::new(),
        );

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
    fn sample_source_folder_projection_uses_row_folder_path() {
        let file = file_entry();
        let row = VisibleSampleRow {
            file: &file,
            selected: false,
            copy_flash: false,
            drag_revision: 0,
            drag_active: false,
            drag_source: false,
            cached: true,
            missing: false,
            rename: None,
            similarity_anchor: false,
            similarity_strength: None,
            similarity_aspect_strengths:
                crate::native_app::sample_library::folder_browser::model::EMPTY_SIMILARITY_ASPECT_STRENGTHS,
            collection_colors: Vec::new(),
            source_folder_path: String::from("drums/kicks"),
        };
        let column = FileColumn::for_tests("source_folder", "Folder", 160.0);

        let display = sample_column_display(
            &file,
            &row,
            &column,
            SampleNameViewMode::DiskFilename,
            &HashMap::new(),
        );

        assert!(matches!(
            display.content,
            SampleColumnContent::Text { value, cached: true } if value == "drums/kicks"
        ));
    }

    #[test]
    fn sample_playback_type_projection_uses_metadata_tags() {
        let file = file_entry();
        let row = VisibleSampleRow {
            file: &file,
            selected: false,
            copy_flash: false,
            drag_revision: 0,
            drag_active: false,
            drag_source: false,
            cached: false,
            missing: false,
            rename: None,
            similarity_anchor: false,
            similarity_strength: None,
            similarity_aspect_strengths:
                crate::native_app::sample_library::folder_browser::model::EMPTY_SIMILARITY_ASPECT_STRENGTHS,
            collection_colors: Vec::new(),
            source_folder_path: String::from("drums/kicks"),
        };
        let column = FileColumn::for_tests("playback_type", "Type", 76.0);
        let metadata_tags_by_file =
            HashMap::from([(file.id.clone(), vec![String::from("one-shot")])]);

        let display = sample_column_display(
            &file,
            &row,
            &column,
            SampleNameViewMode::DiskFilename,
            &metadata_tags_by_file,
        );

        assert!(matches!(
            display.content,
            SampleColumnContent::PlaybackType(Some("One-shot"))
        ));
    }

    #[test]
    fn sample_playback_type_projection_handles_unknown_tags() {
        let file = file_entry();
        let row = VisibleSampleRow {
            file: &file,
            selected: false,
            copy_flash: false,
            drag_revision: 0,
            drag_active: false,
            drag_source: false,
            cached: false,
            missing: false,
            rename: None,
            similarity_anchor: false,
            similarity_strength: None,
            similarity_aspect_strengths:
                crate::native_app::sample_library::folder_browser::model::EMPTY_SIMILARITY_ASPECT_STRENGTHS,
            collection_colors: Vec::new(),
            source_folder_path: String::from("drums/kicks"),
        };
        let column = FileColumn::for_tests("playback_type", "Type", 76.0);

        let display = sample_column_display(
            &file,
            &row,
            &column,
            SampleNameViewMode::DiskFilename,
            &HashMap::new(),
        );

        assert!(matches!(
            display.content,
            SampleColumnContent::PlaybackType(None)
        ));
    }
}
