use radiant::{
    gui::{
        types::{Point, Rect, Rgba8},
        visualization::CanvasSelectionGeometry,
    },
    runtime::{PaintTextAlign, WidgetPaint},
};

use crate::ui_formatting::{format_selection_duration, format_waveform_bpm_input};

use super::{WaveformActiveDragKind, WaveformSelectionKind, WaveformWidget};

const PLAYMARK_LABEL_HEIGHT: f32 = 18.0;
const PLAYMARK_LABEL_BOTTOM_INSET: f32 = 2.0;
const PLAYMARK_LABEL_SELECTION_INSET: f32 = 4.0;
const PLAYMARK_LABEL_OUTSIDE_GAP: f32 = 6.0;
const PLAYMARK_LABEL_HORIZONTAL_PADDING: f32 = 10.0;
const PLAYMARK_LABEL_GLYPH_WIDTH: f32 = 10.0;
const PLAYMARK_LABEL_MIN_WIDTH: f32 = 80.0;
const PLAYMARK_LABEL_MAX_WIDTH: f32 = 150.0;
const PLAYMARK_LABEL_BACKGROUND: Rgba8 = Rgba8::new(25, 18, 16, 214);
const PLAYMARK_LABEL_TEXT: Rgba8 = Rgba8::new(255, 226, 210, 255);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PlaymarkLabelPlacement {
    Inside,
    OutsideRight,
    OutsideLeft,
    FallbackRight,
    FallbackLeft,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PlaymarkLabelLayout {
    pub(super) rect: Rect,
    pub(super) placement: PlaymarkLabelPlacement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaymarkLabelPaintOwner {
    Base,
    Runtime,
}

impl WaveformWidget {
    pub(super) fn append_playmark_label_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
        selection: wavecrate::selection::SelectionRange,
    ) {
        let Some(label) = playmark_selection_label(
            selection,
            self.file.frames,
            self.file.sample_rate,
            self.beat_guides_enabled,
            self.beat_guide_count,
        ) else {
            return;
        };
        let Some(layout) = playmark_label_layout(bounds, geometry.rect, label.len()) else {
            return;
        };

        paint.push_visible_fill_rect(layout.rect, PLAYMARK_LABEL_BACKGROUND);
        paint.push_text(
            label,
            layout.rect,
            PLAYMARK_LABEL_TEXT,
            PaintTextAlign::Center,
        );
    }

    pub(super) fn base_owns_playmark_label(&self) -> bool {
        self.playmark_label_paint_owner() == PlaymarkLabelPaintOwner::Base
    }

    pub(super) fn runtime_owns_playmark_label(&self) -> bool {
        self.playmark_label_paint_owner() == PlaymarkLabelPaintOwner::Runtime
    }

    pub(super) fn append_runtime_playmark_label_fallback_paint(
        &self,
        primitives: &mut Vec<radiant::runtime::PaintPrimitive>,
        bounds: Rect,
    ) {
        if !self.runtime_owns_playmark_label()
            || self
                .live_selection_preview
                .is_some_and(|preview| preview.kind == WaveformSelectionKind::Play)
        {
            return;
        }
        let Some(selection) = self.play_selection else {
            return;
        };
        let Some(geometry) = self.selection_geometry(bounds, Some(selection)) else {
            return;
        };
        self.append_playmark_label_paint(
            &mut WidgetPaint::new(primitives, self.common.id),
            bounds,
            geometry,
            selection,
        );
    }

    fn playmark_label_paint_owner(&self) -> PlaymarkLabelPaintOwner {
        match self.active_drag_kind {
            Some(WaveformActiveDragKind::Selection(WaveformSelectionKind::Play))
            | Some(WaveformActiveDragKind::SelectionMove(WaveformSelectionKind::Play)) => {
                PlaymarkLabelPaintOwner::Runtime
            }
            _ => PlaymarkLabelPaintOwner::Base,
        }
    }
}

fn playmark_selection_label(
    selection: wavecrate::selection::SelectionRange,
    frames: usize,
    sample_rate: u32,
    beat_guides_enabled: bool,
    beat_guide_count: u8,
) -> Option<String> {
    let duration_seconds = selection.width() * frames as f32 / sample_rate.max(1) as f32;
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return None;
    }

    if beat_guides_enabled && beat_guide_count > 0 {
        let bpm = f32::from(beat_guide_count) * 60.0 / duration_seconds;
        return format_waveform_bpm_input(bpm).map(|bpm| format!("{bpm} BPM"));
    }

    Some(format_selection_duration(duration_seconds))
}

pub(super) fn playmark_label_layout(
    bounds: Rect,
    selection_rect: Rect,
    label_len: usize,
) -> Option<PlaymarkLabelLayout> {
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return None;
    }

    let desired_width =
        label_len as f32 * PLAYMARK_LABEL_GLYPH_WIDTH + PLAYMARK_LABEL_HORIZONTAL_PADDING * 2.0;
    let width = desired_width
        .clamp(PLAYMARK_LABEL_MIN_WIDTH, PLAYMARK_LABEL_MAX_WIDTH)
        .min(bounds.width());
    let selection_left = selection_rect.min.x.clamp(bounds.min.x, bounds.max.x);
    let selection_right = selection_rect.max.x.clamp(selection_left, bounds.max.x);
    let selection_width = selection_right - selection_left;
    let (left, placement) = if selection_width >= width + PLAYMARK_LABEL_SELECTION_INSET * 2.0 {
        (
            (selection_left + (selection_width - width) * 0.5)
                .clamp(bounds.min.x, bounds.max.x - width),
            PlaymarkLabelPlacement::Inside,
        )
    } else {
        outside_label_left(bounds, selection_left, selection_right, width)
    };
    let bottom = (bounds.max.y - PLAYMARK_LABEL_BOTTOM_INSET).max(bounds.min.y);
    let top = (bottom - PLAYMARK_LABEL_HEIGHT).max(bounds.min.y);

    Some(PlaymarkLabelLayout {
        rect: Rect::from_min_max(Point::new(left, top), Point::new(left + width, bottom)),
        placement,
    })
}

fn outside_label_left(
    bounds: Rect,
    selection_left: f32,
    selection_right: f32,
    width: f32,
) -> (f32, PlaymarkLabelPlacement) {
    let right = selection_right + PLAYMARK_LABEL_OUTSIDE_GAP;
    if right + width <= bounds.max.x {
        return (right, PlaymarkLabelPlacement::OutsideRight);
    }

    let left = selection_left - PLAYMARK_LABEL_OUTSIDE_GAP - width;
    if left >= bounds.min.x {
        return (left, PlaymarkLabelPlacement::OutsideLeft);
    }

    let right_room = bounds.max.x - selection_right;
    let left_room = selection_left - bounds.min.x;
    if right_room >= left_room {
        (
            right.clamp(bounds.min.x, bounds.max.x - width),
            PlaymarkLabelPlacement::FallbackRight,
        )
    } else {
        (
            left.clamp(bounds.min.x, bounds.max.x - width),
            PlaymarkLabelPlacement::FallbackLeft,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: f32, right: f32) -> Rect {
        Rect::from_min_max(Point::new(left, 0.0), Point::new(right, 80.0))
    }

    #[test]
    fn wide_playmark_contains_centered_label_with_inset() {
        let layout = playmark_label_layout(rect(0.0, 400.0), rect(100.0, 300.0), 6)
            .expect("wide label layout");

        assert_eq!(layout.placement, PlaymarkLabelPlacement::Inside);
        assert_eq!(layout.rect.min.x, 160.0);
        assert_eq!(layout.rect.max.x, 240.0);
    }

    #[test]
    fn narrow_playmark_prefers_right_then_falls_back_left_at_right_edge() {
        let centered = playmark_label_layout(rect(0.0, 400.0), rect(195.0, 205.0), 6)
            .expect("centered narrow layout");
        assert_eq!(centered.placement, PlaymarkLabelPlacement::OutsideRight);
        assert_eq!(centered.rect.min.x, 211.0);

        let right_edge = playmark_label_layout(rect(0.0, 400.0), rect(350.0, 360.0), 6)
            .expect("right-edge narrow layout");
        assert_eq!(right_edge.placement, PlaymarkLabelPlacement::OutsideLeft);
        assert_eq!(right_edge.rect.max.x, 344.0);
    }

    #[test]
    fn narrow_playmark_near_left_edge_keeps_complete_label_to_right() {
        let layout = playmark_label_layout(rect(0.0, 400.0), rect(10.0, 20.0), 6)
            .expect("left-edge narrow layout");

        assert_eq!(layout.placement, PlaymarkLabelPlacement::OutsideRight);
        assert_eq!(layout.rect.min.x, 26.0);
        assert_eq!(layout.rect.max.x, 106.0);
    }

    #[test]
    fn both_sides_fit_uses_stable_right_side_tie_breaker() {
        let first =
            playmark_label_layout(rect(0.0, 400.0), rect(195.0, 205.0), 6).expect("first layout");
        let second =
            playmark_label_layout(rect(0.0, 400.0), rect(195.0, 205.0), 6).expect("second layout");

        assert_eq!(first, second);
        assert_eq!(first.placement, PlaymarkLabelPlacement::OutsideRight);
    }

    #[test]
    fn narrow_viewport_fallback_stays_in_bounds_and_chooses_roomier_side() {
        let tied = playmark_label_layout(rect(0.0, 100.0), rect(35.0, 65.0), 6)
            .expect("tied fallback layout");
        assert_eq!(tied.placement, PlaymarkLabelPlacement::FallbackRight);
        assert_eq!(tied.rect.min.x, 20.0);
        assert_eq!(tied.rect.max.x, 100.0);

        let left_roomier = playmark_label_layout(rect(0.0, 100.0), rect(60.0, 80.0), 6)
            .expect("left-roomier fallback layout");
        assert_eq!(left_roomier.placement, PlaymarkLabelPlacement::FallbackLeft);
        assert_eq!(left_roomier.rect.min.x, 0.0);
        assert_eq!(left_roomier.rect.max.x, 80.0);
    }

    #[test]
    fn duration_and_bpm_widths_share_outside_placement_policy() {
        let duration = playmark_label_layout(rect(0.0, 400.0), rect(150.0, 160.0), 6)
            .expect("duration layout");
        let bpm =
            playmark_label_layout(rect(0.0, 400.0), rect(150.0, 160.0), 7).expect("bpm layout");

        assert_eq!(duration.placement, PlaymarkLabelPlacement::OutsideRight);
        assert_eq!(bpm.placement, PlaymarkLabelPlacement::OutsideRight);
        assert_eq!(duration.rect.width(), 80.0);
        assert_eq!(bpm.rect.width(), 90.0);
        assert!(duration.rect.min.x > 160.0);
        assert!(bpm.rect.min.x > 160.0);
    }
}
