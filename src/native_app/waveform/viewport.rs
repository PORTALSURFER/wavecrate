use radiant::prelude as ui;

const MAX_SILENCE_MARGIN_VIEWPORT_FACTOR: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct WaveformViewport {
    pub(in crate::native_app) start: i64,
    pub(in crate::native_app) end: i64,
}

impl WaveformViewport {
    pub(super) fn full(total_frames: usize) -> Self {
        Self {
            start: 0,
            end: total_frames.max(1) as i64,
        }
    }

    pub(super) fn visible_items(self) -> usize {
        self.end.saturating_sub(self.start).max(1) as usize
    }

    pub(super) fn frame_range(self) -> [f32; 2] {
        [self.start as f32, self.end as f32]
    }

    pub(super) fn visible_fraction(self, total_frames: usize) -> f32 {
        (self.visible_items() as f32 / total_frames.max(1) as f32).min(1.0)
    }

    pub(super) fn offset_fraction(self, total_frames: usize) -> f32 {
        let total_frames = total_frames.max(1) as i64;
        let visible = self.visible_items() as i64;
        if visible >= total_frames {
            return 0.0;
        }
        let free = total_frames - visible;
        if free <= 0 {
            0.0
        } else {
            (self.start.clamp(0, free) as f32 / free as f32).clamp(0.0, 1.0)
        }
    }

    pub(super) fn is_zoomed_in(self, total_frames: usize) -> bool {
        self.visible_items() < total_frames.max(1)
    }

    pub(super) fn extends_beyond_audio(self, total_frames: usize) -> bool {
        let total_frames = total_frames.max(1) as i64;
        self.start < 0 || self.end > total_frames
    }

    pub(super) fn clamp_to_audio(self, total_frames: usize, min_visible_frames: usize) -> Self {
        self.clamp(total_frames, min_visible_frames, total_frames.max(1))
    }

    pub(super) fn clamp_to_current_domain(
        self,
        total_frames: usize,
        min_visible_frames: usize,
    ) -> Self {
        let max_visible = if self.extends_beyond_audio(total_frames) {
            Self::virtual_max_visible_items(total_frames)
        } else {
            total_frames.max(1)
        };
        self.clamp(total_frames, min_visible_frames, max_visible)
    }

    pub(super) fn zoom_around_anchor(
        self,
        total_frames: usize,
        min_visible_frames: usize,
        factor: f32,
        anchor_ratio: f32,
        allow_silence_margin: bool,
    ) -> Self {
        let total_frames = total_frames.max(1);
        let max_visible = if allow_silence_margin {
            Self::virtual_max_visible_items(total_frames)
        } else {
            total_frames
        };
        let viewport = self.clamp(total_frames, min_visible_frames, max_visible);
        let visible = viewport.visible_items();
        let factor = finite_positive_or(factor, 1.0);
        let anchor_ratio = finite_unit_or(anchor_ratio, 0.5);
        let next_visible = ((visible as f32) * factor).round().clamp(
            min_visible_frames.max(1).min(total_frames) as f32,
            max_visible.max(1) as f32,
        ) as usize;
        let anchor_frame = viewport.start as f64 + visible as f64 * f64::from(anchor_ratio);
        let start = (anchor_frame - next_visible as f64 * f64::from(anchor_ratio)).round() as i64;
        Self {
            start,
            end: start + next_visible as i64,
        }
        .clamp(total_frames, min_visible_frames, max_visible)
    }

    pub(super) fn pan_by_visible_fraction(
        self,
        total_frames: usize,
        min_visible_frames: usize,
        fraction: f32,
    ) -> Self {
        let total_frames = total_frames.max(1);
        let max_visible = if self.extends_beyond_audio(total_frames) {
            Self::virtual_max_visible_items(total_frames)
        } else {
            total_frames
        };
        let viewport = self.clamp(total_frames, min_visible_frames, max_visible);
        let fraction = if fraction.is_finite() { fraction } else { 0.0 };
        let delta = (viewport.visible_items() as f32 * fraction).round() as i64;
        Self {
            start: viewport.start + delta,
            end: viewport.end + delta,
        }
        .clamp(total_frames, min_visible_frames, max_visible)
    }

    pub(super) fn pan_by_visible_ratio_drag(
        self,
        total_frames: usize,
        min_visible_frames: usize,
        anchor_ratio: f32,
        current_ratio: f32,
    ) -> Self {
        let total_frames = total_frames.max(1);
        let max_visible = if self.extends_beyond_audio(total_frames) {
            Self::virtual_max_visible_items(total_frames)
        } else {
            total_frames
        };
        let viewport = self.clamp(total_frames, min_visible_frames, max_visible);
        let visible = viewport.visible_items();
        if visible >= total_frames && !viewport.extends_beyond_audio(total_frames) {
            return viewport;
        }
        let anchor_ratio = finite_unit_or(anchor_ratio, 0.0);
        let current_ratio = finite_unit_or(current_ratio, anchor_ratio);
        let delta = ((current_ratio - anchor_ratio) * visible as f32).round() as i64;
        Self {
            start: viewport.start - delta,
            end: viewport.end - delta,
        }
        .clamp(total_frames, min_visible_frames, max_visible)
    }

    pub(super) fn with_offset_fraction(
        self,
        total_frames: usize,
        min_visible_frames: usize,
        offset_fraction: f32,
    ) -> Self {
        let viewport = self.clamp_to_audio(total_frames, min_visible_frames);
        let total_frames = total_frames.max(1) as i64;
        let visible = viewport.visible_items() as i64;
        let free = total_frames.saturating_sub(visible);
        let offset_fraction = finite_unit_or(offset_fraction, 0.0);
        let start = (free as f32 * offset_fraction).round() as i64;
        Self {
            start,
            end: start + visible,
        }
        .clamp_to_audio(total_frames as usize, min_visible_frames)
    }

    pub(super) fn absolute_ratio_from_visible(
        self,
        total_frames: usize,
        visible_ratio: f32,
    ) -> f32 {
        let visible_ratio = finite_unit_or(visible_ratio, 0.0);
        let frame = self.start as f64 + self.visible_items() as f64 * f64::from(visible_ratio);
        (frame / total_frames.max(1) as f64) as f32
    }

    pub(super) fn visible_ratio_from_absolute(
        self,
        total_frames: usize,
        absolute_ratio: f32,
    ) -> Option<f32> {
        if !absolute_ratio.is_finite() {
            return None;
        }
        let frame = f64::from(absolute_ratio) * total_frames.max(1) as f64;
        let visible_start = self.start as f64;
        let visible_width = self.visible_items() as f64;
        let visible_ratio = (frame - visible_start) / visible_width.max(1.0);
        if !(-f64::EPSILON..=1.0 + f64::EPSILON).contains(&visible_ratio) {
            return None;
        }
        Some(visible_ratio.clamp(0.0, 1.0) as f32)
    }

    pub(super) fn visible_range_from_absolute(
        self,
        total_frames: usize,
        start_ratio: f32,
        end_ratio: f32,
    ) -> Option<(f32, f32)> {
        if !start_ratio.is_finite() || !end_ratio.is_finite() {
            return None;
        }
        let total_frames = total_frames.max(1) as f64;
        let visible_start = self.start as f64;
        let visible_end = self.end as f64;
        let visible_width = self.visible_items() as f64;
        let start_frame = f64::from(start_ratio) * total_frames;
        let end_frame = f64::from(end_ratio) * total_frames;
        let left = start_frame.min(end_frame).max(visible_start);
        let right = start_frame.max(end_frame).min(visible_end);
        if right <= left {
            return None;
        }
        Some((
            ((left - visible_start) / visible_width.max(1.0)).clamp(0.0, 1.0) as f32,
            ((right - visible_start) / visible_width.max(1.0)).clamp(0.0, 1.0) as f32,
        ))
    }

    pub(super) fn clamped_index_viewport(
        self,
        total_frames: usize,
        min_visible_frames: usize,
    ) -> ui::IndexViewport {
        let viewport = self.clamp_to_audio(total_frames, min_visible_frames);
        ui::IndexViewport {
            start: viewport.start.max(0) as usize,
            end: viewport.end.max(viewport.start + 1).max(1) as usize,
        }
    }

    pub(super) fn virtual_max_visible_items(total_frames: usize) -> usize {
        ((total_frames.max(1) as f32) * MAX_SILENCE_MARGIN_VIEWPORT_FACTOR)
            .round()
            .max(total_frames.max(1) as f32) as usize
    }

    pub(super) fn clamp(
        self,
        total_frames: usize,
        min_visible_frames: usize,
        max_visible_frames: usize,
    ) -> Self {
        let total_frames = total_frames.max(1) as i64;
        let min_visible_frames = min_visible_frames.max(1).min(total_frames as usize);
        let max_visible_frames = max_visible_frames.max(min_visible_frames).max(1);
        let visible = self
            .visible_items()
            .clamp(min_visible_frames, max_visible_frames) as i64;
        let (min_start, max_start) = if visible >= total_frames {
            (total_frames - visible, 0)
        } else {
            (0, total_frames - visible)
        };
        let start = self.start.clamp(min_start, max_start);
        Self {
            start,
            end: start + visible,
        }
    }
}

fn finite_unit_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

fn finite_positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > f32::EPSILON {
        value
    } else {
        fallback
    }
}
