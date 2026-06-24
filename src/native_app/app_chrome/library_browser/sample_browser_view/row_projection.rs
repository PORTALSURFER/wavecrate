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
    pub(super) cut_pending: bool,
    pub(super) drag_active: bool,
    pub(super) drag_source: bool,
    pub(super) cached: bool,
    pub(super) missing: bool,
    pub(super) similarity_anchor: bool,
    pub(super) similarity_strength: Option<f32>,
    pub(super) columns: Vec<SampleColumnDisplay>,
}

/// Product projection for one visible sample-browser cell.
pub(super) struct SampleColumnDisplay {
    pub(super) width: f32,
    pub(super) content: SampleColumnContent,
}

pub(super) enum SampleColumnContent {
    Text(String),
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
    cut_file_ids: Option<&[String]>,
) -> SampleRowDisplay<'a> {
    let file = row.file;
    SampleRowDisplay {
        file_id: file.id.as_str(),
        selected: row.selected,
        copy_flash: row.copy_flash,
        cut_pending: cut_file_ids.is_some_and(|ids| ids.iter().any(|id| id == &file.id)),
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
) -> Vec<SampleColumnDisplay> {
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
            displays.push(similarity_column_display(row, similarity_aspect_enabled));
        }
    }
    displays
}

/// Build the synthetic similarity column displayed after the sample name.
fn similarity_column_display(
    row: &VisibleSampleRow<'_>,
    aspect_enabled: [bool; wavecrate_analysis::aspects::ASPECT_COUNT],
) -> SampleColumnDisplay {
    SampleColumnDisplay {
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
) -> SampleColumnDisplay {
    let content = match column.kind() {
        FileColumnKind::Name => row.rename.clone().map_or_else(
            || {
                SampleColumnContent::Text(sample_name_cell_value(
                    file,
                    name_view_mode,
                    metadata_tags_by_file,
                ))
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
        FileColumnKind::SourceFolder => SampleColumnContent::Text(row.source_folder_path.clone()),
        kind => SampleColumnContent::Text(sample_file_column_value(file, kind)),
    };
    SampleColumnDisplay {
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
mod tests;
