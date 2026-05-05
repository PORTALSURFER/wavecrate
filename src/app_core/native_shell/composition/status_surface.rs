//! Generic status-bar surface projection for the native-shell compatibility layer.
//!
//! This module lets the compat shell compose one production footer slice from
//! public `radiant::layout`, `radiant::runtime`, and `radiant::widgets`
//! building blocks before the whole native window runtime migrates away from
//! the legacy `AppModel` path.

use super::style::SizingTokens;
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{
        Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, MainAlign, OverflowPolicy,
        SizeModeCross, SizeModeMain, SlotParams, layout_tree,
    },
    runtime::{SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{CanvasWidget, TextWidget, WidgetSizing, WidgetSpec},
};

const STATUS_ROOT_ID: u64 = 960;
const STATUS_ROW_ID: u64 = 961;
const STATUS_LEFT_SEGMENT_ID: u64 = 962;
const STATUS_CENTER_SEGMENT_ID: u64 = 963;
const STATUS_RIGHT_SEGMENT_ID: u64 = 964;
const STATUS_PROGRESS_SEGMENT_ID: u64 = 965;
const STATUS_LEFT_ALIGN_ID: u64 = 966;
const STATUS_CENTER_ALIGN_ID: u64 = 967;
const STATUS_RIGHT_ALIGN_ID: u64 = 968;
const STATUS_PROGRESS_COLUMN_ID: u64 = 969;
const STATUS_LEFT_TEXT_ID: u64 = 970;
const STATUS_CENTER_TEXT_ID: u64 = 971;
const STATUS_RIGHT_TEXT_ID: u64 = 972;
const STATUS_PROGRESS_TEXT_ID: u64 = 973;
const STATUS_PROGRESS_TRACK_ID: u64 = 974;
const STATUS_LEFT_GAP_ID: u64 = 975;
const STATUS_RIGHT_GAP_ID: u64 = 976;
const STATUS_PROGRESS_GAP_ID: u64 = 977;
const STATUS_PROGRESS_ALIGN_ID: u64 = 978;
const STATUS_LEFT_RATIO: f32 = 0.30;
const STATUS_RIGHT_RATIO: f32 = 0.22;
const STATUS_PROGRESS_RATIO: f32 = 0.16;
const STATUS_PROGRESS_MIN_WIDTH: f32 = 84.0;
const STATUS_PROGRESS_MAX_WIDTH: f32 = 144.0;

/// User-facing content projected into the generic status-bar surface.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StatusSurfaceContent {
    /// Left-aligned footer copy.
    pub left_label: String,
    /// Center footer copy. This becomes the inline progress label when active.
    pub center_label: String,
    /// Right-aligned footer copy.
    pub right_label: String,
    /// Counter shown in the compact progress slot.
    pub progress_counter: String,
}

/// Resolved layout rectangles for the generic status-bar surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StatusSurfaceLayout {
    /// Left segment bounds.
    pub left_segment: Rect,
    /// Center segment bounds.
    pub center_segment: Rect,
    /// Right segment bounds.
    pub right_segment: Rect,
    /// Progress segment bounds.
    pub progress_segment: Rect,
    /// Left text widget bounds.
    pub left_text_rect: Rect,
    /// Center text widget bounds.
    pub center_text_rect: Rect,
    /// Right text widget bounds.
    pub right_text_rect: Rect,
    /// Progress counter widget bounds.
    pub progress_text_rect: Rect,
    /// Progress track canvas bounds.
    pub progress_track_rect: Rect,
}

/// Build a generic declarative status-bar surface for one footer snapshot.
pub(crate) fn build_status_surface(
    content: &StatusSurfaceContent,
    sizing: SizingTokens,
    viewport_width: f32,
) -> UiSurface<()> {
    let progress_width = progress_slot_width(viewport_width, sizing);
    UiSurface::new(SurfaceNode::container(
        STATUS_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: sizing.panel_inset.max(0.0),
                right: sizing.panel_inset.max(0.0),
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::new(
            SlotParams::fill(),
            SurfaceNode::container(
                STATUS_ROW_ID,
                ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing: 0.0,
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    SurfaceChild::new(
                        percent_slot(STATUS_LEFT_RATIO),
                        segment_surface(
                            STATUS_LEFT_SEGMENT_ID,
                            STATUS_LEFT_ALIGN_ID,
                            STATUS_LEFT_TEXT_ID,
                            &content.left_label,
                            sizing,
                        ),
                    ),
                    SurfaceChild::new(
                        fixed_slot(sizing.status_segment_gap),
                        spacer_surface(STATUS_LEFT_GAP_ID),
                    ),
                    SurfaceChild::new(
                        SlotParams::fill(),
                        segment_surface(
                            STATUS_CENTER_SEGMENT_ID,
                            STATUS_CENTER_ALIGN_ID,
                            STATUS_CENTER_TEXT_ID,
                            &content.center_label,
                            sizing,
                        ),
                    ),
                    SurfaceChild::new(
                        fixed_slot(sizing.status_segment_gap),
                        spacer_surface(STATUS_RIGHT_GAP_ID),
                    ),
                    SurfaceChild::new(
                        percent_slot(STATUS_RIGHT_RATIO),
                        segment_surface(
                            STATUS_RIGHT_SEGMENT_ID,
                            STATUS_RIGHT_ALIGN_ID,
                            STATUS_RIGHT_TEXT_ID,
                            &content.right_label,
                            sizing,
                        ),
                    ),
                    SurfaceChild::new(
                        fixed_slot(sizing.status_segment_gap),
                        spacer_surface(STATUS_PROGRESS_GAP_ID),
                    ),
                    SurfaceChild::new(
                        fixed_slot(progress_width),
                        progress_surface(content, sizing, progress_width),
                    ),
                ],
            ),
        )],
    ))
}

/// Resolve the generic status-bar surface layout inside one footer rect.
pub(crate) fn resolve_status_surface_layout(
    status_bar: Rect,
    sizing: SizingTokens,
    content: &StatusSurfaceContent,
) -> StatusSurfaceLayout {
    let surface = build_status_surface(content, sizing, status_bar.width());
    let output = layout_tree(&surface.layout_node(), status_bar);
    let empty = Rect::from_min_max(status_bar.min, status_bar.min);
    StatusSurfaceLayout {
        left_segment: clamp_rect_to_bounds(
            rect_for(&output.rects, STATUS_LEFT_SEGMENT_ID, empty),
            status_bar,
        ),
        center_segment: clamp_rect_to_bounds(
            rect_for(&output.rects, STATUS_CENTER_SEGMENT_ID, empty),
            status_bar,
        ),
        right_segment: clamp_rect_to_bounds(
            rect_for(&output.rects, STATUS_RIGHT_SEGMENT_ID, empty),
            status_bar,
        ),
        progress_segment: clamp_rect_to_bounds(
            rect_for(&output.rects, STATUS_PROGRESS_SEGMENT_ID, empty),
            status_bar,
        ),
        left_text_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, STATUS_LEFT_TEXT_ID, empty),
            status_bar,
        ),
        center_text_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, STATUS_CENTER_TEXT_ID, empty),
            status_bar,
        ),
        right_text_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, STATUS_RIGHT_TEXT_ID, empty),
            status_bar,
        ),
        progress_text_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, STATUS_PROGRESS_TEXT_ID, empty),
            status_bar,
        ),
        progress_track_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, STATUS_PROGRESS_TRACK_ID, empty),
            status_bar,
        ),
    }
}

fn segment_surface(
    segment_id: u64,
    align_id: u64,
    text_id: u64,
    label: &str,
    sizing: SizingTokens,
) -> SurfaceNode<()> {
    SurfaceNode::container(
        segment_id,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: (sizing.text_inset_x + sizing.header_label_gutter).max(0.0),
                right: sizing.text_inset_x.max(0.0),
                top: sizing.text_inset_y.max(0.0),
                bottom: sizing.text_inset_y.max(0.0),
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::new(
            SlotParams::fill(),
            SurfaceNode::container(
                align_id,
                ContainerPolicy {
                    kind: ContainerKind::AlignBox,
                    align_main: MainAlign::Center,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::new(
                    text_slot(sizing.font_status),
                    SurfaceNode::widget(
                        WidgetSpec::Text(TextWidget::new(
                            text_id,
                            label,
                            WidgetSizing::fixed(Vector2::new(1.0, sizing.font_status.max(1.0)))
                                .with_baseline((sizing.font_status * 0.75).max(0.0)),
                        )),
                        WidgetMessageMapper::None,
                    ),
                )],
            ),
        )],
    )
}

fn progress_surface(
    content: &StatusSurfaceContent,
    sizing: SizingTokens,
    progress_width: f32,
) -> SurfaceNode<()> {
    let track_height = (sizing.border_width * 2.0).max(4.0);
    SurfaceNode::container(
        STATUS_PROGRESS_SEGMENT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: (sizing.text_inset_x + sizing.header_label_gutter).max(0.0),
                right: sizing.text_inset_x.max(0.0),
                top: sizing.text_inset_y.max(0.0),
                bottom: sizing.text_inset_y.max(1.0),
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::new(
            SlotParams::fill(),
            SurfaceNode::container(
                STATUS_PROGRESS_COLUMN_ID,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: 0.0,
                    align_main: MainAlign::End,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    SurfaceChild::new(
                        SlotParams::fill(),
                        SurfaceNode::container(
                            STATUS_PROGRESS_ALIGN_ID,
                            ContainerPolicy {
                                kind: ContainerKind::AlignBox,
                                align_main: MainAlign::Center,
                                align_cross: CrossAlign::Stretch,
                                overflow: OverflowPolicy::Clip,
                                ..ContainerPolicy::default()
                            },
                            vec![SurfaceChild::new(
                                text_slot(sizing.font_status),
                                SurfaceNode::widget(
                                    WidgetSpec::Text(TextWidget::new(
                                        STATUS_PROGRESS_TEXT_ID,
                                        &content.progress_counter,
                                        WidgetSizing::fixed(Vector2::new(
                                            progress_width.max(1.0),
                                            sizing.font_status.max(1.0),
                                        ))
                                        .with_baseline((sizing.font_status * 0.75).max(0.0)),
                                    )),
                                    WidgetMessageMapper::None,
                                ),
                            )],
                        ),
                    ),
                    SurfaceChild::new(
                        fixed_slot(track_height),
                        SurfaceNode::widget(
                            WidgetSpec::Canvas(CanvasWidget::new(
                                STATUS_PROGRESS_TRACK_ID,
                                WidgetSizing::fixed(Vector2::new(
                                    progress_width.max(1.0),
                                    track_height.max(1.0),
                                )),
                            )),
                            WidgetMessageMapper::None,
                        ),
                    ),
                ],
            ),
        )],
    )
}

fn progress_slot_width(viewport_width: f32, sizing: SizingTokens) -> f32 {
    let inner_width = (viewport_width - (sizing.panel_inset.max(0.0) * 2.0)).max(0.0);
    (inner_width * STATUS_PROGRESS_RATIO)
        .clamp(STATUS_PROGRESS_MIN_WIDTH, STATUS_PROGRESS_MAX_WIDTH)
}

fn spacer_surface(id: u64) -> SurfaceNode<()> {
    SurfaceNode::widget(
        WidgetSpec::Canvas(CanvasWidget::new(
            id,
            WidgetSizing::fixed(Vector2::new(1.0, 1.0)),
        )),
        WidgetMessageMapper::None,
    )
}

fn percent_slot(ratio: f32) -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Percent(ratio.max(0.0)),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(0.0, f32::INFINITY, 0.0, f32::INFINITY),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Stretch),
        allow_fixed_compress: false,
    }
}

fn fixed_slot(width: f32) -> SlotParams {
    let width = width.max(0.0);
    SlotParams {
        size_main: SizeModeMain::Fixed(width),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(width, width, 0.0, f32::INFINITY),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Stretch),
        allow_fixed_compress: false,
    }
}

fn text_slot(font_size: f32) -> SlotParams {
    let font_size = font_size.max(1.0);
    SlotParams {
        size_main: SizeModeMain::Fixed(font_size),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(0.0, f32::INFINITY, font_size, font_size),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Stretch),
        allow_fixed_compress: false,
    }
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn status_surface_uses_public_text_and_canvas_widgets() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let surface = build_status_surface(
            &StatusSurfaceContent {
                left_label: String::from("Transport: running"),
                center_label: String::from("rows: 20 | selected: 2"),
                right_label: String::from("col: 2/3"),
                progress_counter: String::from("4/9"),
            },
            style.sizing,
            1280.0,
        );
        let left = surface
            .find_widget(STATUS_LEFT_TEXT_ID)
            .expect("left text widget");
        let track = surface
            .find_widget(STATUS_PROGRESS_TRACK_ID)
            .expect("progress track widget");
        assert_eq!(left.widget().kind(), crate::widgets::WidgetKind::Text);
        assert_eq!(track.widget().kind(), crate::widgets::WidgetKind::Canvas);
    }

    #[test]
    fn status_surface_layout_keeps_segments_ordered() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let bar = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(1280.0, 20.0));
        let layout =
            resolve_status_surface_layout(bar, style.sizing, &StatusSurfaceContent::default());
        assert_inside(bar, layout.left_segment);
        assert_inside(bar, layout.center_segment);
        assert_inside(bar, layout.right_segment);
        assert_inside(bar, layout.progress_segment);
        assert!(layout.left_segment.max.x <= layout.center_segment.min.x);
        assert!(layout.center_segment.max.x <= layout.right_segment.min.x);
        assert!(layout.right_segment.max.x <= layout.progress_segment.min.x);
    }

    #[test]
    fn status_surface_text_and_progress_widgets_stay_inside_footer() {
        let style = StyleTokens::for_viewport_width(820.0);
        let bar = Rect::from_min_max(Point::new(10.0, 5.0), Point::new(360.0, 24.0));
        let layout =
            resolve_status_surface_layout(bar, style.sizing, &StatusSurfaceContent::default());
        assert_inside(layout.left_segment, layout.left_text_rect);
        assert_inside(layout.center_segment, layout.center_text_rect);
        assert_inside(layout.right_segment, layout.right_text_rect);
        assert_inside(layout.progress_segment, layout.progress_text_rect);
        assert_inside(layout.progress_segment, layout.progress_track_rect);
    }
}
