use radiant::{
    gui::{
        types::{Point, Rect, Rgba8},
        visualization::CanvasSelectionGeometry,
    },
    runtime::{PaintTextAlign, WidgetPaint},
};

use crate::ui_formatting::{format_selection_duration, format_waveform_bpm_input};

use super::WaveformWidget;

const PLAYMARK_LABEL_HEIGHT: f32 = 18.0;
const PLAYMARK_LABEL_BOTTOM_INSET: f32 = 2.0;
const PLAYMARK_LABEL_HORIZONTAL_PADDING: f32 = 10.0;
const PLAYMARK_LABEL_GLYPH_WIDTH: f32 = 10.0;
const PLAYMARK_LABEL_MIN_WIDTH: f32 = 80.0;
const PLAYMARK_LABEL_MAX_WIDTH: f32 = 150.0;
const PLAYMARK_LABEL_BACKGROUND: Rgba8 = Rgba8::new(25, 18, 16, 214);
const PLAYMARK_LABEL_TEXT: Rgba8 = Rgba8::new(255, 226, 210, 255);

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
        let Some(label_rect) = playmark_label_rect(bounds, geometry.rect, label.len()) else {
            return;
        };

        paint.push_visible_fill_rect(label_rect, PLAYMARK_LABEL_BACKGROUND);
        paint.push_text(
            label,
            label_rect,
            PLAYMARK_LABEL_TEXT,
            PaintTextAlign::Center,
        );
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

fn playmark_label_rect(bounds: Rect, selection_rect: Rect, label_len: usize) -> Option<Rect> {
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return None;
    }

    let desired_width =
        label_len as f32 * PLAYMARK_LABEL_GLYPH_WIDTH + PLAYMARK_LABEL_HORIZONTAL_PADDING * 2.0;
    let width = desired_width
        .clamp(PLAYMARK_LABEL_MIN_WIDTH, PLAYMARK_LABEL_MAX_WIDTH)
        .min(bounds.width());
    let center_x = selection_rect
        .center()
        .x
        .clamp(bounds.min.x + width * 0.5, bounds.max.x - width * 0.5);
    let bottom = (bounds.max.y - PLAYMARK_LABEL_BOTTOM_INSET).max(bounds.min.y);
    let top = (bottom - PLAYMARK_LABEL_HEIGHT).max(bounds.min.y);

    Some(Rect::from_min_max(
        Point::new(center_x - width * 0.5, top),
        Point::new(center_x + width * 0.5, bottom),
    ))
}
