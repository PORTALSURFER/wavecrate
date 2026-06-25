use radiant::gui::range::NormalizedRange;
use radiant::gui::types::{Point, Rect};
use radiant::gui::visualization::{
    CanvasSelectionAffordanceStyle, CanvasSelectionBodyHandleStyle, CanvasSelectionEdgeVisualStyle,
    CanvasSelectionGeometry, CanvasSelectionTrailingControlStyle, DragHandleRole,
};

use super::{WaveformSelectionEdge, WaveformSelectionKind, widget::WaveformWidget};
use wavecrate::selection::SelectionRange;

pub(super) const SELECTION_MOVE_HANDLE_HEIGHT: f32 = 7.0;
pub(super) const SELECTION_MOVE_HANDLE_END_INSET: f32 = 9.0;
pub(super) const SELECTION_EXPORT_HANDLE_SIZE: f32 = 16.0;
pub(super) const SELECTION_RESIZE_HANDLE_WIDTH: f32 = 7.0;
pub(super) const SELECTION_RESIZE_HANDLE_STRIP_HEIGHT: f32 = 22.0;
pub(super) const EDIT_GAIN_HANDLE_WIDTH: f32 = 12.0;
pub(super) const EDIT_GAIN_HANDLE_HEIGHT: f32 = 10.0;
pub(super) const EDIT_GAIN_HANDLE_HIT_SIZE: f32 = 18.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WaveformSelectionHandleHover {
    pub(super) kind: WaveformSelectionKind,
    pub(super) role: DragHandleRole,
}

impl WaveformWidget {
    pub(super) fn selection_handle_hover_at(
        &self,
        bounds: Rect,
        position: Point,
    ) -> Option<WaveformSelectionHandleHover> {
        self.play_selection_handle_hover_at(bounds, position)
            .or_else(|| self.edit_selection_handle_hover_at(bounds, position))
    }

    pub(super) fn similar_section_at(
        &self,
        bounds: Rect,
        position: Point,
    ) -> Option<SelectionRange> {
        self.similar_section_ranges
            .iter()
            .rev()
            .copied()
            .find(|range| {
                self.selection_geometry(bounds, Some(*range))
                    .is_some_and(|geometry| geometry.rect.contains(position))
            })
    }

    pub(super) fn play_selection_export_handle_at(&self, bounds: Rect, position: Point) -> bool {
        self.selection_geometry(bounds, self.play_selection)
            .and_then(|geometry| {
                selection_export_affordance_style().affordance_at_point(geometry, bounds, position)
            })
            == Some(DragHandleRole::TrailingControl)
    }

    pub(super) fn edit_gain_handle_at(&self, bounds: Rect, position: Point) -> bool {
        self.edit_gain_handle_hit_rect(bounds)
            .is_some_and(|rect| rect.contains(position))
    }

    pub(super) fn edit_gain_handle_rect(&self, bounds: Rect) -> Option<Rect> {
        let geometry = self.selection_geometry(bounds, self.edit_selection)?;
        edit_gain_handle_rect_for_geometry(
            bounds,
            geometry,
            EDIT_GAIN_HANDLE_WIDTH,
            EDIT_GAIN_HANDLE_HEIGHT,
        )
    }

    fn edit_gain_handle_hit_rect(&self, bounds: Rect) -> Option<Rect> {
        let geometry = self.selection_geometry(bounds, self.edit_selection)?;
        edit_gain_handle_rect_for_geometry(
            bounds,
            geometry,
            EDIT_GAIN_HANDLE_HIT_SIZE,
            EDIT_GAIN_HANDLE_HIT_SIZE,
        )
    }

    pub(super) fn selection_move_handle_at(
        &self,
        bounds: Rect,
        position: Point,
        kind: WaveformSelectionKind,
    ) -> bool {
        let range = match kind {
            WaveformSelectionKind::Play => self.play_selection,
            WaveformSelectionKind::Edit => self.edit_selection,
        };
        self.selection_geometry(bounds, range).and_then(|geometry| {
            selection_move_affordance_style().affordance_at_point(geometry, bounds, position)
        }) == Some(DragHandleRole::Body)
    }

    pub(super) fn selection_resize_handle_at(
        &self,
        bounds: Rect,
        position: Point,
        kind: WaveformSelectionKind,
    ) -> Option<WaveformSelectionEdge> {
        match kind {
            WaveformSelectionKind::Play => self.play_selection_resize_handle_at(bounds, position),
            WaveformSelectionKind::Edit => self.edit_selection_resize_handle_at(bounds, position),
        }
    }

    fn play_selection_resize_handle_at(
        &self,
        bounds: Rect,
        position: Point,
    ) -> Option<WaveformSelectionEdge> {
        let role = self
            .selection_geometry(bounds, self.play_selection)
            .and_then(|geometry| {
                selection_resize_affordance_style().affordance_at_point(
                    geometry,
                    bounds.top_edge_strip(SELECTION_RESIZE_HANDLE_STRIP_HEIGHT),
                    position,
                )
            })?;
        waveform_selection_edge(role)
    }

    fn edit_selection_resize_handle_at(
        &self,
        bounds: Rect,
        position: Point,
    ) -> Option<WaveformSelectionEdge> {
        let selection = self.edit_selection?;
        let role = self
            .selection_geometry(bounds, Some(selection))
            .and_then(|geometry| {
                selection_resize_affordance_style().affordance_at_point(
                    geometry,
                    edit_selection_resize_edge_bounds(bounds),
                    position,
                )
            })?;
        let edge = waveform_selection_edge(role)?;
        edit_selection_resize_edge_visible(selection, edge).then_some(edge)
    }

    fn play_selection_handle_hover_at(
        &self,
        bounds: Rect,
        position: Point,
    ) -> Option<WaveformSelectionHandleHover> {
        self.selection_geometry(bounds, self.play_selection)
            .and_then(|geometry| {
                selection_move_resize_affordance_style().affordance_at_point(
                    geometry,
                    bounds.top_edge_strip(SELECTION_RESIZE_HANDLE_STRIP_HEIGHT),
                    position,
                )
            })
            .map(|role| WaveformSelectionHandleHover {
                kind: WaveformSelectionKind::Play,
                role,
            })
    }

    fn edit_selection_handle_hover_at(
        &self,
        bounds: Rect,
        position: Point,
    ) -> Option<WaveformSelectionHandleHover> {
        if let Some(edge) = self.edit_selection_resize_handle_at(bounds, position) {
            return Some(WaveformSelectionHandleHover {
                kind: WaveformSelectionKind::Edit,
                role: waveform_selection_edge_role(edge),
            });
        }
        self.selection_geometry(bounds, self.edit_selection)
            .and_then(|geometry| {
                selection_move_affordance_style().affordance_at_point(geometry, bounds, position)
            })
            .map(|role| WaveformSelectionHandleHover {
                kind: WaveformSelectionKind::Edit,
                role,
            })
    }

    pub(super) fn visible_normalized_range_for_selection(
        &self,
        range: Option<wavecrate::selection::SelectionRange>,
    ) -> Option<NormalizedRange> {
        let range = range?;
        let (start, end) = self.viewport.visible_range_from_absolute(
            self.file.frames,
            range.start(),
            range.end(),
        )?;
        Some(NormalizedRange::from_fractions(start, end))
    }

    pub(super) fn visible_ratio_for_absolute(&self, ratio: Option<f32>) -> Option<f32> {
        self.viewport
            .visible_ratio_from_absolute(self.file.frames, ratio?)
    }

    pub(super) fn absolute_ratio_for_visible(&self, visible_ratio: f32) -> Option<f32> {
        Some(
            self.viewport
                .absolute_ratio_from_visible(self.file.frames, visible_ratio),
        )
    }

    pub(super) fn selection_geometry(
        &self,
        bounds: Rect,
        range: Option<wavecrate::selection::SelectionRange>,
    ) -> Option<CanvasSelectionGeometry> {
        let range = self.visible_normalized_range_for_selection(range)?;
        CanvasSelectionGeometry::new(bounds, range.start_fraction(), range.end_fraction())
    }
}

fn selection_export_affordance_style() -> CanvasSelectionAffordanceStyle {
    CanvasSelectionAffordanceStyle::new().with_trailing_control(selection_export_handle_style())
}

fn selection_move_affordance_style() -> CanvasSelectionAffordanceStyle {
    CanvasSelectionAffordanceStyle::new().with_body(selection_move_handle_style())
}

fn selection_resize_affordance_style() -> CanvasSelectionAffordanceStyle {
    CanvasSelectionAffordanceStyle::new().with_edge(selection_resize_edge_style())
}

fn selection_move_resize_affordance_style() -> CanvasSelectionAffordanceStyle {
    CanvasSelectionAffordanceStyle::new()
        .with_body(selection_move_handle_style())
        .with_edge(selection_resize_edge_style())
        .with_trailing_control(selection_export_handle_style())
}

pub(super) const fn selection_move_handle_style() -> CanvasSelectionBodyHandleStyle {
    CanvasSelectionBodyHandleStyle::new(
        SELECTION_MOVE_HANDLE_HEIGHT,
        SELECTION_MOVE_HANDLE_END_INSET,
        0.28,
        1.0,
    )
}

pub(super) const fn selection_resize_edge_style() -> CanvasSelectionEdgeVisualStyle {
    CanvasSelectionEdgeVisualStyle::new(SELECTION_RESIZE_HANDLE_WIDTH, 0.0)
}

pub(super) const fn selection_export_handle_style() -> CanvasSelectionTrailingControlStyle {
    CanvasSelectionTrailingControlStyle::new(SELECTION_EXPORT_HANDLE_SIZE, 0.0)
}

pub(super) fn edit_selection_resize_edge_bounds(bounds: Rect) -> Rect {
    bounds.bottom_edge_strip(SELECTION_RESIZE_HANDLE_STRIP_HEIGHT)
}

pub(super) fn edit_selection_resize_edge_visible(
    selection: wavecrate::selection::SelectionRange,
    edge: WaveformSelectionEdge,
) -> bool {
    match edge {
        WaveformSelectionEdge::Start => selection.fade_in().is_none(),
        WaveformSelectionEdge::End => selection.fade_out().is_none(),
    }
}

pub(super) const fn waveform_selection_edge_role(edge: WaveformSelectionEdge) -> DragHandleRole {
    match edge {
        WaveformSelectionEdge::Start => DragHandleRole::Start,
        WaveformSelectionEdge::End => DragHandleRole::End,
    }
}

fn waveform_selection_edge(role: DragHandleRole) -> Option<WaveformSelectionEdge> {
    match role {
        DragHandleRole::Start => Some(WaveformSelectionEdge::Start),
        DragHandleRole::End => Some(WaveformSelectionEdge::End),
        _ => None,
    }
}

fn edit_gain_handle_rect_for_geometry(
    bounds: Rect,
    geometry: CanvasSelectionGeometry,
    width: f32,
    height: f32,
) -> Option<Rect> {
    if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
        return None;
    }
    let center_x = geometry.rect.center().x;
    Some(Rect::from_xy_size(center_x - width * 0.5, bounds.min.y, width, height).clamp_to(bounds))
}
