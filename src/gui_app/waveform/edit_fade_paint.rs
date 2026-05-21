use radiant::{
    gui::{
        range::NormalizedRange,
        types::{Point, Rect, Rgba8},
    },
    runtime::PaintPrimitive,
};

use super::{WaveformEditFadeHandle, WaveformWidget};

const EDIT_FADE_HANDLE_TAB_SIZE: f32 = 10.0;
const EDIT_FADE_HANDLE_WIDTH: f32 = 3.0;

impl WaveformWidget {
    pub(super) fn append_edit_fade_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        let Some(selection) = self.edit_preview.selection else {
            return;
        };
        let Some(selection_rect) = self.visible_rect_for_normalized_range(bounds, selection) else {
            return;
        };
        let accent = Rgba8 {
            r: 82,
            g: 168,
            b: 255,
            a: 210,
        };
        if let Some(fade_rect) = self.fade_in_rect(bounds, selection, selection_rect) {
            self.push_fill(primitives, fade_rect, Rgba8 { a: 52, ..accent });
        }
        if let Some(fade_rect) = self.fade_out_rect(bounds, selection, selection_rect) {
            self.push_fill(primitives, fade_rect, Rgba8 { a: 52, ..accent });
        }
        if let Some(fade_rect) = self.fade_in_outer_rect(bounds, selection, selection_rect) {
            self.push_fill(primitives, fade_rect, Rgba8 { a: 38, ..accent });
        }
        if let Some(fade_rect) = self.fade_out_outer_rect(bounds, selection, selection_rect) {
            self.push_fill(primitives, fade_rect, Rgba8 { a: 38, ..accent });
        }
        self.append_edit_fade_curve_paint(primitives, bounds, selection_rect, accent);
        for handle in [
            WaveformEditFadeHandle::FadeInEnd,
            WaveformEditFadeHandle::FadeOutStart,
            WaveformEditFadeHandle::FadeInStart,
            WaveformEditFadeHandle::FadeOutEnd,
            WaveformEditFadeHandle::FadeInOuterStart,
            WaveformEditFadeHandle::FadeOutOuterEnd,
        ] {
            if let Some(rect) = self.edit_fade_handle_rect(bounds, selection_rect, handle) {
                self.push_fill(primitives, rect, Rgba8 { a: 205, ..accent });
            }
        }
    }

    fn append_edit_fade_curve_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        selection_rect: Rect,
        color: Rgba8,
    ) {
        let Some(selection) = self.edit_selection else {
            return;
        };
        let width = selection.width();
        if width <= 0.0 {
            return;
        }
        if let Some(fade_in) = selection.fade_in().filter(|fade| fade.length > 0.0) {
            let start = (selection.start() - width * fade_in.mute).max(0.0);
            let end = (selection.start() + width * fade_in.length).min(selection.end());
            self.push_edit_fade_curve_points(
                primitives,
                bounds,
                selection_rect,
                selection,
                start,
                end,
                Rgba8 { a: 225, ..color },
            );
        }
        if let Some(fade_out) = selection.fade_out().filter(|fade| fade.length > 0.0) {
            let end = (selection.end() + width * fade_out.mute).min(1.0);
            let start = (selection.end() - width * fade_out.length).max(selection.start());
            self.push_edit_fade_curve_points(
                primitives,
                bounds,
                selection_rect,
                selection,
                start,
                end,
                Rgba8 { a: 225, ..color },
            );
        }
    }

    fn push_edit_fade_curve_points(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        selection_rect: Rect,
        selection: wavecrate::selection::SelectionRange,
        start: f32,
        end: f32,
        color: Rgba8,
    ) {
        let width = ((end - start).abs() * bounds.width()).max(1.0);
        let steps = ((width / 4.0).round() as usize).clamp(10, 96);
        let marker = 2.25_f32.min(selection_rect.height().max(1.0));
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let position = start + (end - start) * t;
            let Some(visible_ratio) = self.visible_ratio_for_absolute(Some(position)) else {
                continue;
            };
            let x = bounds.min.x + bounds.width() * visible_ratio.clamp(0.0, 1.0);
            let gain = selection.gain_at_position(position, 0.0).clamp(0.0, 1.0);
            let y = selection_rect.max.y - selection_rect.height() * gain;
            let half = marker * 0.5;
            self.push_fill(
                primitives,
                Rect::from_min_max(
                    Point::new(
                        (x - half).clamp(bounds.min.x, bounds.max.x),
                        (y - half).clamp(selection_rect.min.y, selection_rect.max.y),
                    ),
                    Point::new(
                        (x + half).clamp(bounds.min.x, bounds.max.x),
                        (y + half).clamp(selection_rect.min.y, selection_rect.max.y),
                    ),
                ),
                color,
            );
        }
    }

    pub(super) fn edit_fade_handle_at(
        &self,
        bounds: Rect,
        position: Point,
    ) -> Option<WaveformEditFadeHandle> {
        let selection = self.edit_preview.selection?;
        let selection_rect = self.visible_rect_for_normalized_range(bounds, selection)?;
        [
            WaveformEditFadeHandle::FadeInEnd,
            WaveformEditFadeHandle::FadeOutStart,
            WaveformEditFadeHandle::FadeInStart,
            WaveformEditFadeHandle::FadeOutEnd,
            WaveformEditFadeHandle::FadeInOuterStart,
            WaveformEditFadeHandle::FadeOutOuterEnd,
        ]
        .into_iter()
        .find(|handle| {
            self.edit_fade_handle_rect(bounds, selection_rect, *handle)
                .is_some_and(|rect| rect.contains(position))
        })
    }

    fn fade_in_rect(
        &self,
        bounds: Rect,
        selection: NormalizedRange,
        selection_rect: Rect,
    ) -> Option<Rect> {
        let end = self
            .edit_preview
            .leading_end_micros
            .unwrap_or(selection.start_micros);
        if end <= selection.start_micros {
            return None;
        }
        let x = self.x_for_micros(bounds, end)?;
        Some(Rect::from_min_max(
            Point::new(selection_rect.min.x, selection_rect.min.y),
            Point::new(
                x.clamp(selection_rect.min.x, selection_rect.max.x),
                selection_rect.max.y,
            ),
        ))
    }

    fn fade_out_rect(
        &self,
        bounds: Rect,
        selection: NormalizedRange,
        selection_rect: Rect,
    ) -> Option<Rect> {
        let start = self
            .edit_preview
            .trailing_start_micros
            .unwrap_or(selection.end_micros);
        if start >= selection.end_micros {
            return None;
        }
        let x = self.x_for_micros(bounds, start)?;
        Some(Rect::from_min_max(
            Point::new(
                x.clamp(selection_rect.min.x, selection_rect.max.x),
                selection_rect.min.y,
            ),
            Point::new(selection_rect.max.x, selection_rect.max.y),
        ))
    }

    fn fade_in_outer_rect(
        &self,
        bounds: Rect,
        selection: NormalizedRange,
        selection_rect: Rect,
    ) -> Option<Rect> {
        let start = self.edit_preview.leading_inner_start_micros?;
        if start >= selection.start_micros {
            return None;
        }
        let x = self.x_for_micros(bounds, start)?;
        Some(Rect::from_min_max(
            Point::new(
                x.clamp(bounds.min.x, selection_rect.min.x),
                selection_rect.min.y,
            ),
            Point::new(selection_rect.min.x, selection_rect.max.y),
        ))
    }

    fn fade_out_outer_rect(
        &self,
        bounds: Rect,
        selection: NormalizedRange,
        selection_rect: Rect,
    ) -> Option<Rect> {
        let end = self.edit_preview.trailing_inner_end_micros?;
        if end <= selection.end_micros {
            return None;
        }
        let x = self.x_for_micros(bounds, end)?;
        Some(Rect::from_min_max(
            Point::new(selection_rect.max.x, selection_rect.min.y),
            Point::new(
                x.clamp(selection_rect.max.x, bounds.max.x),
                selection_rect.max.y,
            ),
        ))
    }

    fn edit_fade_handle_rect(
        &self,
        bounds: Rect,
        selection_rect: Rect,
        handle: WaveformEditFadeHandle,
    ) -> Option<Rect> {
        let selection = self.edit_preview.selection?;
        let micros = match handle {
            WaveformEditFadeHandle::FadeInEnd => self
                .edit_preview
                .leading_end_micros
                .unwrap_or(selection.start_micros),
            WaveformEditFadeHandle::FadeOutStart => self
                .edit_preview
                .trailing_start_micros
                .unwrap_or(selection.end_micros),
            WaveformEditFadeHandle::FadeInStart => self
                .edit_preview
                .leading_end_micros
                .map(|_| selection.start_micros)?,
            WaveformEditFadeHandle::FadeOutEnd => self
                .edit_preview
                .trailing_start_micros
                .map(|_| selection.end_micros)?,
            WaveformEditFadeHandle::FadeInOuterStart => self.edit_preview.leading_end_micros.and(
                self.edit_preview
                    .leading_inner_start_micros
                    .or(Some(selection.start_micros)),
            )?,
            WaveformEditFadeHandle::FadeOutOuterEnd => {
                self.edit_preview.trailing_start_micros.and(
                    self.edit_preview
                        .trailing_inner_end_micros
                        .or(Some(selection.end_micros)),
                )?
            }
        };
        let x = self.x_for_micros(bounds, micros)?;
        let size = EDIT_FADE_HANDLE_TAB_SIZE
            .max(EDIT_FADE_HANDLE_WIDTH)
            .min(bounds.width().max(1.0))
            .min(bounds.height().max(1.0));
        let half = size * 0.5;
        let left = (x - half).clamp(bounds.min.x, bounds.max.x - size.max(1.0));
        let right = (left + size).min(bounds.max.x).max(left + 1.0);
        let (top, bottom) = match handle {
            WaveformEditFadeHandle::FadeInEnd | WaveformEditFadeHandle::FadeOutStart => {
                let bottom = (selection_rect.min.y + size)
                    .min(selection_rect.max.y)
                    .max(selection_rect.min.y + 1.0);
                (selection_rect.min.y, bottom)
            }
            WaveformEditFadeHandle::FadeInStart | WaveformEditFadeHandle::FadeOutEnd => {
                let top = (selection_rect.max.y - size)
                    .max(selection_rect.min.y)
                    .min(selection_rect.max.y - 1.0);
                (top, selection_rect.max.y)
            }
            WaveformEditFadeHandle::FadeInOuterStart | WaveformEditFadeHandle::FadeOutOuterEnd => {
                let center_y = selection_rect.center().y;
                let top = (center_y - half)
                    .max(selection_rect.min.y)
                    .min(selection_rect.max.y - 1.0);
                let bottom = (top + size).min(selection_rect.max.y).max(top + 1.0);
                (top, bottom)
            }
        };
        Some(Rect::from_min_max(
            Point::new(left, top),
            Point::new(right, bottom),
        ))
    }

    fn visible_rect_for_normalized_range(
        &self,
        bounds: Rect,
        range: NormalizedRange,
    ) -> Option<Rect> {
        let start = self.x_for_micros(bounds, range.start_micros)?;
        let end = self.x_for_micros(bounds, range.end_micros)?;
        let min_x = start.min(end).max(bounds.min.x);
        let max_x = start.max(end).min(bounds.max.x);
        if max_x <= min_x {
            return None;
        }
        Some(Rect::from_min_max(
            Point::new(min_x, bounds.min.y),
            Point::new(max_x, bounds.max.y),
        ))
    }

    fn x_for_micros(&self, bounds: Rect, micros: u32) -> Option<f32> {
        let ratio = micros.min(1_000_000) as f32 / 1_000_000.0;
        let visible_ratio = self.visible_ratio_for_absolute(Some(ratio))?;
        Some(bounds.min.x + bounds.width() * visible_ratio)
    }
}
