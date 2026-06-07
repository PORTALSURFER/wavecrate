use radiant::prelude as ui;
use std::collections::HashMap;

use super::row_widgets::RatingIndicator;
use crate::native_app::app::SampleNameViewMode;
use crate::native_app::sample_library::folder_browser::{
    self, FileColumn, FileEntry, FolderBrowserState,
};

pub(super) struct SampleRowDisplay<'a> {
    pub(super) file_id: &'a str,
    pub(super) selected: bool,
    pub(super) drag_revision: u64,
    pub(super) drag_active: bool,
    pub(super) drag_source: bool,
    pub(super) cached: bool,
    pub(super) suppress_row_hover: bool,
    pub(super) columns: Vec<SampleColumnDisplay<'a>>,
}

pub(super) struct SampleColumnDisplay<'a> {
    pub(super) file_id: &'a str,
    pub(super) id: &'a str,
    pub(super) width: f32,
    pub(super) content: SampleColumnContent,
}

pub(super) enum SampleColumnContent {
    Text { value: String, cached: bool },
    Rename(folder_browser::FileRenameView),
    Rating(RatingIndicator),
    Collection(Vec<ui::Rgba8>),
}

pub(super) fn sample_row_display<'a>(
    file: &'a FileEntry,
    folder_browser: &FolderBrowserState,
    columns: &[&'a FileColumn],
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
    cached: bool,
    suppress_row_hover: bool,
) -> SampleRowDisplay<'a> {
    let rename = folder_browser.file_rename_view(&file.id);
    SampleRowDisplay {
        file_id: file.id.as_str(),
        selected: folder_browser.is_file_selected(&file.id),
        drag_revision: folder_browser.drag_revision(),
        drag_active: folder_browser.file_drag_active(),
        drag_source: folder_browser.file_drag_source(&file.id),
        cached,
        suppress_row_hover,
        columns: columns
            .iter()
            .map(|column| {
                sample_column_display(
                    file,
                    rename.clone(),
                    column,
                    folder_browser,
                    name_view_mode,
                    metadata_tags_by_file,
                    cached,
                )
            })
            .collect(),
    }
}

fn sample_column_display<'a>(
    file: &'a FileEntry,
    rename: Option<folder_browser::FileRenameView>,
    column: &'a FileColumn,
    folder_browser: &FolderBrowserState,
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
    cached: bool,
) -> SampleColumnDisplay<'a> {
    let content = match column.id.as_str() {
        "name" => rename.map_or_else(
            || SampleColumnContent::Text {
                value: sample_name_cell_value(file, name_view_mode, metadata_tags_by_file),
                cached,
            },
            SampleColumnContent::Rename,
        ),
        "rating" => {
            SampleColumnContent::Rating(RatingIndicator::new(file.rating, file.rating_locked))
        }
        "collection" => {
            SampleColumnContent::Collection(sample_collection_colors(file, folder_browser))
        }
        column_id => SampleColumnContent::Text {
            value: sample_file_column_value(file, column_id),
            cached,
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

fn sample_file_column_value(file: &FileEntry, column_id: &str) -> String {
    match column_id {
        "extension" => file.extension.clone(),
        "size" => file.size.clone(),
        "modified" => file.modified.clone(),
        "kind" => file.kind.clone(),
        "collection" => file
            .collection_memberships()
            .into_iter()
            .map(folder_browser::collection_hotkey)
            .map(|hotkey| hotkey.to_string())
            .collect::<Vec<_>>()
            .join(","),
        "path" => file.id.clone(),
        _ => file.stem.clone(),
    }
}

fn sample_collection_colors(
    file: &FileEntry,
    folder_browser: &FolderBrowserState,
) -> Vec<ui::Rgba8> {
    file.collection_memberships()
        .into_iter()
        .filter_map(|collection| folder_browser.collection_color(collection))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::sample_library::folder_browser::FolderBrowserState;
    use wavecrate::sample_sources::{Rating, SampleCollection};

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
        let first = SampleCollection::new(0).expect("collection");
        let third = SampleCollection::new(2).expect("collection");
        let mut file = file_entry();
        file.collections = vec![third, first];
        let folder_browser = FolderBrowserState::load_default();

        assert_eq!(
            sample_collection_colors(&file, &folder_browser),
            vec![
                folder_browser.collection_color(first).expect("first color"),
                folder_browser.collection_color(third).expect("third color"),
            ]
        );
    }
}
