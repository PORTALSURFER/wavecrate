use radiant::{
    gui::types::{Point, Rect, Rgba8},
    layout::{LayoutOutput, Vector2},
    prelude as ui,
    runtime::{
        PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintText, PaintTextAlign, PaintTextRun,
    },
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, TextWrap, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use std::collections::HashMap;
use wavecrate::sample_sources::Rating;

use super::{SampleFileHitMessage, SampleFileHitTarget};
use crate::gui_app::{
    GuiMessage, SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_OVERSCAN_ROWS, SAMPLE_BROWSER_ROW_HEIGHT,
    SampleNameViewMode,
    folder_browser::{self, FileColumn, FileEntry, FolderBrowserMessage, FolderBrowserState},
};

pub(super) fn sample_browser_rows(
    folder_browser: &FolderBrowserState,
    files: &[&FileEntry],
    columns: &[&FileColumn],
    window: ui::VirtualListWindow,
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
    suppress_row_hover: bool,
) -> ui::View<GuiMessage> {
    if files.is_empty() {
        return ui::text("No audio files in selected folder")
            .height(24.0)
            .fill_width()
            .fill_height();
    }

    ui::virtual_list_window(
        window,
        SAMPLE_BROWSER_ROW_HEIGHT,
        |index| {
            let file = files[index];
            sample_browser_row(
                file,
                folder_browser.is_file_selected(&file.id),
                folder_browser.file_rename_view(&file.id),
                folder_browser.drag_revision(),
                folder_browser.file_drag_active(),
                folder_browser.file_drag_source(&file.id),
                columns,
                name_view_mode,
                metadata_tags_by_file,
                suppress_row_hover,
            )
        },
        SAMPLE_BROWSER_ROW_HEIGHT * SAMPLE_BROWSER_OVERSCAN_ROWS as f32,
    )
    .id(SAMPLE_BROWSER_LIST_ID)
    .fill()
}

fn sample_browser_row(
    file: &FileEntry,
    selected: bool,
    rename: Option<folder_browser::FileRenameView>,
    drag_revision: u64,
    drag_active: bool,
    drag_source: bool,
    columns: &[&FileColumn],
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
    suppress_row_hover: bool,
) -> ui::View<GuiMessage> {
    let hit_path = file.id.clone();
    let hit_target = sample_file_hit_target(
        file,
        selected,
        drag_revision,
        drag_active,
        drag_source,
        hit_path,
        suppress_row_hover,
    );
    let row = ui::stack([
        hit_target,
        compact_details_row(columns.iter().map(|column| {
            sample_column_cell(
                file,
                rename.clone(),
                column,
                name_view_mode,
                metadata_tags_by_file,
            )
        })),
    ])
    .key(format!("sample-row-{}", file.id))
    .fill_width()
    .height(22.0);
    row.style(ui::WidgetStyle::default())
}

fn sample_file_hit_target(
    file: &FileEntry,
    selected: bool,
    drag_revision: u64,
    drag_active: bool,
    drag_source: bool,
    hit_path: String,
    suppress_hover: bool,
) -> ui::View<GuiMessage> {
    ui::custom_widget_mapped(
        SampleFileHitTarget::new(selected, drag_active, drag_source, suppress_hover),
        move |message| match message {
            SampleFileHitMessage::Activate(modifiers) => GuiMessage::SelectSampleWithModifiers {
                path: hit_path.clone(),
                modifiers,
            },
            SampleFileHitMessage::ContextMenu(position) => GuiMessage::OpenSampleContextMenu {
                path: hit_path.clone(),
                position,
            },
            SampleFileHitMessage::Drag(drag) => GuiMessage::DragSampleFile {
                path: hit_path.clone(),
                drag,
            },
        },
    )
    .key(format!("sample-row-hit-{}-{drag_revision}", file.id))
    .fill_width()
    .height(22.0)
}

fn sample_column_cell(
    file: &FileEntry,
    rename: Option<folder_browser::FileRenameView>,
    column: &FileColumn,
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
) -> ui::View<GuiMessage> {
    if column.id == "name" {
        return sample_name_cell(
            file,
            rename,
            column.width,
            name_view_mode,
            metadata_tags_by_file,
        );
    }
    if column.id == "rating" {
        return sample_rating_cell(file, column.width);
    }
    sample_file_cell(
        file,
        sample_file_column_value(file, column.id.as_str()),
        column.width,
        column.id.as_str(),
    )
}

fn sample_name_cell(
    file: &FileEntry,
    rename: Option<folder_browser::FileRenameView>,
    width: f32,
    name_view_mode: SampleNameViewMode,
    metadata_tags_by_file: &HashMap<String, Vec<String>>,
) -> ui::View<GuiMessage> {
    let Some(rename) = rename else {
        return sample_file_cell(
            file,
            sample_name_cell_value(file, name_view_mode, metadata_tags_by_file),
            width,
            "name",
        );
    };
    ui::text_input(rename.draft)
        .selection(rename.selection_start, rename.selection_end)
        .message_event(|message| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
        })
        .id(rename.input_id)
        .key(format!("sample-rename-input-{}", file.id))
        .width(width)
        .height(20.0)
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
        "path" => file.id.clone(),
        _ => file.stem.clone(),
    }
}

fn sample_rating_cell(file: &FileEntry, width: f32) -> ui::View<GuiMessage> {
    ui::custom_widget(RatingSquares::new(file.rating, file.rating_locked), |_| {
        None
    })
    .key(format!("sample-rating-{}", file.id))
    .height(20.0)
    .width(width)
}

fn sample_file_cell(
    file: &FileEntry,
    value: String,
    width: f32,
    column_id: &str,
) -> ui::View<GuiMessage> {
    ui::text(value)
        .key(format!("sample-{}-{column_id}", file.id))
        .height(20.0)
        .width(width)
        .truncate()
}

fn compact_details_row(
    children: impl IntoIterator<Item = ui::View<GuiMessage>>,
) -> ui::View<GuiMessage> {
    ui::row(children)
        .fill_width()
        .height(22.0)
        .padding_x(8.0)
        .padding_y(1.0)
        .spacing(10.0)
}

#[derive(Clone, Debug)]
struct RatingSquares {
    common: WidgetCommon,
    rating: Rating,
    locked: bool,
}

impl RatingSquares {
    fn new(rating: Rating, locked: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            rating,
            locked,
        }
    }

    fn count(&self) -> usize {
        self.rating.val().unsigned_abs().min(3) as usize
    }

    fn color(&self) -> Option<Rgba8> {
        if self.rating.is_keep() {
            Some(Rgba8 {
                r: 122,
                g: 226,
                b: 96,
                a: 235,
            })
        } else if self.rating.is_trash() {
            Some(Rgba8 {
                r: 238,
                g: 77,
                b: 67,
                a: 235,
            })
        } else {
            None
        }
    }
}

impl Widget for RatingSquares {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        if self.locked && self.rating == Rating::KEEP_3 {
            paint_keep_badge(primitives, self.common.id, bounds);
            return;
        }
        let Some(color) = self.color() else {
            return;
        };
        let count = self.count();
        if count == 0 {
            return;
        }

        let size = 5.0_f32.min(bounds.height().max(0.0));
        let gap = 4.0;
        let total_width = count as f32 * size + count.saturating_sub(1) as f32 * gap;
        let start_x = bounds.max.x - total_width - 4.0;
        let y = bounds.min.y + (bounds.height() - size) * 0.5;
        for index in 0..count {
            let x = start_x + index as f32 * (size + gap);
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(Point::new(x, y), Point::new(x + size, y + size)),
                color,
            }));
        }
    }
}

fn paint_keep_badge(primitives: &mut Vec<PaintPrimitive>, widget_id: u64, bounds: Rect) {
    let badge_width = 38.0_f32.min(bounds.width().max(0.0));
    let badge_height = 14.0_f32.min(bounds.height().max(0.0));
    if badge_width <= 0.0 || badge_height <= 0.0 {
        return;
    }
    let x = bounds.max.x - badge_width - 2.0;
    let y = bounds.min.y + (bounds.height() - badge_height) * 0.5;
    let rect = Rect::from_min_max(
        Point::new(x, y),
        Point::new(x + badge_width, y + badge_height),
    );
    let gold = Rgba8 {
        r: 221,
        g: 177,
        b: 54,
        a: 245,
    };
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color: Rgba8 {
            r: 54,
            g: 43,
            b: 14,
            a: 170,
        },
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color: gold,
        width: 1.0,
    }));
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: PaintText::from("KEEP"),
        rect,
        font_size: 9.0,
        baseline: Some(((rect.height() - 9.0) * 0.5 + 9.0 * 0.78).round()),
        color: gold,
        align: PaintTextAlign::Center,
        wrap: TextWrap::None,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
