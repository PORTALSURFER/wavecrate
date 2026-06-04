use radiant::{
    gui::types::{Point, Rect},
    layout::{LayoutOutput, Vector2},
    runtime::{PaintFillRect, PaintPrimitive},
    theme::ThemeTokens,
    widgets::{PaintBounds, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing},
};

use super::{TREE_DEPTH_INDENT, TREE_ROW_HEIGHT, VisibleFolder};

const GUIDE_OVERLAY_WIDGET_ID: u64 = 0x7776_6372_6174_6502;
const GUIDE_WIDTH: f32 = 1.0;
const GUIDE_END_GAP: f32 = 5.0;
const GUIDE_COLOR: radiant::gui::types::Rgba8 = radiant::gui::types::Rgba8 {
    r: 255,
    g: 126,
    b: 64,
    a: 152,
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct FolderTreeGuideSegment {
    level: usize,
    start_row: usize,
    end_row_exclusive: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FolderTreeGuideOverlay {
    common: WidgetCommon,
    first_row: usize,
    row_count: usize,
    segments: Vec<FolderTreeGuideSegment>,
}

impl FolderTreeGuideOverlay {
    fn new(first_row: usize, row_count: usize, segments: Vec<FolderTreeGuideSegment>) -> Self {
        let mut common = WidgetCommon::new(
            GUIDE_OVERLAY_WIDGET_ID,
            WidgetSizing::fixed(Vector2::new(0.0, (row_count as f32) * TREE_ROW_HEIGHT)),
        )
        .without_default_chrome();
        common.paint.bounds = PaintBounds::AllowOverflow;
        common.state.disabled = true;
        Self {
            common,
            first_row,
            row_count,
            segments,
        }
    }
}

impl Widget for FolderTreeGuideOverlay {
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

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let last_row = self.first_row + self.row_count;
        for segment in &self.segments {
            let start = segment.start_row.max(self.first_row);
            let end = segment.end_row_exclusive.min(last_row);
            if start >= end {
                continue;
            }

            let x = guide_x(bounds, segment.level);
            let y0 = bounds.min.y + ((start - self.first_row) as f32) * TREE_ROW_HEIGHT;
            let raw_y1 = bounds.min.y + ((end - self.first_row) as f32) * TREE_ROW_HEIGHT;
            let y1 = if segment.end_row_exclusive <= last_row {
                raw_y1 - GUIDE_END_GAP
            } else {
                raw_y1
            };
            if y1 <= y0 {
                continue;
            }
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(Point::new(x, y0), Point::new(x + GUIDE_WIDTH, y1)),
                color: GUIDE_COLOR,
            }));
        }
    }
}

pub(super) fn folder_tree_guides_overlay<Message: 'static>(
    folders: &[VisibleFolder],
    first_row: usize,
    end_row: usize,
) -> radiant::prelude::View<Message> {
    let row_count = end_row.saturating_sub(first_row);
    radiant::prelude::custom_widget(
        FolderTreeGuideOverlay::new(first_row, row_count, folder_tree_guide_segments(folders)),
        |_| None,
    )
    .key(format!("folder-tree-guides-overlay-{first_row}-{end_row}"))
    .fill_width()
    .height((row_count as f32) * TREE_ROW_HEIGHT)
}

pub(super) fn folder_tree_indent<Message: 'static>(
    depth: usize,
) -> radiant::prelude::View<Message> {
    radiant::prelude::spacer()
        .width((depth as f32) * TREE_DEPTH_INDENT)
        .height(TREE_ROW_HEIGHT)
}

fn folder_tree_guide_segments(folders: &[VisibleFolder]) -> Vec<FolderTreeGuideSegment> {
    folders
        .iter()
        .enumerate()
        .filter(|(_, folder)| starts_descendant_group(folder))
        .filter_map(|(index, folder)| {
            let group_end = descendant_group_end(folders, index + 1, folder.depth);
            if group_end <= index + 1 {
                return None;
            }
            Some(FolderTreeGuideSegment {
                level: folder.depth.saturating_sub(1),
                start_row: index,
                end_row_exclusive: group_end,
            })
        })
        .collect()
}

fn starts_descendant_group(folder: &VisibleFolder) -> bool {
    folder.has_children && folder.expanded && !folder.is_source_root
}

fn descendant_group_end(folders: &[VisibleFolder], start: usize, parent_depth: usize) -> usize {
    folders[start..]
        .iter()
        .position(|folder| folder.depth <= parent_depth)
        .map_or(folders.len(), |offset| start + offset)
}

fn guide_x(bounds: Rect, level: usize) -> f32 {
    bounds.min.x + (level as f32) * TREE_DEPTH_INDENT + (TREE_DEPTH_INDENT - GUIDE_WIDTH) * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanded_folder_segment_spans_descendant_block_only() {
        let folders = vec![
            visible_folder(".", 0, true, true),
            visible_folder("Samples", 1, true, true),
            visible_folder("Processed", 2, false, false),
            visible_folder("3rdparty", 2, false, false),
            visible_folder("bounce", 1, false, false),
        ];

        assert_eq!(
            folder_tree_guide_segments(&folders),
            vec![FolderTreeGuideSegment {
                level: 0,
                start_row: 1,
                end_row_exclusive: 4,
            },]
        );
    }

    #[test]
    fn expanded_folder_guides_leave_gaps_between_direct_child_siblings() {
        let folders = vec![
            visible_folder(".", 0, true, true),
            visible_folder("samples", 1, true, true),
            visible_folder("Ableton Folder Info", 2, false, false),
            visible_folder("Lexzure", 2, true, true),
            visible_folder("Ableton Folder Info", 3, false, false),
            visible_folder("textures", 2, true, true),
            visible_folder("Ableton Folder Info", 3, false, false),
            visible_folder("gbs Project", 1, true, false),
        ];

        assert_eq!(
            folder_tree_guide_segments(&folders),
            vec![
                FolderTreeGuideSegment {
                    level: 0,
                    start_row: 1,
                    end_row_exclusive: 7,
                },
                FolderTreeGuideSegment {
                    level: 1,
                    start_row: 3,
                    end_row_exclusive: 5,
                },
                FolderTreeGuideSegment {
                    level: 1,
                    start_row: 5,
                    end_row_exclusive: 7,
                },
            ]
        );
    }

    #[test]
    fn overlay_paints_one_continuous_orange_rect_for_visible_segment() {
        let overlay = FolderTreeGuideOverlay::new(
            2,
            2,
            vec![FolderTreeGuideSegment {
                level: 1,
                start_row: 2,
                end_row_exclusive: 4,
            }],
        );
        let plan = overlay.paint_plan_with_defaults(Rect::from_size(80.0, TREE_ROW_HEIGHT * 2.0));
        let lines = plan.fill_rects().collect::<Vec<_>>();

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].color, GUIDE_COLOR);
        assert_eq!(lines[0].rect.min.y, 0.0);
        assert_eq!(lines[0].rect.max.y, TREE_ROW_HEIGHT * 2.0 - GUIDE_END_GAP);
    }

    #[test]
    fn overlay_clips_segment_to_materialized_window() {
        let overlay = FolderTreeGuideOverlay::new(
            3,
            1,
            vec![FolderTreeGuideSegment {
                level: 1,
                start_row: 2,
                end_row_exclusive: 4,
            }],
        );
        let plan = overlay.paint_plan_with_defaults(Rect::from_size(80.0, TREE_ROW_HEIGHT));
        let line = plan.fill_rects().next().expect("expected clipped guide");

        assert_eq!(line.rect.min.y, 0.0);
        assert_eq!(line.rect.max.y, TREE_ROW_HEIGHT - GUIDE_END_GAP);
    }

    fn visible_folder(
        name: &str,
        depth: usize,
        has_children: bool,
        expanded: bool,
    ) -> VisibleFolder {
        VisibleFolder {
            id: name.to_owned(),
            name: name.to_owned(),
            depth,
            is_source_root: depth == 0,
            has_children,
            expanded,
            selected: false,
            drag_active: false,
            drag_source: false,
            drop_candidate: false,
            drop_target: false,
            drop_target_active: false,
            rename_draft: None,
            rename_input_id: None,
        }
    }
}
