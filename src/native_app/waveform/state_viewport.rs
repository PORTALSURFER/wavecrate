use radiant::gui::types::Vector2;
use wavecrate::selection::SelectionRange;

use super::{MIN_VISIBLE_FRAMES, WAVEFORM_WIDTH, WaveformState, interaction::WaveformPanDrag};

const KEYBOARD_SELECTION_ZOOM_FACTOR: f32 = 0.82;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectionFrameRange {
    start: i64,
    end: i64,
}

impl WaveformState {
    pub(super) fn absolute_ratio_from_visible(&self, visible_ratio: f32) -> f32 {
        self.viewport
            .absolute_ratio_from_visible(self.file.frames, visible_ratio)
    }

    pub(super) fn handle_wheel(
        &mut self,
        delta: Vector2,
        anchor_ratio: f32,
        expand_silence_margin: bool,
    ) {
        if delta.x.abs() > delta.y.abs() && delta.x.abs() > f32::EPSILON {
            self.pan_by_visible_fraction(delta.x / WAVEFORM_WIDTH as f32);
            return;
        }
        if delta.y < -f32::EPSILON {
            self.zoom_around_anchor(0.82, anchor_ratio, true);
        } else if delta.y > f32::EPSILON {
            if !expand_silence_margin && self.viewport.extends_beyond_audio(self.file.frames) {
                return;
            }
            self.zoom_around_anchor(1.22, anchor_ratio, expand_silence_margin);
        }
    }

    pub(super) fn set_offset_fraction(&mut self, offset_fraction: f32) {
        self.viewport = self.viewport.with_offset_fraction(
            self.file.frames,
            MIN_VISIBLE_FRAMES,
            offset_fraction,
        );
    }

    pub(in crate::native_app) fn zoom_to_play_selection(&mut self) {
        if let Some(viewport) = self.play_selection_zoom_viewport() {
            self.viewport = viewport;
            return;
        }
        let anchor_ratio = self
            .play_mark_ratio
            .and_then(|ratio| self.visible_ratio_for_absolute(ratio))
            .unwrap_or(self.zoom_anchor_ratio);
        self.zoom_around_anchor(KEYBOARD_SELECTION_ZOOM_FACTOR, anchor_ratio, true);
    }

    pub(in crate::native_app) fn ensure_play_selection_visible(&mut self) {
        let Some(selection) = self
            .play_selection
            .filter(|selection| selection.width_f64() > f64::EPSILON)
        else {
            return;
        };
        self.viewport = self.viewport_containing_selection(selection);
    }

    pub(in crate::native_app) fn zoom_full(&mut self) {
        self.viewport = super::WaveformViewport::full(self.file.frames);
    }

    pub(super) fn update_active_pan(&mut self, drag: WaveformPanDrag, visible_ratio: f32) {
        self.viewport = drag.viewport.pan_by_visible_ratio_drag(
            self.file.frames,
            MIN_VISIBLE_FRAMES,
            drag.anchor_visible_ratio,
            visible_ratio,
        );
    }

    fn zoom_around_anchor(&mut self, factor: f32, anchor_ratio: f32, allow_silence_margin: bool) {
        self.viewport = self.viewport.zoom_around_anchor(
            self.file.frames,
            MIN_VISIBLE_FRAMES,
            factor,
            anchor_ratio,
            allow_silence_margin,
        );
    }

    fn pan_by_visible_fraction(&mut self, fraction: f32) {
        self.viewport =
            self.viewport
                .pan_by_visible_fraction(self.file.frames, MIN_VISIBLE_FRAMES, fraction);
    }

    fn play_selection_zoom_viewport(&self) -> Option<super::WaveformViewport> {
        let selection = self
            .play_selection
            .filter(|selection| selection.width_f64() > f64::EPSILON)?;
        Some(self.viewport_for_selection(selection))
    }

    fn viewport_for_selection(&self, selection: SelectionRange) -> super::WaveformViewport {
        let selection_frames = self.selection_frame_range(selection);
        self.viewport_for_selection_frames(selection, selection_frames)
    }

    fn viewport_containing_selection(&self, selection: SelectionRange) -> super::WaveformViewport {
        let current = self
            .viewport
            .clamp_to_current_domain(self.file.frames, MIN_VISIBLE_FRAMES);
        let selection_frames = self.selection_frame_range(selection);
        if viewport_contains_frame_range(current, selection_frames) {
            return current;
        }

        if selection_frames.visible_items() > current.visible_items() {
            return self.viewport_for_selection_frames(selection, selection_frames);
        }

        self.viewport_centered_on_selection_with_visible_span(selection, current.visible_items())
    }

    fn viewport_for_selection_frames(
        &self,
        selection: SelectionRange,
        selection_frames: SelectionFrameRange,
    ) -> super::WaveformViewport {
        let total_frames = self.file.frames.max(1);
        let min_visible_frames = MIN_VISIBLE_FRAMES.min(total_frames);
        let max_visible_frames = max_visible_frames_for_selection(total_frames, selection_frames);
        let selected_frames = selection_frames
            .visible_items()
            .max(1)
            .clamp(min_visible_frames, max_visible_frames);
        let center = selection_center_frame(selection, total_frames);
        let start = (center - selected_frames as f64 * 0.5).round() as i64;
        super::WaveformViewport {
            start,
            end: start + selected_frames as i64,
        }
        .clamp(total_frames, MIN_VISIBLE_FRAMES, max_visible_frames)
    }

    fn viewport_centered_on_selection_with_visible_span(
        &self,
        selection: SelectionRange,
        visible_frames: usize,
    ) -> super::WaveformViewport {
        let total_frames = self.file.frames.max(1);
        let selection_frames = self.selection_frame_range(selection);
        let max_visible_frames =
            max_visible_frames_for_selection(total_frames, selection_frames).max(visible_frames);
        let visible_frames =
            visible_frames.clamp(MIN_VISIBLE_FRAMES.min(total_frames), max_visible_frames);
        let center = selection_center_frame(selection, total_frames);
        let start = (center - visible_frames as f64 * 0.5).round() as i64;
        super::WaveformViewport {
            start,
            end: start + visible_frames as i64,
        }
        .clamp(total_frames, MIN_VISIBLE_FRAMES, max_visible_frames)
    }

    fn selection_frame_range(&self, selection: SelectionRange) -> SelectionFrameRange {
        let total_frames = self.file.frames.max(1);
        let bounds = selection.signed_frame_bounds(total_frames);
        SelectionFrameRange {
            start: bounds.start_frame,
            end: bounds.end_frame,
        }
    }
}

impl SelectionFrameRange {
    fn visible_items(self) -> usize {
        self.end.saturating_sub(self.start).max(1) as usize
    }
}

fn viewport_contains_frame_range(
    viewport: super::WaveformViewport,
    range: SelectionFrameRange,
) -> bool {
    range.start >= viewport.start && range.end <= viewport.end
}

fn max_visible_frames_for_selection(
    total_frames: usize,
    selection_frames: SelectionFrameRange,
) -> usize {
    if selection_frames.start < 0 || selection_frames.end > total_frames.max(1) as i64 {
        super::WaveformViewport::virtual_max_visible_items(total_frames)
    } else {
        total_frames.max(1)
    }
}

fn selection_center_frame(selection: SelectionRange, total_frames: usize) -> f64 {
    ((selection.start_f64() + selection.end_f64()) * 0.5 * total_frames.max(1) as f64).round()
}
