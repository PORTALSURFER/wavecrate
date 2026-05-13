#![allow(missing_docs)]

use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    gui::{range::NormalizedRange, visualization::TimelineEditPreview},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle, GpuSurfaceRuntimeOverlays,
        PaintFillRect, PaintGpuSurface, PaintPrimitive,
    },
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, PaintBounds, PointerButton, Widget, WidgetCommon, WidgetInput, WidgetOutput,
        WidgetSizing,
    },
};
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io::Cursor,
    path::PathBuf,
    sync::Arc,
};

use super::GuiMessage;

const WAVEFORM_WIDTH: usize = 1200;
const WAVEFORM_HEIGHT: usize = 320;
const MIN_VISIBLE_FRAMES: usize = 256;
const BAND_COUNT: usize = 4;
const SELECTION_DRAG_EPSILON: f32 = 0.001;
const EDIT_FADE_HANDLE_TAB_SIZE: f32 = 10.0;
const EDIT_FADE_HANDLE_WIDTH: f32 = 3.0;
#[cfg(test)]
const SYNTHETIC_SAMPLE_RATE: u32 = 48_000;
#[cfg(test)]
const SYNTHETIC_SECONDS: usize = 1;

#[derive(Clone, Debug)]
pub(super) struct WaveformState {
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    zoom_anchor_ratio: f32,
    playing: bool,
    playhead_ratio: Option<f32>,
    play_mark_ratio: Option<f32>,
    edit_mark_ratio: Option<f32>,
    play_selection: Option<wavecrate::selection::SelectionRange>,
    edit_selection: Option<wavecrate::selection::SelectionRange>,
    active_drag: Option<WaveformDrag>,
    pending_playback_start: Option<f32>,
}

impl WaveformState {
    pub(super) fn load_default() -> Result<Self, String> {
        Self::load_path(default_sample_path())
    }

    pub(super) fn load_path(path: PathBuf) -> Result<Self, String> {
        let file = Arc::new(load_waveform_file(path)?);
        Ok(Self::from_file(file))
    }

    #[cfg(test)]
    pub(super) fn synthetic_for_tests() -> Self {
        Self::from_file(Arc::new(synthetic_waveform_file()))
    }

    fn from_file(file: Arc<WaveformFile>) -> Self {
        let viewport = WaveformViewport::full(file.frames);
        Self {
            file,
            viewport,
            zoom_anchor_ratio: 0.5,
            playing: false,
            playhead_ratio: None,
            play_mark_ratio: None,
            edit_mark_ratio: None,
            play_selection: None,
            edit_selection: None,
            active_drag: None,
            pending_playback_start: None,
        }
    }

    pub(super) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(super) fn file(&self) -> Arc<WaveformFile> {
        Arc::clone(&self.file)
    }

    pub(super) fn viewport(&self) -> WaveformViewport {
        self.viewport
    }

    pub(super) fn cursor_ratio(&self) -> Option<f32> {
        Some(self.zoom_anchor_ratio)
    }

    pub(super) fn playhead_ratio(&self) -> Option<f32> {
        self.playhead_ratio
    }

    pub(super) fn play_mark_ratio(&self) -> Option<f32> {
        self.play_mark_ratio
    }

    pub(super) fn edit_mark_ratio(&self) -> Option<f32> {
        self.edit_mark_ratio
    }

    pub(super) fn play_selection(&self) -> Option<wavecrate::selection::SelectionRange> {
        self.play_selection
    }

    pub(super) fn edit_selection(&self) -> Option<wavecrate::selection::SelectionRange> {
        self.edit_selection
    }

    fn active_drag_kind(&self) -> Option<WaveformActiveDragKind> {
        self.active_drag.map(WaveformDrag::kind)
    }

    pub(super) fn take_pending_playback_start(&mut self) -> Option<f32> {
        self.pending_playback_start.take()
    }

    pub(super) fn start_playback(&mut self, ratio: f32) {
        let ratio = ratio.clamp(0.0, 1.0);
        self.playing = true;
        self.play_mark_ratio = Some(ratio);
        self.playhead_ratio = Some(ratio);
        self.zoom_anchor_ratio = ratio;
    }

    pub(super) fn set_playhead_ratio(&mut self, ratio: f32) {
        let ratio = ratio.clamp(0.0, 1.0);
        self.playhead_ratio = Some(ratio);
        self.zoom_anchor_ratio = ratio;
    }

    pub(super) fn stop_playback(&mut self) {
        self.playing = false;
        self.playhead_ratio = None;
    }

    pub(super) fn sample_rate(&self) -> u32 {
        self.file.sample_rate
    }

    pub(super) fn channels(&self) -> usize {
        self.file.channels
    }

    pub(super) fn frames(&self) -> usize {
        self.file.frames
    }

    pub(super) fn file_name(&self) -> String {
        self.file
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| self.file.path.display().to_string())
    }

    pub(super) fn path(&self) -> PathBuf {
        self.file.path.clone()
    }

    pub(super) fn audio_bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.file.audio_bytes)
    }

    pub(super) fn visible_fraction(&self) -> f32 {
        self.viewport.visible_fraction(self.file.frames)
    }

    pub(super) fn offset_fraction(&self) -> f32 {
        self.viewport.offset_fraction(self.file.frames)
    }

    pub(super) fn apply_interaction(&mut self, interaction: WaveformInteraction) {
        match interaction {
            WaveformInteraction::Wheel {
                delta,
                anchor_ratio,
            } => {
                self.zoom_anchor_ratio = anchor_ratio;
                self.handle_wheel(delta, anchor_ratio);
            }
            WaveformInteraction::ScrollTo { offset_fraction } => {
                self.set_offset_fraction(offset_fraction);
            }
            WaveformInteraction::BeginSelection {
                kind,
                visible_ratio,
            } => {
                let ratio = self.absolute_ratio_from_visible(visible_ratio);
                self.active_drag = Some(WaveformDrag::Selection(WaveformSelectionDrag::new(
                    kind, ratio,
                )));
                match kind {
                    WaveformSelectionKind::Play => {
                        self.play_mark_ratio = Some(ratio);
                        self.play_selection = None;
                    }
                    WaveformSelectionKind::Edit => {
                        self.edit_mark_ratio = Some(ratio);
                        self.edit_selection = None;
                    }
                }
            }
            WaveformInteraction::BeginEditFade {
                handle,
                visible_ratio,
            } => {
                let Some(selection) = self.edit_selection else {
                    return;
                };
                let ratio = self.absolute_ratio_from_visible(visible_ratio);
                self.active_drag = Some(WaveformDrag::EditFade(WaveformEditFadeDrag::new(
                    handle, selection,
                )));
                self.update_active_edit_fade(ratio);
            }
            WaveformInteraction::BeginSelectionResize {
                kind,
                edge,
                visible_ratio,
            } => {
                let Some(selection) = self.selection_for_kind(kind) else {
                    return;
                };
                let ratio = self.absolute_ratio_from_visible(visible_ratio);
                self.active_drag = Some(WaveformDrag::SelectionResize(
                    WaveformSelectionResizeDrag::new(kind, edge, selection),
                ));
                self.update_active_selection_resize(ratio);
            }
            WaveformInteraction::BeginPan { visible_ratio } => {
                self.active_drag = Some(WaveformDrag::Pan(WaveformPanDrag::new(
                    visible_ratio,
                    self.viewport.clamp(self.file.frames.max(1)),
                )));
            }
            WaveformInteraction::UpdateSelection { visible_ratio } => {
                self.update_active_drag(visible_ratio);
            }
            WaveformInteraction::FinishSelection { visible_ratio } => {
                self.finish_active_drag(visible_ratio);
            }
            WaveformInteraction::Frame => {
                // Playback progress is driven by the audio engine; frames only keep repainting.
            }
        }
    }

    pub(super) fn absolute_ratio_from_visible(&self, visible_ratio: f32) -> f32 {
        let total = self.file.frames.max(1);
        let viewport = self.viewport.clamp(total);
        let visible_ratio = visible_ratio.clamp(0.0, 1.0);
        let frame = viewport.start as f64 + viewport.visible_frames() as f64 * visible_ratio as f64;
        ((frame / total as f64) as f32).clamp(0.0, 1.0)
    }

    fn handle_wheel(&mut self, delta: Vector2, anchor_ratio: f32) {
        if delta.x.abs() > delta.y.abs() && delta.x.abs() > f32::EPSILON {
            self.pan_by_visible_fraction(delta.x / WAVEFORM_WIDTH as f32);
            return;
        }
        if delta.y < -f32::EPSILON {
            self.zoom_around_anchor(0.82, anchor_ratio);
        } else if delta.y > f32::EPSILON {
            self.zoom_around_anchor(1.22, anchor_ratio);
        }
    }

    fn zoom_around_anchor(&mut self, factor: f32, anchor_ratio: f32) {
        let total = self.file.frames.max(1);
        let current = self.viewport.clamp(total);
        let anchor_ratio = anchor_ratio.clamp(0.0, 1.0);
        let anchor_frame = current.start as f32 + current.visible_frames() as f32 * anchor_ratio;
        let next_visible = ((current.visible_frames() as f32) * factor)
            .round()
            .clamp(MIN_VISIBLE_FRAMES.min(total) as f32, total as f32)
            as usize;
        let start = (anchor_frame - next_visible as f32 * anchor_ratio)
            .round()
            .max(0.0) as usize;
        self.viewport = WaveformViewport {
            start,
            end: start + next_visible,
        }
        .clamp(total);
    }

    fn pan_by_visible_fraction(&mut self, fraction: f32) {
        let total = self.file.frames.max(1);
        let current = self.viewport.clamp(total);
        let delta = (current.visible_frames() as f32 * fraction).round() as isize;
        let start = current.start.saturating_add_signed(delta);
        self.viewport = WaveformViewport {
            start,
            end: start + current.visible_frames(),
        }
        .clamp(total);
    }

    fn set_offset_fraction(&mut self, offset_fraction: f32) {
        let total = self.file.frames.max(1);
        let current = self.viewport.clamp(total);
        let visible = current.visible_frames();
        let free_frames = total.saturating_sub(visible);
        let start = (free_frames as f32 * offset_fraction.clamp(0.0, 1.0)).round() as usize;
        self.viewport = WaveformViewport {
            start,
            end: start + visible,
        }
        .clamp(total);
    }

    fn update_active_drag(&mut self, visible_ratio: f32) {
        let ratio = self.absolute_ratio_from_visible(visible_ratio);
        let Some(drag) = self.active_drag else {
            return;
        };
        match drag {
            WaveformDrag::Selection(mut drag) => {
                drag.update(ratio);
                self.active_drag = Some(WaveformDrag::Selection(drag));
                if drag.moved {
                    self.set_selection_for_drag(drag);
                }
            }
            WaveformDrag::EditFade(_) => {
                self.update_active_edit_fade(ratio);
            }
            WaveformDrag::SelectionResize(_) => {
                self.update_active_selection_resize(ratio);
            }
            WaveformDrag::Pan(drag) => {
                self.update_active_pan(drag, visible_ratio);
            }
        }
    }

    fn finish_active_drag(&mut self, visible_ratio: f32) {
        let ratio = self.absolute_ratio_from_visible(visible_ratio);
        let Some(drag) = self.active_drag.take() else {
            return;
        };
        match drag {
            WaveformDrag::Selection(mut drag) => {
                drag.update(ratio);
                if drag.moved {
                    self.set_selection_for_drag(drag);
                    return;
                }
                match drag.kind {
                    WaveformSelectionKind::Play => {
                        self.play_selection = None;
                        self.start_playback(ratio);
                        self.pending_playback_start = Some(ratio);
                    }
                    WaveformSelectionKind::Edit => {
                        self.edit_selection = None;
                        self.edit_mark_ratio = Some(ratio);
                    }
                }
            }
            WaveformDrag::EditFade(_) => {
                self.active_drag = Some(drag);
                self.update_active_edit_fade(ratio);
                self.active_drag = None;
            }
            WaveformDrag::SelectionResize(_) => {
                self.active_drag = Some(drag);
                self.update_active_selection_resize(ratio);
                self.active_drag = None;
            }
            WaveformDrag::Pan(drag) => {
                self.update_active_pan(drag, visible_ratio);
            }
        }
    }

    fn set_selection_for_drag(&mut self, drag: WaveformSelectionDrag) {
        let range = wavecrate::selection::SelectionRange::new(drag.anchor_ratio, drag.current_ratio);
        match drag.kind {
            WaveformSelectionKind::Play => {
                self.play_mark_ratio = Some(drag.anchor_ratio);
                self.play_selection = Some(range);
            }
            WaveformSelectionKind::Edit => {
                self.edit_mark_ratio = Some(drag.anchor_ratio);
                self.edit_selection = Some(range);
            }
        }
    }

    fn update_active_edit_fade(&mut self, ratio: f32) {
        let Some(WaveformDrag::EditFade(drag)) = self.active_drag else {
            return;
        };
        let Some(selection) = self.edit_selection else {
            return;
        };
        self.edit_selection = Some(drag.apply(selection, ratio));
    }

    fn update_active_selection_resize(&mut self, ratio: f32) {
        let Some(WaveformDrag::SelectionResize(drag)) = self.active_drag else {
            return;
        };
        let Some(selection) = self.selection_for_kind(drag.kind) else {
            return;
        };
        let selection = drag.apply(selection, ratio);
        match drag.kind {
            WaveformSelectionKind::Play => {
                self.play_mark_ratio = Some(selection.start());
                self.play_selection = Some(selection);
            }
            WaveformSelectionKind::Edit => {
                self.edit_mark_ratio = Some(selection.start());
                self.edit_selection = Some(selection);
            }
        }
    }

    fn update_active_pan(&mut self, drag: WaveformPanDrag, visible_ratio: f32) {
        let total = self.file.frames.max(1);
        let viewport = drag.viewport.clamp(total);
        let visible = viewport.visible_frames();
        if visible >= total {
            return;
        }
        let delta = ((visible_ratio - drag.anchor_visible_ratio) * visible as f32).round() as isize;
        let start = viewport.start.saturating_add_signed(-delta);
        self.viewport = WaveformViewport {
            start,
            end: start + visible,
        }
        .clamp(total);
    }

    fn selection_for_kind(
        &self,
        kind: WaveformSelectionKind,
    ) -> Option<sempal::selection::SelectionRange> {
        match kind {
            WaveformSelectionKind::Play => self.play_selection,
            WaveformSelectionKind::Edit => self.edit_selection,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum WaveformInteraction {
    Wheel {
        delta: Vector2,
        anchor_ratio: f32,
    },
    ScrollTo {
        offset_fraction: f32,
    },
    BeginSelection {
        kind: WaveformSelectionKind,
        visible_ratio: f32,
    },
    BeginEditFade {
        handle: WaveformEditFadeHandle,
        visible_ratio: f32,
    },
    BeginSelectionResize {
        kind: WaveformSelectionKind,
        edge: WaveformSelectionEdge,
        visible_ratio: f32,
    },
    BeginPan {
        visible_ratio: f32,
    },
    UpdateSelection {
        visible_ratio: f32,
    },
    FinishSelection {
        visible_ratio: f32,
    },
    Frame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaveformSelectionKind {
    Play,
    Edit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaveformSelectionEdge {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaveformEditFadeHandle {
    FadeInEnd,
    FadeInStart,
    FadeInOuterStart,
    FadeOutStart,
    FadeOutEnd,
    FadeOutOuterEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaveformActiveDragKind {
    Selection(WaveformSelectionKind),
    SelectionResize(WaveformSelectionKind, WaveformSelectionEdge),
    EditFade(WaveformEditFadeHandle),
    Pan,
}

#[derive(Clone, Copy, Debug)]
enum WaveformDrag {
    Selection(WaveformSelectionDrag),
    SelectionResize(WaveformSelectionResizeDrag),
    EditFade(WaveformEditFadeDrag),
    Pan(WaveformPanDrag),
}

impl WaveformDrag {
    fn kind(self) -> WaveformActiveDragKind {
        match self {
            WaveformDrag::Selection(drag) => WaveformActiveDragKind::Selection(drag.kind),
            WaveformDrag::SelectionResize(drag) => {
                WaveformActiveDragKind::SelectionResize(drag.kind, drag.edge)
            }
            WaveformDrag::EditFade(drag) => WaveformActiveDragKind::EditFade(drag.handle),
            WaveformDrag::Pan(_) => WaveformActiveDragKind::Pan,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct WaveformPanDrag {
    anchor_visible_ratio: f32,
    viewport: WaveformViewport,
}

impl WaveformPanDrag {
    fn new(anchor_visible_ratio: f32, viewport: WaveformViewport) -> Self {
        Self {
            anchor_visible_ratio,
            viewport,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct WaveformSelectionDrag {
    kind: WaveformSelectionKind,
    anchor_ratio: f32,
    current_ratio: f32,
    moved: bool,
}

impl WaveformSelectionDrag {
    fn new(kind: WaveformSelectionKind, ratio: f32) -> Self {
        Self {
            kind,
            anchor_ratio: ratio,
            current_ratio: ratio,
            moved: false,
        }
    }

    fn update(&mut self, ratio: f32) {
        self.current_ratio = ratio;
        self.moved |= (self.current_ratio - self.anchor_ratio).abs() > SELECTION_DRAG_EPSILON;
    }
}

#[derive(Clone, Copy, Debug)]
struct WaveformSelectionResizeDrag {
    kind: WaveformSelectionKind,
    edge: WaveformSelectionEdge,
    fixed_ratio: f32,
}

impl WaveformSelectionResizeDrag {
    fn new(
        kind: WaveformSelectionKind,
        edge: WaveformSelectionEdge,
        selection: sempal::selection::SelectionRange,
    ) -> Self {
        let fixed_ratio = match edge {
            WaveformSelectionEdge::Start => selection.end(),
            WaveformSelectionEdge::End => selection.start(),
        };
        Self {
            kind,
            edge,
            fixed_ratio,
        }
    }

    fn apply(
        self,
        _selection: sempal::selection::SelectionRange,
        ratio: f32,
    ) -> sempal::selection::SelectionRange {
        let ratio = ratio.clamp(0.0, 1.0);
        match self.edge {
            WaveformSelectionEdge::Start => {
                sempal::selection::SelectionRange::new(ratio, self.fixed_ratio)
            }
            WaveformSelectionEdge::End => {
                sempal::selection::SelectionRange::new(self.fixed_ratio, ratio)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct WaveformEditFadeDrag {
    handle: WaveformEditFadeHandle,
    fixed_ratio: f32,
    curve: f32,
    baseline: sempal::selection::SelectionRange,
}

impl WaveformEditFadeDrag {
    fn new(handle: WaveformEditFadeHandle, selection: wavecrate::selection::SelectionRange) -> Self {
        let curve = match handle {
            WaveformEditFadeHandle::FadeInEnd
            | WaveformEditFadeHandle::FadeInStart
            | WaveformEditFadeHandle::FadeInOuterStart => {
                selection.fade_in().map(|fade| fade.curve).unwrap_or(0.5)
            }
            WaveformEditFadeHandle::FadeOutStart
            | WaveformEditFadeHandle::FadeOutEnd
            | WaveformEditFadeHandle::FadeOutOuterEnd => {
                selection.fade_out().map(|fade| fade.curve).unwrap_or(0.5)
            }
        };
        let fixed_ratio = match handle {
            WaveformEditFadeHandle::FadeInStart => selection
                .fade_in()
                .map(|fade| selection.start() + selection.width() * fade.length)
                .unwrap_or(selection.start()),
            WaveformEditFadeHandle::FadeOutEnd => selection
                .fade_out()
                .map(|fade| selection.end() - selection.width() * fade.length)
                .unwrap_or(selection.end()),
            WaveformEditFadeHandle::FadeInEnd
            | WaveformEditFadeHandle::FadeOutStart
            | WaveformEditFadeHandle::FadeInOuterStart
            | WaveformEditFadeHandle::FadeOutOuterEnd => 0.0,
        };
        Self {
            handle,
            fixed_ratio,
            curve,
            baseline: selection,
        }
    }

    fn apply(
        self,
        selection: wavecrate::selection::SelectionRange,
        ratio: f32,
    ) -> wavecrate::selection::SelectionRange {
        let ratio = ratio.clamp(0.0, 1.0);
        match self.handle {
            WaveformEditFadeHandle::FadeInEnd => {
                resize_fade_in_end_with_collision(selection, self.baseline, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeOutStart => {
                resize_fade_out_start_with_collision(selection, self.baseline, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeInStart => {
                resize_fade_in_start(selection, self.fixed_ratio, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeOutEnd => {
                resize_fade_out_end(selection, self.fixed_ratio, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeInOuterStart => {
                resize_fade_in_outer_start(selection, ratio)
            }
            WaveformEditFadeHandle::FadeOutOuterEnd => resize_fade_out_outer_end(selection, ratio),
        }
    }
}

fn fade_in_length_for_end(selection: wavecrate::selection::SelectionRange, end_ratio: f32) -> f32 {
    if selection.width() <= f32::EPSILON {
        return 0.0;
    }
    ((end_ratio.clamp(selection.start(), selection.end()) - selection.start()) / selection.width())
        .clamp(0.0, 1.0)
}

fn fade_out_length_for_start(
    selection: wavecrate::selection::SelectionRange,
    start_ratio: f32,
) -> f32 {
    if selection.width() <= f32::EPSILON {
        return 0.0;
    }
    ((selection.end() - start_ratio.clamp(selection.start(), selection.end())) / selection.width())
        .clamp(0.0, 1.0)
}

fn resize_fade_in_end_with_collision(
    selection: sempal::selection::SelectionRange,
    baseline: sempal::selection::SelectionRange,
    end_ratio: f32,
    curve: f32,
) -> sempal::selection::SelectionRange {
    let width = selection.width();
    if width <= f32::EPSILON {
        return selection;
    }
    let start = selection.start();
    let end = selection.end();
    let fade_in_end = end_ratio.clamp(start, end);
    let fade_in_abs = fade_in_end - start;
    let baseline_fade_out_abs = baseline.fade_out().map_or(0.0, |fade| {
        (baseline.end() - (baseline.end() - baseline.width() * fade.length)).max(0.0)
    });
    let baseline_fade_out_start = end - baseline_fade_out_abs;
    let fade_out_abs = if fade_in_end > baseline_fade_out_start {
        (end - fade_in_end).max(0.0)
    } else {
        baseline_fade_out_abs
    };
    rebuild_edit_fades_for_same_range(
        selection,
        Some((fade_in_abs / width, curve)),
        fade_out_for_same_width(selection, baseline, fade_out_abs).map(|length| {
            (
                length,
                baseline.fade_out().map(|fade| fade.curve).unwrap_or(0.5),
            )
        }),
    )
}

fn resize_fade_out_start_with_collision(
    selection: sempal::selection::SelectionRange,
    baseline: sempal::selection::SelectionRange,
    start_ratio: f32,
    curve: f32,
) -> sempal::selection::SelectionRange {
    let width = selection.width();
    if width <= f32::EPSILON {
        return selection;
    }
    let start = selection.start();
    let end = selection.end();
    let fade_out_start = start_ratio.clamp(start, end);
    let fade_out_abs = end - fade_out_start;
    let baseline_fade_in_abs = baseline.fade_in().map_or(0.0, |fade| {
        ((baseline.start() + baseline.width() * fade.length) - baseline.start()).max(0.0)
    });
    let baseline_fade_in_end = start + baseline_fade_in_abs;
    let fade_in_abs = if fade_out_start < baseline_fade_in_end {
        (fade_out_start - start).max(0.0)
    } else {
        baseline_fade_in_abs
    };
    rebuild_edit_fades_for_same_range(
        selection,
        fade_in_for_same_width(selection, baseline, fade_in_abs).map(|length| {
            (
                length,
                baseline.fade_in().map(|fade| fade.curve).unwrap_or(0.5),
            )
        }),
        Some((fade_out_abs / width, curve)),
    )
}

fn resize_fade_in_outer_start(
    selection: sempal::selection::SelectionRange,
    outer_start_ratio: f32,
) -> sempal::selection::SelectionRange {
    let Some(fade) = selection.fade_in() else {
        return selection;
    };
    let width = selection.width();
    if width <= f32::EPSILON {
        return selection;
    }
    let outer_start = outer_start_ratio.clamp(0.0, selection.start());
    let mute =
        ((selection.start() - outer_start) / width).clamp(0.0, selection.max_fade_in_mute_length());
    selection
        .with_fade_in(fade.length, fade.curve)
        .with_fade_in_mute(mute)
}

fn resize_fade_out_outer_end(
    selection: sempal::selection::SelectionRange,
    outer_end_ratio: f32,
) -> sempal::selection::SelectionRange {
    let Some(fade) = selection.fade_out() else {
        return selection;
    };
    let width = selection.width();
    if width <= f32::EPSILON {
        return selection;
    }
    let outer_end = outer_end_ratio.clamp(selection.end(), 1.0);
    let mute =
        ((outer_end - selection.end()) / width).clamp(0.0, selection.max_fade_out_mute_length());
    selection
        .with_fade_out(fade.length, fade.curve)
        .with_fade_out_mute(mute)
}

fn rebuild_edit_fades_for_same_range(
    selection: sempal::selection::SelectionRange,
    fade_in: Option<(f32, f32)>,
    fade_out: Option<(f32, f32)>,
) -> sempal::selection::SelectionRange {
    let mut rebuilt = sempal::selection::SelectionRange::new(selection.start(), selection.end())
        .with_gain(selection.gain());
    if let Some((length, curve)) = fade_in {
        rebuilt = rebuilt.with_fade_in(length.clamp(0.0, 1.0), curve);
        if let Some(fade) = selection.fade_in().filter(|fade| fade.mute > 0.0) {
            rebuilt = rebuilt.with_fade_in_mute(fade.mute);
        }
    }
    if let Some((length, curve)) = fade_out {
        rebuilt = rebuilt.with_fade_out(length.clamp(0.0, 1.0), curve);
        if let Some(fade) = selection.fade_out().filter(|fade| fade.mute > 0.0) {
            rebuilt = rebuilt.with_fade_out_mute(fade.mute);
        }
    }
    rebuilt
}

fn fade_in_for_same_width(
    selection: sempal::selection::SelectionRange,
    baseline: sempal::selection::SelectionRange,
    fade_in_abs: f32,
) -> Option<f32> {
    baseline.fade_in()?;
    Some((fade_in_abs / selection.width().max(f32::EPSILON)).clamp(0.0, 1.0))
}

fn fade_out_for_same_width(
    selection: sempal::selection::SelectionRange,
    baseline: sempal::selection::SelectionRange,
    fade_out_abs: f32,
) -> Option<f32> {
    baseline.fade_out()?;
    Some((fade_out_abs / selection.width().max(f32::EPSILON)).clamp(0.0, 1.0))
}

fn resize_fade_in_start(
    selection: wavecrate::selection::SelectionRange,
    fade_end: f32,
    start_ratio: f32,
    curve: f32,
) -> wavecrate::selection::SelectionRange {
    let new_start = start_ratio.clamp(0.0, selection.end());
    let mut resized = wavecrate::selection::SelectionRange::new(new_start, selection.end());
    if let Some(fade_out) = selection.fade_out() {
        let fade_out_abs = selection.width() * fade_out.length;
        let length = if resized.width() <= f32::EPSILON {
            0.0
        } else {
            (fade_out_abs / resized.width()).clamp(0.0, 1.0)
        };
        resized = resized
            .with_fade_out(length, fade_out.curve)
            .with_fade_out_mute(fade_out.mute);
    }
    let length = fade_in_length_for_end(resized, fade_end);
    resized.with_fade_in(length, curve)
}

fn resize_fade_out_end(
    selection: wavecrate::selection::SelectionRange,
    fade_start: f32,
    end_ratio: f32,
    curve: f32,
) -> wavecrate::selection::SelectionRange {
    let new_end = end_ratio.clamp(selection.start(), 1.0);
    let mut resized = wavecrate::selection::SelectionRange::new(selection.start(), new_end);
    if let Some(fade_in) = selection.fade_in() {
        let fade_in_abs = selection.width() * fade_in.length;
        let length = if resized.width() <= f32::EPSILON {
            0.0
        } else {
            (fade_in_abs / resized.width()).clamp(0.0, 1.0)
        };
        resized = resized
            .with_fade_in(length, fade_in.curve)
            .with_fade_in_mute(fade_in.mute);
    }
    let length = fade_out_length_for_start(resized, fade_start);
    resized.with_fade_out(length, curve)
}

fn edit_preview_for_selection(
    selection: Option<wavecrate::selection::SelectionRange>,
) -> TimelineEditPreview {
    let Some(selection) = selection else {
        return TimelineEditPreview::default();
    };
    let start = selection.start();
    let end = selection.end();
    let width = selection.width();
    let fade_in = selection.fade_in();
    let fade_out = selection.fade_out();
    TimelineEditPreview::new(
        Some(NormalizedRange::from_micros(
            normalized_to_micros(start),
            normalized_to_micros(end),
        )),
        fade_in.map(|fade| normalized_to_milli(start + width * fade.length)),
        fade_in.map(|fade| normalized_to_micros(start + width * fade.length)),
        fade_in.map(|fade| normalized_to_milli(start - width * fade.mute)),
        fade_in.map(|fade| normalized_to_micros(start - width * fade.mute)),
        fade_in.map(|fade| normalized_to_milli(fade.curve)),
        fade_out.map(|fade| normalized_to_milli(end - width * fade.length)),
        fade_out.map(|fade| normalized_to_micros(end - width * fade.length)),
        fade_out.map(|fade| normalized_to_milli(end + width * fade.mute)),
        fade_out.map(|fade| normalized_to_micros(end + width * fade.mute)),
        fade_out.map(|fade| normalized_to_milli(fade.curve)),
    )
}

fn normalized_to_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn normalized_to_micros(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}

#[derive(Clone, Debug)]
pub(super) struct WaveformFile {
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
    sample_rate: u32,
    channels: usize,
    frames: usize,
    gpu_signal_samples: Arc<[f32]>,
    gpu_signal_summary: Arc<radiant::runtime::GpuSignalSummary>,
}

impl WaveformFile {
    fn path_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.path.hash(&mut hasher);
        self.frames.hash(&mut hasher);
        self.sample_rate.hash(&mut hasher);
        self.channels.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformViewport {
    start: usize,
    end: usize,
}

pub(super) fn default_sample_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/portal_SS_kick_003.wav")
}

pub(super) fn waveform_viewport_view(state: &WaveformState) -> ui::View<super::GuiMessage> {
    ui::stack([
        ui::custom_widget(
            WaveformSignalWidget::new(state.file(), state.viewport(), state.edit_selection()),
            |_| None,
        )
        .id(11)
        .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32),
        ui::custom_widget(
            WaveformWidget::new(
                state.file(),
                state.viewport(),
                state.cursor_ratio(),
                state.playhead_ratio(),
                state.play_mark_ratio(),
                state.edit_mark_ratio(),
                state.play_selection(),
                state.edit_selection(),
                state.active_drag_kind(),
            ),
            |output| {
                output
                    .typed_ref::<WaveformInteraction>()
                    .copied()
                    .map(GuiMessage::Waveform)
            },
        )
        .id(12)
        .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32),
    ])
    .id(10)
    .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)
}

fn load_waveform_file(path: PathBuf) -> Result<WaveformFile, String> {
    let bytes: Arc<[u8]> = fs::read(&path)
        .map_err(|err| format!("failed to read audio file: {err}"))?
        .into();
    if is_wav_path(&path) {
        if let Ok(file) = load_wav_waveform_file(path.clone(), Arc::clone(&bytes)) {
            return Ok(file);
        }
    }
    let decoded =
        wavecrate::waveform::WaveformRenderer::new(WAVEFORM_WIDTH as u32, WAVEFORM_HEIGHT as u32)
            .decode_from_bytes(&bytes)
            .map_err(|err| format!("failed to decode audio file: {err}"))?;
    let channels = decoded.channel_count();
    let frames = decoded.frame_count();
    let mono_samples = if decoded.samples.is_empty() {
        decoded.analysis_samples.iter().copied().collect::<Vec<_>>()
    } else {
        downmix_to_mono(&decoded.samples, channels, frames)
    };
    if mono_samples.is_empty() {
        return Err(String::from("audio file contains no complete frames"));
    }
    Ok(waveform_file_from_mono_samples(
        path,
        bytes,
        decoded.sample_rate,
        channels,
        mono_samples,
    ))
}

#[cfg(test)]
fn synthetic_waveform_file() -> WaveformFile {
    let frames = SYNTHETIC_SAMPLE_RATE as usize * SYNTHETIC_SECONDS;
    let samples = (0..frames)
        .map(|frame| {
            let t = frame as f32 / SYNTHETIC_SAMPLE_RATE as f32;
            let envelope = (1.0 - t / SYNTHETIC_SECONDS as f32).clamp(0.18, 1.0);
            let low = (std::f32::consts::TAU * 72.0 * t).sin() * 0.48;
            let mid = (std::f32::consts::TAU * 220.0 * t).sin() * 0.24;
            let high = (std::f32::consts::TAU * 1_760.0 * t).sin() * 0.1;
            ((low + mid + high) * envelope).clamp(-1.0, 1.0)
        })
        .collect::<Vec<_>>();
    waveform_file_from_mono_samples(
        PathBuf::from("synthetic-waveform"),
        Arc::from([]),
        SYNTHETIC_SAMPLE_RATE,
        1,
        samples,
    )
}

fn waveform_file_from_mono_samples(
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
    sample_rate: u32,
    channels: usize,
    mono_samples: Vec<f32>,
) -> WaveformFile {
    let gpu_signal_samples = split_frequency_bands(&mono_samples, sample_rate);
    let gpu_signal_summary = Arc::new(
        radiant::runtime::GpuSignalSummary::from_interleaved_samples(
            &gpu_signal_samples,
            mono_samples.len(),
            BAND_COUNT,
        ),
    );
    WaveformFile {
        path,
        audio_bytes,
        sample_rate,
        channels,
        frames: mono_samples.len(),
        gpu_signal_samples: Arc::from(gpu_signal_samples),
        gpu_signal_summary,
    }
}

fn apply_edit_fade_to_signal_samples(
    samples: &mut [f32],
    frames: usize,
    band_count: usize,
    selection: sempal::selection::SelectionRange,
) {
    if frames == 0 || band_count == 0 || !selection.has_edit_effects() {
        return;
    }
    let max_frame = frames.saturating_sub(1).max(1);
    for frame in 0..frames {
        let position = frame as f32 / max_frame as f32;
        let gain = selection.gain_at_position(position, 0.0);
        if (gain - 1.0).abs() <= f32::EPSILON {
            continue;
        }
        let base = frame * band_count;
        for band in 0..band_count {
            if let Some(sample) = samples.get_mut(base + band) {
                *sample *= gain;
            }
        }
    }
}

fn edit_selection_revision(selection: Option<sempal::selection::SelectionRange>) -> u64 {
    let Some(selection) = selection else {
        return 0;
    };
    if !selection.has_edit_effects() {
        return 0;
    }
    let mut hasher = DefaultHasher::new();
    selection.start().to_bits().hash(&mut hasher);
    selection.end().to_bits().hash(&mut hasher);
    selection.gain().to_bits().hash(&mut hasher);
    if let Some(fade) = selection.fade_in() {
        fade.length.to_bits().hash(&mut hasher);
        fade.curve.to_bits().hash(&mut hasher);
        fade.mute.to_bits().hash(&mut hasher);
    }
    if let Some(fade) = selection.fade_out() {
        fade.length.to_bits().hash(&mut hasher);
        fade.curve.to_bits().hash(&mut hasher);
        fade.mute.to_bits().hash(&mut hasher);
    }
    hasher.finish().max(1)
}

fn is_wav_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}

fn load_wav_waveform_file(path: PathBuf, bytes: Arc<[u8]>) -> Result<WaveformFile, String> {
    let cursor = Cursor::new(bytes.as_ref());
    let mut reader =
        hound::WavReader::new(cursor).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| {
                sample
                    .map(|value| value.clamp(-1.0, 1.0))
                    .map_err(|err| format!("failed to read float sample: {err}"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            let max =
                ((1_i32 << (u32::from(spec.bits_per_sample).saturating_sub(1))) - 1).max(1) as f32;
            reader
                .samples::<i16>()
                .map(|sample| {
                    sample
                        .map(|value| (f32::from(value) / max).clamp(-1.0, 1.0))
                        .map_err(|err| format!("failed to read integer sample: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        hound::SampleFormat::Int => {
            let max =
                ((1_i64 << (u32::from(spec.bits_per_sample).saturating_sub(1))) - 1).max(1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| ((value as f32) / max).clamp(-1.0, 1.0))
                        .map_err(|err| format!("failed to read integer sample: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    if samples.is_empty() {
        return Err(String::from("WAV contains no samples"));
    }

    let frames = samples.len() / channels;
    let mono_samples = downmix_to_mono(&samples, channels, frames);
    if mono_samples.is_empty() {
        return Err(String::from("WAV contains no complete frames"));
    }
    Ok(waveform_file_from_mono_samples(
        path,
        bytes,
        spec.sample_rate,
        channels,
        mono_samples,
    ))
}

fn split_frequency_bands(samples: &[f32], sample_rate: u32) -> Arc<[f32]> {
    if samples.is_empty() {
        return Arc::from([]);
    }
    let alpha_low = lowpass_alpha(sample_rate, 180.0);
    let alpha_mid = lowpass_alpha(sample_rate, 2_600.0);
    let mut low = 0.0_f32;
    let mut mid_low = 0.0_f32;
    let mut bands = Vec::with_capacity(samples.len().saturating_mul(BAND_COUNT));
    for sample in samples {
        let sample = sample.clamp(-1.0, 1.0);
        low += alpha_low * (sample - low);
        mid_low += alpha_mid * (sample - mid_low);
        let mid = (mid_low - low).clamp(-1.0, 1.0);
        let high = (sample - mid_low).clamp(-1.0, 1.0);
        bands.push(low.clamp(-1.0, 1.0));
        bands.push(mid);
        bands.push(high);
        bands.push(sample);
    }
    bands.into()
}

fn lowpass_alpha(sample_rate: u32, cutoff_hz: f32) -> f32 {
    (1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate.max(1) as f32).exp()).clamp(0.0, 1.0)
}

fn downmix_to_mono(samples: &[f32], channels: usize, frames: usize) -> Vec<f32> {
    let channels = channels.max(1);
    (0..frames)
        .map(|frame| {
            let start = frame * channels;
            let sum = samples[start..start + channels]
                .iter()
                .copied()
                .sum::<f32>();
            (sum / channels as f32).clamp(-1.0, 1.0)
        })
        .collect()
}

impl WaveformViewport {
    fn full(frames: usize) -> Self {
        Self {
            start: 0,
            end: frames.max(1),
        }
    }

    fn visible_frames(self) -> usize {
        self.end.saturating_sub(self.start).max(1)
    }

    fn visible_fraction(self, total_frames: usize) -> f32 {
        self.visible_frames() as f32 / total_frames.max(1) as f32
    }

    fn offset_fraction(self, total_frames: usize) -> f32 {
        let total_frames = total_frames.max(1);
        let free_frames = total_frames.saturating_sub(self.visible_frames());
        if free_frames == 0 {
            0.0
        } else {
            self.start as f32 / free_frames as f32
        }
    }

    fn clamp(self, total_frames: usize) -> Self {
        let total_frames = total_frames.max(1);
        let visible = self
            .visible_frames()
            .clamp(MIN_VISIBLE_FRAMES.min(total_frames), total_frames);
        let start = self.start.min(total_frames.saturating_sub(visible));
        Self {
            start,
            end: start + visible,
        }
    }
}

#[derive(Clone, Debug)]
struct WaveformSignalWidget {
    common: WidgetCommon,
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    edit_selection: Option<sempal::selection::SelectionRange>,
}

impl WaveformSignalWidget {
    fn new(
        file: Arc<WaveformFile>,
        viewport: WaveformViewport,
        edit_selection: Option<sempal::selection::SelectionRange>,
    ) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)),
        );
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            file,
            viewport,
            edit_selection,
        }
    }

    fn signal_summary(&self) -> Arc<radiant::runtime::GpuSignalSummary> {
        let Some(selection) = self.edit_selection else {
            return Arc::clone(&self.file.gpu_signal_summary);
        };
        if !selection.has_edit_effects() {
            return Arc::clone(&self.file.gpu_signal_summary);
        }
        let mut samples = self.file.gpu_signal_samples.as_ref().to_vec();
        apply_edit_fade_to_signal_samples(&mut samples, self.file.frames, BAND_COUNT, selection);
        Arc::new(
            radiant::runtime::GpuSignalSummary::from_interleaved_samples(
                &samples,
                self.file.frames,
                BAND_COUNT,
            ),
        )
    }

    fn signal_revision(&self) -> u64 {
        edit_selection_revision(self.edit_selection)
    }
}

impl Widget for WaveformSignalWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let summary = self.signal_summary();
        primitives.push(PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: self.common.id,
            key: self.file.path_hash(),
            revision: self.signal_revision(),
            rect: bounds,
            content: GpuSurfaceContent::SignalSummaryBands {
                frames: self.file.frames,
                band_count: BAND_COUNT,
                frame_range: [self.viewport.start as f32, self.viewport.end as f32],
                summary,
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: true,
                runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(
                    GpuSurfaceLineStyle {
                        color: Rgba8 {
                            r: 255,
                            g: 255,
                            b: 255,
                            a: 235,
                        },
                        width: 1.0,
                    },
                ),
            },
            overlays: Vec::new(),
        }));
    }
}

#[derive(Clone, Debug)]
struct WaveformWidget {
    common: WidgetCommon,
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    playhead_ratio: Option<f32>,
    play_mark_ratio: Option<f32>,
    edit_mark_ratio: Option<f32>,
    play_selection: Option<wavecrate::selection::SelectionRange>,
    edit_selection: Option<wavecrate::selection::SelectionRange>,
    edit_preview: TimelineEditPreview,
    active_drag_kind: Option<WaveformActiveDragKind>,
}

impl WaveformWidget {
    fn new(
        file: Arc<WaveformFile>,
        viewport: WaveformViewport,
        _cursor_ratio: Option<f32>,
        playhead_ratio: Option<f32>,
        play_mark_ratio: Option<f32>,
        edit_mark_ratio: Option<f32>,
        play_selection: Option<wavecrate::selection::SelectionRange>,
        edit_selection: Option<wavecrate::selection::SelectionRange>,
        active_drag_kind: Option<WaveformActiveDragKind>,
    ) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            file,
            viewport,
            playhead_ratio,
            play_mark_ratio,
            edit_mark_ratio,
            play_selection,
            edit_selection,
            edit_preview: edit_preview_for_selection(edit_selection),
            active_drag_kind,
        }
    }

    fn ratio_from_position(&self, bounds: Rect, position: Point) -> f32 {
        ((position.x - bounds.min.x) / bounds.width().max(1.0)).clamp(0.0, 1.0)
    }
}

impl Widget for WaveformWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                self.active_drag_kind.map(|_| {
                    WidgetOutput::typed(WaveformInteraction::UpdateSelection {
                        visible_ratio: self.ratio_from_position(bounds, position),
                    })
                })
            }
            WidgetInput::Wheel { position, delta } if bounds.contains(position) => {
                Some(WidgetOutput::typed(WaveformInteraction::Wheel {
                    delta,
                    anchor_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                if let Some(handle) = self.edit_fade_handle_at(bounds, position) {
                    return Some(WidgetOutput::typed(WaveformInteraction::BeginEditFade {
                        handle,
                        visible_ratio: self.ratio_from_position(bounds, position),
                    }));
                }
                if let Some(edge) =
                    self.selection_resize_handle_at(bounds, position, WaveformSelectionKind::Play)
                {
                    return Some(WidgetOutput::typed(
                        WaveformInteraction::BeginSelectionResize {
                            kind: WaveformSelectionKind::Play,
                            edge,
                            visible_ratio: self.ratio_from_position(bounds, position),
                        },
                    ));
                }
                Some(WidgetOutput::typed(WaveformInteraction::BeginSelection {
                    kind: WaveformSelectionKind::Play,
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Secondary,
            } if bounds.contains(position) => {
                if let Some(handle) = self.edit_fade_handle_at(bounds, position) {
                    return Some(WidgetOutput::typed(WaveformInteraction::BeginEditFade {
                        handle,
                        visible_ratio: self.ratio_from_position(bounds, position),
                    }));
                }
                Some(WidgetOutput::typed(WaveformInteraction::BeginSelection {
                    kind: WaveformSelectionKind::Edit,
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Auxiliary,
            } if bounds.contains(position) => {
                Some(WidgetOutput::typed(WaveformInteraction::BeginPan {
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } if self.active_drag_kind
                == Some(WaveformActiveDragKind::Selection(
                    WaveformSelectionKind::Play,
                ))
                || matches!(
                    self.active_drag_kind,
                    Some(
                        WaveformActiveDragKind::EditFade(_)
                            | WaveformActiveDragKind::SelectionResize(
                                WaveformSelectionKind::Play,
                                _
                            )
                    )
                ) =>
            {
                Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Secondary,
            } if self.active_drag_kind
                == Some(WaveformActiveDragKind::Selection(
                    WaveformSelectionKind::Edit,
                ))
                || matches!(
                    self.active_drag_kind,
                    Some(WaveformActiveDragKind::EditFade(_))
                ) =>
            {
                Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Auxiliary,
            } if self.active_drag_kind == Some(WaveformActiveDragKind::Pan) => {
                Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            _ => None,
        }
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.append_selection_and_marker_paint(primitives, bounds);
        self.append_edit_fade_paint(primitives, bounds);
    }
}

impl WaveformWidget {
    fn append_selection_and_marker_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        if let Some((start, end)) = self.visible_range_for_selection(self.play_selection) {
            self.push_visible_range_fill(
                primitives,
                bounds,
                start,
                end,
                Rgba8 {
                    r: 255,
                    g: 142,
                    b: 92,
                    a: 48,
                },
            );
            self.append_selection_resize_handles(
                primitives,
                bounds,
                start,
                end,
                Rgba8 {
                    r: 255,
                    g: 142,
                    b: 92,
                    a: 220,
                },
            );
        }
        if let Some((start, end)) = self.visible_range_for_selection(self.edit_selection) {
            self.push_visible_range_fill(
                primitives,
                bounds,
                start,
                end,
                Rgba8 {
                    r: 82,
                    g: 168,
                    b: 255,
                    a: 46,
                },
            );
        }
        if let Some(play_mark_ratio) = self.visible_ratio_for_absolute(self.play_mark_ratio) {
            self.push_visible_cursor(
                primitives,
                bounds,
                play_mark_ratio,
                Rgba8 {
                    r: 255,
                    g: 142,
                    b: 92,
                    a: 230,
                },
                1.25,
            );
        }
        if let Some(edit_mark_ratio) = self.visible_ratio_for_absolute(self.edit_mark_ratio) {
            self.push_visible_cursor(
                primitives,
                bounds,
                edit_mark_ratio,
                Rgba8 {
                    r: 82,
                    g: 168,
                    b: 255,
                    a: 230,
                },
                1.25,
            );
        }
        if let Some(playhead_ratio) = self.visible_ratio_for_absolute(self.playhead_ratio) {
            self.push_visible_cursor(
                primitives,
                bounds,
                playhead_ratio,
                Rgba8 {
                    r: 71,
                    g: 220,
                    b: 255,
                    a: 245,
                },
                1.75,
            );
        }
    }

    fn append_edit_fade_paint(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
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

    fn append_selection_resize_handles(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        start: f32,
        end: f32,
        color: Rgba8,
    ) {
        for edge in [WaveformSelectionEdge::Start, WaveformSelectionEdge::End] {
            if let Some(rect) = self.selection_resize_handle_rect(bounds, start, end, edge) {
                self.push_fill(primitives, rect, color);
            }
        }
    }

    fn selection_resize_handle_at(
        &self,
        bounds: Rect,
        position: Point,
        kind: WaveformSelectionKind,
    ) -> Option<WaveformSelectionEdge> {
        let range = match kind {
            WaveformSelectionKind::Play => self.play_selection,
            WaveformSelectionKind::Edit => self.edit_selection,
        };
        let (start, end) = self.visible_range_for_selection(range)?;
        [WaveformSelectionEdge::Start, WaveformSelectionEdge::End]
            .into_iter()
            .find(|edge| {
                self.selection_resize_handle_rect(bounds, start, end, *edge)
                    .is_some_and(|rect| rect.contains(position))
            })
    }

    fn selection_resize_handle_rect(
        &self,
        bounds: Rect,
        start: f32,
        end: f32,
        edge: WaveformSelectionEdge,
    ) -> Option<Rect> {
        let x_ratio = match edge {
            WaveformSelectionEdge::Start => start,
            WaveformSelectionEdge::End => end,
        };
        let x = bounds.min.x + bounds.width() * x_ratio.clamp(0.0, 1.0);
        let width = 7.0_f32.min(bounds.width().max(1.0));
        let half_width = width * 0.5;
        let top = bounds.min.y;
        let bottom = (bounds.min.y + 22.0)
            .min(bounds.max.y)
            .max(bounds.min.y + 1.0);
        let left = (x - half_width).clamp(bounds.min.x, bounds.max.x - width.max(1.0));
        let right = (left + width).min(bounds.max.x).max(left + 1.0);
        Some(Rect::from_min_max(
            Point::new(left, top),
            Point::new(right, bottom),
        ))
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
        selection: sempal::selection::SelectionRange,
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

    fn edit_fade_handle_at(&self, bounds: Rect, position: Point) -> Option<WaveformEditFadeHandle> {
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

    fn push_fill(&self, primitives: &mut Vec<PaintPrimitive>, rect: Rect, color: Rgba8) {
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect,
            color,
        }));
    }

    fn push_visible_range_fill(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        start: f32,
        end: f32,
        color: Rgba8,
    ) {
        let min_x = bounds.min.x + bounds.width() * start.min(end).clamp(0.0, 1.0);
        let max_x = bounds.min.x + bounds.width() * start.max(end).clamp(0.0, 1.0);
        self.push_fill(
            primitives,
            Rect::from_min_max(
                Point::new(min_x, bounds.min.y),
                Point::new(max_x, bounds.max.y),
            ),
            color,
        );
    }

    fn push_visible_cursor(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        ratio: f32,
        color: Rgba8,
        width: f32,
    ) {
        let x = bounds.min.x + bounds.width() * ratio.clamp(0.0, 1.0);
        let half_width = (width * 0.5).max(0.5);
        self.push_fill(
            primitives,
            Rect::from_min_max(
                Point::new((x - half_width).max(bounds.min.x), bounds.min.y),
                Point::new((x + half_width).min(bounds.max.x), bounds.max.y),
            ),
            color,
        );
    }

    fn visible_range_for_selection(
        &self,
        range: Option<wavecrate::selection::SelectionRange>,
    ) -> Option<(f32, f32)> {
        let range = range?;
        let total = self.file.frames.max(1) as f32;
        let visible_start = self.viewport.start as f32;
        let visible_end = self.viewport.end as f32;
        let visible_width = self.viewport.visible_frames() as f32;
        let start_frame = range.start().clamp(0.0, 1.0) * total;
        let end_frame = range.end().clamp(0.0, 1.0) * total;
        let left_frame = start_frame.min(end_frame).max(visible_start);
        let right_frame = start_frame.max(end_frame).min(visible_end);
        if right_frame <= left_frame {
            return None;
        }
        let start = ((left_frame - visible_start) / visible_width.max(1.0)).clamp(0.0, 1.0);
        let end = ((right_frame - visible_start) / visible_width.max(1.0)).clamp(0.0, 1.0);
        Some((start, end))
    }

    fn visible_ratio_for_absolute(&self, ratio: Option<f32>) -> Option<f32> {
        let absolute_ratio = ratio?;
        let frame = absolute_ratio.clamp(0.0, 1.0) * self.file.frames.max(1) as f32;
        let visible_start = self.viewport.start as f32;
        let visible_width = self.viewport.visible_frames() as f32;
        let visible_ratio = (frame - visible_start) / visible_width.max(1.0);
        if !(0.0..=1.0).contains(&visible_ratio) {
            return None;
        }
        Some(visible_ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BAND_COUNT, WaveformEditFadeHandle, WaveformInteraction, WaveformSelectionEdge,
        WaveformSelectionKind, WaveformSignalWidget, WaveformState, WaveformWidget,
        split_frequency_bands, waveform_file_from_mono_samples,
    };
    use radiant::{
        gui::types::{Point, Rect, Vector2},
        runtime::{GpuSurfaceContent, PaintFillRect, PaintPrimitive},
        theme::ThemeTokens,
        widgets::{PointerButton, Widget, WidgetInput},
    };
    use std::sync::Arc;

    #[test]
    fn waveform_summary_preserves_raw_transient_detail() {
        let samples = vec![0.0, 0.12, -0.9, 0.08, 0.0, 0.42, -0.18, 0.0];

        let file = waveform_file_from_mono_samples(
            "test.wav".into(),
            Arc::from([]),
            48_000,
            1,
            samples.clone(),
        );

        assert_eq!(BAND_COUNT, 4);
        let raw_peak_index = samples
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.abs().total_cmp(&right.abs()))
            .map(|(index, _)| index)
            .expect("peak sample");
        let rendered_peak_index = file.gpu_signal_summary.levels[0]
            .buckets
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                left.max
                    .abs()
                    .max(left.min.abs())
                    .total_cmp(&right.max.abs().max(right.min.abs()))
            })
            .map(|(index, _)| index / BAND_COUNT)
            .expect("peak band sample");

        assert_eq!(rendered_peak_index, raw_peak_index);
        let frame_peak = file.gpu_signal_summary.levels[0].buckets
            [raw_peak_index * BAND_COUNT..(raw_peak_index + 1) * BAND_COUNT]
            .iter()
            .map(|bucket| bucket.min.abs().max(bucket.max.abs()))
            .fold(0.0_f32, f32::max);
        assert!(frame_peak > 0.89);
    }

    #[test]
    fn frequency_bands_keep_low_mid_high_and_raw_lanes_separate() {
        let samples = [0.0, 0.7, -0.7, 0.18, -0.18, 0.02, -0.02, 0.0];
        let bands = split_frequency_bands(&samples, 48_000);

        assert_eq!(bands.len(), samples.len() * BAND_COUNT);
        let low_peak = bands
            .chunks_exact(BAND_COUNT)
            .map(|frame| frame[0].abs())
            .fold(0.0_f32, f32::max);
        let mid_peak = bands
            .chunks_exact(BAND_COUNT)
            .map(|frame| frame[1].abs())
            .fold(0.0_f32, f32::max);
        let high_peak = bands
            .chunks_exact(BAND_COUNT)
            .map(|frame| frame[2].abs())
            .fold(0.0_f32, f32::max);
        let raw_peak = bands
            .chunks_exact(BAND_COUNT)
            .map(|frame| frame[3].abs())
            .fold(0.0_f32, f32::max);

        assert!(low_peak > 0.0);
        assert!(mid_peak > 0.0);
        assert!(high_peak > 0.0);
        assert!(raw_peak >= high_peak);
    }

    #[test]
    fn playback_state_starts_at_head_and_clears_on_stop() {
        let mut state = WaveformState::synthetic_for_tests();

        assert!(!state.is_playing());
        assert_eq!(state.playhead_ratio(), None);
        assert_eq!(state.play_mark_ratio(), None);

        state.start_playback(0.0);
        assert!(state.is_playing());
        assert_eq!(state.playhead_ratio(), Some(0.0));
        assert_eq!(state.play_mark_ratio(), Some(0.0));

        state.set_playhead_ratio(0.375);
        assert_eq!(state.playhead_ratio(), Some(0.375));
        assert_eq!(state.play_mark_ratio(), Some(0.0));

        state.stop_playback();
        assert!(!state.is_playing());
        assert_eq!(state.playhead_ratio(), None);
        assert_eq!(state.play_mark_ratio(), Some(0.0));
    }

    #[test]
    fn overlay_paint_projects_play_edit_and_playhead_markers() {
        let mut state = WaveformState::synthetic_for_tests();
        state.start_playback(0.125);
        state.set_playhead_ratio(0.25);
        state.apply_interaction(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Edit,
            visible_ratio: 0.375,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection {
            visible_ratio: 0.375,
        });

        let widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.active_drag_kind(),
        );
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(400.0, 80.0)),
            &Default::default(),
            &ThemeTokens::default(),
        );

        let fills = fill_rects(&primitives);
        assert!(fills.iter().any(|fill| {
            (fill.rect.center().x / 400.0 - 0.125).abs() < 0.01
                && (fill.color.r, fill.color.g, fill.color.b) == (255, 142, 92)
                && fill.color.a == 230
        }));
        assert!(fills.iter().any(|fill| {
            (fill.rect.center().x / 400.0 - 0.375).abs() < 0.01
                && (fill.color.r, fill.color.g, fill.color.b) == (82, 168, 255)
                && fill.color.a == 230
        }));
        assert!(fills.iter().any(|fill| {
            (fill.rect.center().x / 400.0 - 0.25).abs() < 0.01
                && (fill.color.r, fill.color.g, fill.color.b) == (71, 220, 255)
                && fill.color.a == 245
        }));
    }

    #[test]
    fn visible_ratio_maps_to_absolute_audio_position_inside_viewport() {
        let mut state = WaveformState::synthetic_for_tests();
        state.viewport = super::WaveformViewport {
            start: 12_000,
            end: 36_000,
        };

        let ratio = state.absolute_ratio_from_visible(0.5);

        assert!((ratio - 0.5).abs() < 0.0001);
    }

    #[test]
    fn auxiliary_drag_pans_zoomed_waveform_viewport() {
        let mut state = WaveformState::synthetic_for_tests();
        state.viewport = super::WaveformViewport {
            start: 12_000,
            end: 36_000,
        };
        let mut widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));
        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(100.0, 40.0),
                    button: PointerButton::Auxiliary,
                },
            )
            .expect("middle press should arm waveform pan");
        let interaction = output
            .typed_ref::<WaveformInteraction>()
            .copied()
            .expect("waveform pan interaction");

        assert_eq!(
            interaction,
            WaveformInteraction::BeginPan { visible_ratio: 0.5 }
        );
        state.apply_interaction(interaction);
        state.apply_interaction(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.25,
        });

        assert!(
            state.viewport().start > 12_000,
            "dragging left should pan the viewport later in the sample"
        );
        assert_eq!(state.viewport().visible_frames(), 24_000);
    }

    #[test]
    fn primary_press_emits_playback_ratio_matching_hover_cursor_ratio() {
        let state = WaveformState::synthetic_for_tests();
        let mut widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(60.0, 40.0),
                    button: PointerButton::Primary,
                },
            )
            .expect("playback interaction");
        let interaction = output
            .typed_ref::<WaveformInteraction>()
            .copied()
            .expect("waveform interaction");

        assert_eq!(
            interaction,
            WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: 0.25
            }
        );
    }

    #[test]
    fn secondary_press_emits_edit_selection_begin_ratio() {
        let state = WaveformState::synthetic_for_tests();
        let mut widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(160.0, 40.0),
                    button: PointerButton::Secondary,
                },
            )
            .expect("edit selection interaction");
        let interaction = output
            .typed_ref::<WaveformInteraction>()
            .copied()
            .expect("waveform interaction");

        assert_eq!(
            interaction,
            WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Edit,
                visible_ratio: 0.75
            }
        );
    }

    #[test]
    fn dragging_primary_creates_playmark_selection_without_starting_playback() {
        let mut state = WaveformState::synthetic_for_tests();

        state.apply_interaction(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.2,
        });
        state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.6 });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.6 });

        let selection = state.play_selection().expect("playmark selection");
        assert!(!state.is_playing());
        assert!((selection.start() - 0.2).abs() < 0.001);
        assert!((selection.end() - 0.6).abs() < 0.001);
        assert_eq!(state.play_mark_ratio(), Some(0.2));
    }

    #[test]
    fn playmark_range_edges_are_resizable() {
        let mut state = WaveformState::synthetic_for_tests();
        state.play_selection = Some(sempal::selection::SelectionRange::new(0.2, 0.6));
        state.play_mark_ratio = Some(0.2);

        state.apply_interaction(WaveformInteraction::BeginSelectionResize {
            kind: WaveformSelectionKind::Play,
            edge: WaveformSelectionEdge::End,
            visible_ratio: 0.6,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection {
            visible_ratio: 0.75,
        });

        let selection = state.play_selection().expect("playmark selection");
        assert!((selection.start() - 0.2).abs() < 0.001);
        assert!((selection.end() - 0.75).abs() < 0.001);
        assert_eq!(state.play_mark_ratio(), Some(selection.start()));
        assert!(!state.is_playing());
    }

    #[test]
    fn primary_press_on_playmark_handle_starts_resize_instead_of_new_selection() {
        let mut state = WaveformState::synthetic_for_tests();
        state.play_selection = Some(sempal::selection::SelectionRange::new(0.2, 0.6));
        state.play_mark_ratio = Some(0.2);
        let mut widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(120.0, 8.0),
                    button: PointerButton::Primary,
                },
            )
            .expect("playmark resize interaction");
        let interaction = output
            .typed_ref::<WaveformInteraction>()
            .copied()
            .expect("waveform interaction");

        assert_eq!(
            interaction,
            WaveformInteraction::BeginSelectionResize {
                kind: WaveformSelectionKind::Play,
                edge: WaveformSelectionEdge::End,
                visible_ratio: 0.6
            }
        );
    }

    #[test]
    fn dragging_secondary_creates_edit_selection() {
        let mut state = WaveformState::synthetic_for_tests();

        state.apply_interaction(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Edit,
            visible_ratio: 0.7,
        });
        state.apply_interaction(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.25,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection {
            visible_ratio: 0.25,
        });

        let selection = state.edit_selection().expect("edit selection");
        assert!((selection.start() - 0.25).abs() < 0.001);
        assert!((selection.end() - 0.7).abs() < 0.001);
        assert_eq!(state.edit_mark_ratio(), Some(0.7));
    }

    #[test]
    fn edit_fade_top_handle_drag_sets_fade_in_length() {
        let mut state = WaveformState::synthetic_for_tests();
        state.apply_interaction(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Edit,
            visible_ratio: 0.2,
        });
        state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.6 });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.6 });

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeInEnd,
            visible_ratio: 0.2,
        });
        state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.3 });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.3 });

        let selection = state.edit_selection().expect("edit selection");
        let fade = selection.fade_in().expect("fade-in after handle drag");
        assert!((selection.start() - 0.2).abs() < 0.001);
        assert!((selection.end() - 0.6).abs() < 0.001);
        assert!((fade.length - 0.25).abs() < 0.001);
        assert!((fade.curve - 0.5).abs() < 0.001);
    }

    #[test]
    fn edit_fade_top_handles_push_and_restore_opposite_fade() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            sempal::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_in(0.25, 0.2)
                .with_fade_out(0.25, 0.7),
        );

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeInEnd,
            visible_ratio: 0.3,
        });
        state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.6 });
        let pushed = state.edit_selection().expect("pushed edit selection");
        let pushed_fade_in = pushed.fade_in().expect("fade-in after push");
        assert!(pushed.fade_out().is_none());
        assert!((pushed.start() + pushed.width() * pushed_fade_in.length - 0.6).abs() < 0.001);

        state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.3 });
        let restored = state.edit_selection().expect("restored edit selection");
        let restored_fade_in = restored.fade_in().expect("restored fade-in");
        let restored_fade_out = restored.fade_out().expect("restored fade-out");
        let fade_in_end = restored.start() + restored.width() * restored_fade_in.length;
        let fade_out_start = restored.end() - restored.width() * restored_fade_out.length;
        assert!((fade_in_end - 0.3).abs() < 0.001);
        assert!((fade_out_start - 0.5).abs() < 0.001);
        assert!((restored_fade_in.curve - 0.2).abs() < 0.001);
        assert!((restored_fade_out.curve - 0.7).abs() < 0.001);
    }

    #[test]
    fn edit_fade_outer_handles_set_crossfade_lengths_without_resizing_selection() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection =
            Some(sempal::selection::SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2));

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeInOuterStart,
            visible_ratio: 0.2,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.1 });

        let selection = state.edit_selection().expect("edit selection");
        let fade = selection.fade_in().expect("fade-in after outer drag");
        assert!((selection.start() - 0.2).abs() < 0.001);
        assert!((selection.end() - 0.6).abs() < 0.001);
        assert!((fade.length - 0.25).abs() < 0.001);
        assert!((fade.mute - 0.25).abs() < 0.001);

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeInOuterStart,
            visible_ratio: 0.1,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.2 });

        let selection = state.edit_selection().expect("edit selection");
        let fade = selection.fade_in().expect("fade-in should remain");
        assert!((fade.length - 0.25).abs() < 0.001);
        assert!(fade.mute.abs() < 0.001);

        state.edit_selection =
            Some(sempal::selection::SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.7));
        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeOutOuterEnd,
            visible_ratio: 0.6,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.7 });

        let selection = state.edit_selection().expect("edit selection");
        let fade = selection.fade_out().expect("fade-out after outer drag");
        assert!((selection.start() - 0.2).abs() < 0.001);
        assert!((selection.end() - 0.6).abs() < 0.001);
        assert!((fade.length - 0.25).abs() < 0.001);
        assert!((fade.mute - 0.25).abs() < 0.001);
    }

    #[test]
    fn primary_press_on_outer_fade_handle_uses_distinct_handle() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection =
            Some(sempal::selection::SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2));
        let mut widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(40.0, 40.0),
                    button: PointerButton::Primary,
                },
            )
            .expect("outer fade handle interaction");
        let interaction = output
            .typed_ref::<WaveformInteraction>()
            .copied()
            .expect("waveform interaction");

        assert_eq!(
            interaction,
            WaveformInteraction::BeginEditFade {
                handle: WaveformEditFadeHandle::FadeInOuterStart,
                visible_ratio: 0.2
            }
        );
    }

    #[test]
    fn edit_fade_bottom_handle_resizes_selection_and_keeps_fade_boundary() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection =
            Some(wavecrate::selection::SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2));

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeInStart,
            visible_ratio: 0.2,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.1 });

        let selection = state.edit_selection().expect("edit selection");
        let fade = selection.fade_in().expect("fade-in after resize");
        assert!((selection.start() - 0.1).abs() < 0.001);
        assert!((selection.end() - 0.6).abs() < 0.001);
        assert!((selection.start() + selection.width() * fade.length - 0.3).abs() < 0.001);
        assert!((fade.curve - 0.2).abs() < 0.001);
    }

    #[test]
    fn edit_fade_out_bottom_handle_keeps_opposite_fade_boundary_stable() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            sempal::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_in(0.25, 0.2)
                .with_fade_out(0.25, 0.7),
        );

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeOutEnd,
            visible_ratio: 0.6,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.8 });

        let selection = state.edit_selection().expect("edit selection");
        let fade_in = selection.fade_in().expect("fade-in should remain");
        let fade_out = selection.fade_out().expect("fade-out should remain");
        let fade_in_end = selection.start() + selection.width() * fade_in.length;
        let fade_out_start = selection.end() - selection.width() * fade_out.length;
        assert!((fade_in_end - 0.3).abs() < 0.001);
        assert!((fade_out_start - 0.5).abs() < 0.001);
        assert!((fade_in.curve - 0.2).abs() < 0.001);
        assert!((fade_out.curve - 0.7).abs() < 0.001);
    }

    #[test]
    fn edit_fade_in_bottom_handle_keeps_opposite_fade_boundary_stable() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            sempal::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_in(0.25, 0.2)
                .with_fade_out(0.25, 0.7),
        );

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeInStart,
            visible_ratio: 0.2,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.1 });

        let selection = state.edit_selection().expect("edit selection");
        let fade_in = selection.fade_in().expect("fade-in should remain");
        let fade_out = selection.fade_out().expect("fade-out should remain");
        let fade_in_end = selection.start() + selection.width() * fade_in.length;
        let fade_out_start = selection.end() - selection.width() * fade_out.length;
        assert!((fade_in_end - 0.3).abs() < 0.001);
        assert!((fade_out_start - 0.5).abs() < 0.001);
        assert!((fade_in.curve - 0.2).abs() < 0.001);
        assert!((fade_out.curve - 0.7).abs() < 0.001);
    }

    #[test]
    fn primary_press_on_edit_fade_handle_starts_fade_drag_instead_of_playmark() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
        let mut widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(40.0, 4.0),
                    button: PointerButton::Primary,
                },
            )
            .expect("fade handle interaction");
        let interaction = output
            .typed_ref::<WaveformInteraction>()
            .copied()
            .expect("waveform interaction");

        assert_eq!(
            interaction,
            WaveformInteraction::BeginEditFade {
                handle: WaveformEditFadeHandle::FadeInEnd,
                visible_ratio: 0.2
            }
        );
    }

    #[test]
    fn primary_click_without_drag_still_starts_playback_from_click() {
        let mut state = WaveformState::synthetic_for_tests();

        state.apply_interaction(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.45,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection {
            visible_ratio: 0.45,
        });

        assert!(state.is_playing());
        assert_eq!(state.playhead_ratio(), Some(0.45));
        assert_eq!(state.play_mark_ratio(), Some(0.45));
        assert_eq!(state.play_selection(), None);
    }

    #[test]
    fn selection_range_projects_visible_ratios_inside_viewport() {
        let mut state = WaveformState::synthetic_for_tests();
        state.apply_interaction(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Edit,
            visible_ratio: 0.25,
        });
        state.apply_interaction(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.75,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection {
            visible_ratio: 0.75,
        });
        let widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.active_drag_kind(),
        );
        let (start, end) = widget
            .visible_range_for_selection(state.edit_selection())
            .expect("selection range");

        assert!((start - 0.25).abs() < 0.001);
        assert!((end - 0.75).abs() < 0.001);
    }

    #[test]
    fn selection_fill_paints_as_overlay_widget_rects() {
        let mut state = WaveformState::synthetic_for_tests();
        state.apply_interaction(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.2,
        });
        state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.6 });
        let widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.active_drag_kind(),
        );
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
            &Default::default(),
            &ThemeTokens::default(),
        );

        assert!(
            !primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_))),
            "ordinary waveform overlay widget must not emit the GPU waveform"
        );
        let fills = fill_rects(&primitives);
        assert!(fills.iter().any(|fill| {
            (fill.rect.min.x - 40.0).abs() < 0.001
                && (fill.rect.max.x - 120.0).abs() < 0.001
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 48)
        }));
        assert!(fills.iter().any(|fill| {
            (fill.rect.center().x - 40.0).abs() < 1.0
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 230)
        }));
    }

    #[test]
    fn edit_fade_curve_paints_volume_trace_as_overlay_rects() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            sempal::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_in(0.5, 0.8)
                .with_fade_out(0.25, 0.0),
        );
        let widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.active_drag_kind(),
        );
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
            &Default::default(),
            &ThemeTokens::default(),
        );

        let curve_points = fill_rects(&primitives)
            .into_iter()
            .filter(|fill| {
                (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 225)
            })
            .count();
        assert!(
            curve_points >= 16,
            "expected visible fade curve trace points, got {curve_points}"
        );
    }

    #[test]
    fn signal_widget_paints_gpu_surface_without_app_overlay_handles() {
        let state = WaveformState::synthetic_for_tests();
        let widget =
            WaveformSignalWidget::new(state.file(), state.viewport(), state.edit_selection());
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
            &Default::default(),
            &ThemeTokens::default(),
        );

        let surface = primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::GpuSurface(surface)
                    if matches!(
                        surface.content,
                        GpuSurfaceContent::SignalSummaryBands { .. }
                    ) =>
                {
                    Some(surface)
                }
                _ => None,
            })
            .expect("waveform gpu surface");

        assert!(surface.overlays.is_empty());
    }

    #[test]
    fn signal_widget_applies_active_edit_fade_to_gpu_summary() {
        let file = Arc::new(waveform_file_from_mono_samples(
            "fade-preview.wav".into(),
            Arc::from([]),
            48_000,
            1,
            vec![1.0; 16],
        ));
        let viewport = super::WaveformViewport::full(file.frames);
        let edit_selection =
            Some(sempal::selection::SelectionRange::new(0.0, 1.0).with_fade_in(1.0, 0.0));
        let widget = WaveformSignalWidget::new(file, viewport, edit_selection);
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
            &Default::default(),
            &ThemeTokens::default(),
        );

        let surface = primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::GpuSurface(surface) => Some(surface),
                _ => None,
            })
            .expect("waveform gpu surface");

        assert!(surface.revision > 0);
        let GpuSurfaceContent::SignalSummaryBands { summary, .. } = &surface.content else {
            panic!("expected signal summary bands");
        };
        let buckets = &summary.levels[0].buckets;
        let first_raw = &buckets[3];
        let last_raw = &buckets[(15 * BAND_COUNT) + 3];
        assert!(first_raw.max.abs().max(first_raw.min.abs()) < 0.001);
        assert!(last_raw.max.abs().max(last_raw.min.abs()) > 0.9);
    }

    fn fill_rects(primitives: &[PaintPrimitive]) -> Vec<&PaintFillRect> {
        primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill),
                _ => None,
            })
            .collect()
    }
}
