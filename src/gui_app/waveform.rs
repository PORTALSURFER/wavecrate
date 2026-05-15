#![allow(missing_docs)]

use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    gui::{
        range::NormalizedRange,
        visualization::{TimelineEditPreview, TimelineEditPreviewParts},
    },
    layout::LayoutOutput,
    prelude as ui,
    runtime::{
        GpuSignalGainPreview, GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle,
        GpuSurfaceRuntimeOverlays, PaintFillRect, PaintGpuSurface, PaintPrimitive,
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
const SELECTION_MOVE_HANDLE_HEIGHT: f32 = 7.0;
const SELECTION_MOVE_HANDLE_END_INSET: f32 = 9.0;
const SELECTION_FLASH_FRAMES: u8 = 12;
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
    extraction_history: Vec<wavecrate::selection::SelectionRange>,
    play_selection_flash_frames: u8,
    active_drag: Option<WaveformDrag>,
    pending_playback_start: Option<f32>,
}

impl WaveformState {
    pub(super) fn load_default() -> Result<Self, String> {
        Ok(Self::empty())
    }

    pub(super) fn load_path(path: PathBuf) -> Result<Self, String> {
        let file = Arc::new(load_waveform_file(path)?);
        Ok(Self::from_file(file))
    }

    pub(super) fn empty() -> Self {
        Self::from_file(Arc::new(empty_waveform_file()))
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
            extraction_history: Vec::new(),
            play_selection_flash_frames: 0,
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

    pub(super) fn extraction_history(&self) -> &[wavecrate::selection::SelectionRange] {
        &self.extraction_history
    }

    pub(super) fn has_extraction_history(&self) -> bool {
        !self.extraction_history.is_empty()
    }

    pub(super) fn record_current_play_selection_extracted(&mut self) {
        let Some(selection) = self
            .play_selection
            .filter(|selection| selection.width() > 0.0)
        else {
            return;
        };
        self.insert_extraction_history_range(selection);
    }

    pub(super) fn clear_extraction_history(&mut self) {
        self.extraction_history.clear();
    }

    fn insert_extraction_history_range(&mut self, selection: wavecrate::selection::SelectionRange) {
        let mut merged_start = selection.start_f64();
        let mut merged_end = selection.end_f64();
        self.extraction_history.retain(|existing| {
            let overlaps = existing.start_f64() <= merged_end && existing.end_f64() >= merged_start;
            if overlaps {
                merged_start = merged_start.min(existing.start_f64());
                merged_end = merged_end.max(existing.end_f64());
            }
            !overlaps
        });
        self.extraction_history
            .push(wavecrate::selection::SelectionRange::new_precise(
                merged_start,
                merged_end,
            ));
        self.extraction_history
            .sort_by(|a, b| a.start_f64().total_cmp(&b.start_f64()));
    }

    pub(super) fn play_selection_flash_frames(&self) -> u8 {
        self.play_selection_flash_frames
    }

    pub(super) fn play_selection_flash_active(&self) -> bool {
        self.play_selection_flash_frames > 0
    }

    pub(super) fn flash_play_selection(&mut self) {
        self.play_selection_flash_frames = SELECTION_FLASH_FRAMES;
    }

    pub(super) fn extract_play_selection_to_sibling(&self) -> Result<PathBuf, String> {
        let selection = self
            .play_selection
            .filter(|selection| selection.width() > 0.0)
            .ok_or_else(|| String::from("Mark a play range before extracting"))?;
        if !self.has_loaded_sample() {
            return Err(String::from("Load a sample before extracting"));
        }
        if !is_wav_path(&self.file.path) {
            return Err(String::from("Extraction currently supports WAV files"));
        }
        extract_wav_range_to_sibling(
            &self.file.path,
            &self.file.audio_bytes,
            self.file.frames,
            selection,
        )
    }

    pub(super) fn active_drag_kind(&self) -> Option<WaveformActiveDragKind> {
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
        if self.file.path.as_os_str().is_empty() {
            return String::from("No sample loaded");
        }
        self.file
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| self.file.path.display().to_string())
    }

    pub(super) fn path(&self) -> PathBuf {
        self.file.path.clone()
    }

    pub(super) fn rewrite_path_prefix(
        &mut self,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
    ) -> bool {
        if self.file.path == old_path {
            Arc::make_mut(&mut self.file).path = new_path.to_path_buf();
            return true;
        }
        if let Ok(relative) = self.file.path.strip_prefix(old_path) {
            Arc::make_mut(&mut self.file).path = new_path.join(relative);
            return true;
        }
        false
    }

    pub(super) fn has_loaded_sample(&self) -> bool {
        !self.file.audio_bytes.is_empty() && !self.file.path.as_os_str().is_empty()
    }

    pub(super) fn audio_bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.file.audio_bytes)
    }

    pub(super) fn visible_fraction(&self) -> f32 {
        self.viewport.visible_fraction(self.file.frames)
    }

    pub(super) fn fully_zoomed_out(&self) -> bool {
        self.viewport
            .clamp(self.file.frames, MIN_VISIBLE_FRAMES)
            .visible_items()
            >= self.file.frames.max(1)
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
                        self.play_selection_flash_frames = 0;
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
            WaveformInteraction::ClearEditFadeSilence { handle } => {
                self.clear_edit_fade_silence(handle);
                self.active_drag = None;
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
            WaveformInteraction::BeginSelectionMove {
                kind,
                visible_ratio,
            } => {
                let Some(selection) = self.selection_for_kind(kind) else {
                    return;
                };
                let ratio = self.absolute_ratio_from_visible(visible_ratio);
                self.active_drag = Some(WaveformDrag::SelectionMove(
                    WaveformSelectionMoveDrag::new(kind, ratio, selection),
                ));
                self.update_active_selection_move(ratio);
            }
            WaveformInteraction::BeginPan { visible_ratio } => {
                self.active_drag = Some(WaveformDrag::Pan(WaveformPanDrag::new(
                    visible_ratio,
                    self.viewport
                        .clamp(self.file.frames.max(1), MIN_VISIBLE_FRAMES),
                )));
            }
            WaveformInteraction::UpdateSelection { visible_ratio } => {
                self.update_active_drag(visible_ratio);
            }
            WaveformInteraction::FinishSelection { visible_ratio } => {
                self.finish_active_drag(visible_ratio);
            }
            WaveformInteraction::Frame => {
                self.play_selection_flash_frames =
                    self.play_selection_flash_frames.saturating_sub(1);
            }
        }
    }

    pub(super) fn absolute_ratio_from_visible(&self, visible_ratio: f32) -> f32 {
        self.viewport.absolute_ratio_from_visible(
            self.file.frames.max(1),
            MIN_VISIBLE_FRAMES,
            visible_ratio,
        )
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
        self.viewport =
            self.viewport
                .zoom_around_anchor(total, MIN_VISIBLE_FRAMES, factor, anchor_ratio);
    }

    fn pan_by_visible_fraction(&mut self, fraction: f32) {
        let total = self.file.frames.max(1);
        self.viewport = self
            .viewport
            .pan_by_visible_fraction(total, MIN_VISIBLE_FRAMES, fraction);
    }

    fn set_offset_fraction(&mut self, offset_fraction: f32) {
        let total = self.file.frames.max(1);
        self.viewport =
            self.viewport
                .with_offset_fraction(total, MIN_VISIBLE_FRAMES, offset_fraction);
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
            WaveformDrag::SelectionMove(_) => {
                self.update_active_selection_move(ratio);
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
            WaveformDrag::SelectionMove(_) => {
                self.active_drag = Some(drag);
                self.update_active_selection_move(ratio);
                self.active_drag = None;
            }
            WaveformDrag::Pan(drag) => {
                self.update_active_pan(drag, visible_ratio);
            }
        }
    }

    fn set_selection_for_drag(&mut self, drag: WaveformSelectionDrag) {
        let range =
            wavecrate::selection::SelectionRange::new(drag.anchor_ratio, drag.current_ratio);
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

    fn clear_edit_fade_silence(&mut self, handle: WaveformEditFadeHandle) {
        let Some(selection) = self.edit_selection else {
            return;
        };
        let next = match handle {
            WaveformEditFadeHandle::FadeInOuterStart => selection
                .fade_in()
                .map(|fade| selection.with_fade_in_and_mute(fade.length, fade.curve, 0.0)),
            WaveformEditFadeHandle::FadeOutOuterEnd => selection
                .fade_out()
                .map(|fade| selection.with_fade_out_and_mute(fade.length, fade.curve, 0.0)),
            _ => None,
        };
        if let Some(next) = next {
            self.edit_selection = Some(next);
        }
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

    fn update_active_selection_move(&mut self, ratio: f32) {
        let Some(WaveformDrag::SelectionMove(drag)) = self.active_drag else {
            return;
        };
        let selection = drag.apply(ratio);
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
        let viewport = drag.viewport.clamp(total, MIN_VISIBLE_FRAMES);
        let visible = viewport.visible_items();
        if visible >= total {
            return;
        }
        let delta = ((visible_ratio - drag.anchor_visible_ratio) * visible as f32).round() as isize;
        let start = viewport.start.saturating_add_signed(-delta);
        self.viewport = WaveformViewport {
            start,
            end: start + visible,
        }
        .clamp(total, MIN_VISIBLE_FRAMES);
    }

    fn selection_for_kind(
        &self,
        kind: WaveformSelectionKind,
    ) -> Option<wavecrate::selection::SelectionRange> {
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
    ClearEditFadeSilence {
        handle: WaveformEditFadeHandle,
    },
    BeginSelectionResize {
        kind: WaveformSelectionKind,
        edge: WaveformSelectionEdge,
        visible_ratio: f32,
    },
    BeginSelectionMove {
        kind: WaveformSelectionKind,
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
    SelectionMove(WaveformSelectionKind),
    EditFade(WaveformEditFadeHandle),
    Pan,
}

#[derive(Clone, Copy, Debug)]
enum WaveformDrag {
    Selection(WaveformSelectionDrag),
    SelectionResize(WaveformSelectionResizeDrag),
    SelectionMove(WaveformSelectionMoveDrag),
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
            WaveformDrag::SelectionMove(drag) => WaveformActiveDragKind::SelectionMove(drag.kind),
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
struct WaveformSelectionMoveDrag {
    kind: WaveformSelectionKind,
    anchor_ratio: f32,
    baseline: wavecrate::selection::SelectionRange,
}

impl WaveformSelectionMoveDrag {
    fn new(
        kind: WaveformSelectionKind,
        anchor_ratio: f32,
        baseline: wavecrate::selection::SelectionRange,
    ) -> Self {
        Self {
            kind,
            anchor_ratio,
            baseline,
        }
    }

    fn apply(self, ratio: f32) -> wavecrate::selection::SelectionRange {
        self.baseline.shift(ratio - self.anchor_ratio)
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
        selection: wavecrate::selection::SelectionRange,
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
        _selection: wavecrate::selection::SelectionRange,
        ratio: f32,
    ) -> wavecrate::selection::SelectionRange {
        let ratio = ratio.clamp(0.0, 1.0);
        match self.edge {
            WaveformSelectionEdge::Start => {
                wavecrate::selection::SelectionRange::new(ratio, self.fixed_ratio)
            }
            WaveformSelectionEdge::End => {
                wavecrate::selection::SelectionRange::new(self.fixed_ratio, ratio)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct WaveformEditFadeDrag {
    handle: WaveformEditFadeHandle,
    fixed_ratio: f32,
    curve: f32,
    baseline: wavecrate::selection::SelectionRange,
}

impl WaveformEditFadeDrag {
    fn new(
        handle: WaveformEditFadeHandle,
        selection: wavecrate::selection::SelectionRange,
    ) -> Self {
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
                resize_fade_in_start(self.baseline, self.fixed_ratio, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeOutEnd => {
                resize_fade_out_end(self.baseline, self.fixed_ratio, ratio, self.curve)
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
    selection: wavecrate::selection::SelectionRange,
    baseline: wavecrate::selection::SelectionRange,
    end_ratio: f32,
    curve: f32,
) -> wavecrate::selection::SelectionRange {
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
    selection: wavecrate::selection::SelectionRange,
    baseline: wavecrate::selection::SelectionRange,
    start_ratio: f32,
    curve: f32,
) -> wavecrate::selection::SelectionRange {
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
    selection: wavecrate::selection::SelectionRange,
    outer_start_ratio: f32,
) -> wavecrate::selection::SelectionRange {
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
    selection: wavecrate::selection::SelectionRange,
    outer_end_ratio: f32,
) -> wavecrate::selection::SelectionRange {
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
    selection: wavecrate::selection::SelectionRange,
    fade_in: Option<(f32, f32)>,
    fade_out: Option<(f32, f32)>,
) -> wavecrate::selection::SelectionRange {
    let mut rebuilt = wavecrate::selection::SelectionRange::new(selection.start(), selection.end())
        .with_gain(selection.gain());
    if let Some((length, curve)) = fade_in {
        let mute = selection.fade_in().map(|fade| fade.mute).unwrap_or(0.0);
        rebuilt = rebuilt.with_fade_in_and_mute(length.clamp(0.0, 1.0), curve, mute);
    }
    if let Some((length, curve)) = fade_out {
        let mute = selection.fade_out().map(|fade| fade.mute).unwrap_or(0.0);
        rebuilt = rebuilt.with_fade_out_and_mute(length.clamp(0.0, 1.0), curve, mute);
    }
    rebuilt
}

fn fade_in_for_same_width(
    selection: wavecrate::selection::SelectionRange,
    baseline: wavecrate::selection::SelectionRange,
    fade_in_abs: f32,
) -> Option<f32> {
    baseline.fade_in()?;
    Some((fade_in_abs / selection.width().max(f32::EPSILON)).clamp(0.0, 1.0))
}

fn fade_out_for_same_width(
    selection: wavecrate::selection::SelectionRange,
    baseline: wavecrate::selection::SelectionRange,
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
    let old_width = selection.width();
    let mut resized = wavecrate::selection::SelectionRange::new(new_start, selection.end());
    if let Some(fade_out) = selection.fade_out() {
        let fade_out_abs = old_width * fade_out.length;
        let length = if resized.width() <= f32::EPSILON {
            0.0
        } else {
            (fade_out_abs / resized.width()).clamp(0.0, 1.0)
        };
        let old_outer_end = selection.end() + old_width * fade_out.mute;
        let mute = if fade_out.mute <= 0.0 || resized.width() <= f32::EPSILON {
            0.0
        } else {
            fade_out_mute_for_outer_end(resized, old_outer_end)
        };
        resized = resized.with_fade_out_and_mute(length, fade_out.curve, mute);
    }
    let length = fade_in_length_for_end(resized, fade_end);
    let mut resized = resized.with_fade_in(length, curve);
    if let Some(fade_in) = selection.fade_in() {
        let old_outer_start = selection.start() - old_width * fade_in.mute;
        let mute = if fade_in.mute <= 0.0 || resized.width() <= f32::EPSILON {
            0.0
        } else {
            fade_in_mute_for_outer_start(resized, old_outer_start)
        };
        resized = resized.with_fade_in_and_mute(length, curve, mute);
    }
    resized
}

fn resize_fade_out_end(
    selection: wavecrate::selection::SelectionRange,
    fade_start: f32,
    end_ratio: f32,
    curve: f32,
) -> wavecrate::selection::SelectionRange {
    let new_end = end_ratio.clamp(selection.start(), 1.0);
    let old_width = selection.width();
    let mut resized = wavecrate::selection::SelectionRange::new(selection.start(), new_end);
    if let Some(fade_in) = selection.fade_in() {
        let fade_in_abs = old_width * fade_in.length;
        let length = if resized.width() <= f32::EPSILON {
            0.0
        } else {
            (fade_in_abs / resized.width()).clamp(0.0, 1.0)
        };
        let old_outer_start = selection.start() - old_width * fade_in.mute;
        let mute = if fade_in.mute <= 0.0 || resized.width() <= f32::EPSILON {
            0.0
        } else {
            fade_in_mute_for_outer_start(resized, old_outer_start)
        };
        resized = resized.with_fade_in_and_mute(length, fade_in.curve, mute);
    }
    let length = fade_out_length_for_start(resized, fade_start);
    let mut resized = resized.with_fade_out(length, curve);
    if let Some(fade_out) = selection.fade_out() {
        let old_outer_end = selection.end() + old_width * fade_out.mute;
        let mute = if fade_out.mute <= 0.0 || resized.width() <= f32::EPSILON {
            0.0
        } else {
            fade_out_mute_for_outer_end(resized, old_outer_end)
        };
        resized = resized.with_fade_out_and_mute(length, curve, mute);
    }
    resized
}

fn fade_in_mute_for_outer_start(
    selection: wavecrate::selection::SelectionRange,
    outer_start: f32,
) -> f32 {
    if selection.width() <= f32::EPSILON {
        return 0.0;
    }
    let outer_start = snap_to_sample_edge(outer_start).clamp(0.0, selection.start());
    ((selection.start() - outer_start) / selection.width()).max(0.0)
}

fn fade_out_mute_for_outer_end(
    selection: wavecrate::selection::SelectionRange,
    outer_end: f32,
) -> f32 {
    if selection.width() <= f32::EPSILON {
        return 0.0;
    }
    let outer_end = snap_to_sample_edge(outer_end).clamp(selection.end(), 1.0);
    ((outer_end - selection.end()) / selection.width()).max(0.0)
}

fn snap_to_sample_edge(position: f32) -> f32 {
    const EDGE_EPSILON: f32 = 1.0e-6;
    if position <= EDGE_EPSILON {
        0.0
    } else if position >= 1.0 - EDGE_EPSILON {
        1.0
    } else {
        position
    }
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
    TimelineEditPreview::from_parts(TimelineEditPreviewParts {
        selection: Some(NormalizedRange::from_micros(
            normalized_to_micros(start),
            normalized_to_micros(end),
        )),
        leading_end_milli: fade_in.map(|fade| normalized_to_milli(start + width * fade.length)),
        leading_end_micros: fade_in.map(|fade| normalized_to_micros(start + width * fade.length)),
        leading_inner_start_milli: fade_in
            .map(|fade| normalized_to_milli(start - width * fade.mute)),
        leading_inner_start_micros: fade_in
            .map(|fade| normalized_to_micros(start - width * fade.mute)),
        leading_curve_milli: fade_in.map(|fade| normalized_to_milli(fade.curve)),
        trailing_start_milli: fade_out.map(|fade| normalized_to_milli(end - width * fade.length)),
        trailing_start_micros: fade_out.map(|fade| normalized_to_micros(end - width * fade.length)),
        trailing_inner_end_milli: fade_out.map(|fade| normalized_to_milli(end + width * fade.mute)),
        trailing_inner_end_micros: fade_out
            .map(|fade| normalized_to_micros(end + width * fade.mute)),
        trailing_curve_milli: fade_out.map(|fade| normalized_to_milli(fade.curve)),
    })
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
    content_revision: u64,
    sample_rate: u32,
    channels: usize,
    frames: usize,
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

    fn content_revision(&self) -> u64 {
        self.content_revision
    }
}

pub(super) type WaveformViewport = ui::IndexViewport;

pub(super) fn waveform_viewport_view(state: &WaveformState) -> ui::View<super::GuiMessage> {
    ui::stack([
        ui::custom_widget(
            WaveformSignalWidget::new(
                state.file(),
                state.viewport(),
                state.edit_selection(),
                state.active_drag_kind(),
            ),
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
                state.extraction_history(),
                state.play_selection_flash_frames(),
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

fn empty_waveform_file() -> WaveformFile {
    waveform_file_from_mono_samples(PathBuf::new(), Arc::from([]), 0, 1, vec![0.0])
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
        content_revision: content_revision_for_audio_bytes(&audio_bytes),
        audio_bytes,
        sample_rate,
        channels,
        frames: mono_samples.len(),
        gpu_signal_summary,
    }
}

fn gain_preview_for_selection(
    selection: Option<wavecrate::selection::SelectionRange>,
) -> Option<GpuSignalGainPreview> {
    let selection = selection.filter(|selection| selection.has_edit_effects())?;
    let fade_in = selection.fade_in();
    let fade_out = selection.fade_out();
    Some(GpuSignalGainPreview {
        start: selection.start(),
        end: selection.end(),
        gain: selection.gain(),
        fade_in_length: fade_in.map(|fade| fade.length).unwrap_or(0.0),
        fade_in_curve: fade_in.map(|fade| fade.curve).unwrap_or(0.5),
        fade_in_mute: fade_in.map(|fade| fade.mute).unwrap_or(0.0),
        fade_out_length: fade_out.map(|fade| fade.length).unwrap_or(0.0),
        fade_out_curve: fade_out.map(|fade| fade.curve).unwrap_or(0.5),
        fade_out_mute: fade_out.map(|fade| fade.mute).unwrap_or(0.0),
    })
}

fn is_wav_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}

fn extract_wav_range_to_sibling(
    source_path: &std::path::Path,
    bytes: &[u8],
    loaded_frames: usize,
    selection: wavecrate::selection::SelectionRange,
) -> Result<PathBuf, String> {
    let cursor = Cursor::new(bytes);
    let reader =
        hound::WavReader::new(cursor).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let total_frames = (reader.duration() as usize).min(loaded_frames);
    if total_frames == 0 {
        return Err(String::from("WAV contains no complete frames"));
    }
    let frame_range = selection.frame_bounds(total_frames);
    let output_path = next_extraction_path(source_path)?;
    write_wav_frame_range(
        reader,
        spec,
        channels,
        frame_range.start_frame,
        frame_range.end_frame,
        &output_path,
    )?;
    Ok(output_path)
}

fn content_revision_for_audio_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish().max(1)
}

fn next_extraction_path(source_path: &std::path::Path) -> Result<PathBuf, String> {
    let parent = source_path
        .parent()
        .ok_or_else(|| String::from("Source sample has no parent folder"))?;
    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| String::from("Source sample has no file name"))?;
    for index in 0..10_000 {
        let suffix = if index == 0 {
            String::from("_extraction")
        } else {
            format!("_extraction_{index}")
        };
        let candidate = parent.join(format!("{stem}{suffix}.wav"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(String::from(
        "Could not find an available extraction file name",
    ))
}

fn write_wav_frame_range<R: std::io::Read>(
    mut reader: hound::WavReader<R>,
    spec: hound::WavSpec,
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    output_path: &std::path::Path,
) -> Result<(), String> {
    let start_sample = start_frame.saturating_mul(channels);
    let sample_count = end_frame
        .saturating_sub(start_frame)
        .saturating_mul(channels);
    let mut writer = hound::WavWriter::create(output_path, spec)
        .map_err(|err| format!("failed to create extraction: {err}"))?;
    match spec.sample_format {
        hound::SampleFormat::Float => {
            for sample in reader
                .samples::<f32>()
                .skip(start_sample)
                .take(sample_count)
            {
                writer
                    .write_sample(sample.map_err(|err| format!("failed to read sample: {err}"))?)
                    .map_err(|err| format!("failed to write extraction: {err}"))?;
            }
        }
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            for sample in reader
                .samples::<i16>()
                .skip(start_sample)
                .take(sample_count)
            {
                writer
                    .write_sample(sample.map_err(|err| format!("failed to read sample: {err}"))?)
                    .map_err(|err| format!("failed to write extraction: {err}"))?;
            }
        }
        hound::SampleFormat::Int => {
            for sample in reader
                .samples::<i32>()
                .skip(start_sample)
                .take(sample_count)
            {
                writer
                    .write_sample(sample.map_err(|err| format!("failed to read sample: {err}"))?)
                    .map_err(|err| format!("failed to write extraction: {err}"))?;
            }
        }
    }
    writer
        .finalize()
        .map_err(|err| format!("failed to finalize extraction: {err}"))?;
    Ok(())
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
    let alpha_low = lowpass_alpha(sample_rate, 150.0);
    let alpha_mid = lowpass_alpha(sample_rate, 2_200.0);
    let mut low = 0.0_f32;
    let mut mid_low = 0.0_f32;
    let mut low_envelope = 0.0_f32;
    let mut mid_envelope = 0.0_f32;
    let mut high_envelope = 0.0_f32;
    let low_release = envelope_release_alpha(sample_rate, 12.0);
    let mid_release = envelope_release_alpha(sample_rate, 5.5);
    let high_release = envelope_release_alpha(sample_rate, 2.2);
    let mut bands = Vec::with_capacity(samples.len().saturating_mul(BAND_COUNT));
    for sample in samples {
        let sample = sample.clamp(-1.0, 1.0);
        low += alpha_low * (sample - low);
        mid_low += alpha_mid * (sample - mid_low);
        let low_band = (low * 1.08).clamp(-1.0, 1.0);
        let mid_band = ((mid_low - low) * 1.45).clamp(-1.0, 1.0);
        let high_band = ((sample - mid_low) * 2.15).clamp(-1.0, 1.0);
        low_envelope = follow_visual_envelope(low_envelope, low_band.abs(), low_release);
        mid_envelope = follow_visual_envelope(mid_envelope, mid_band.abs(), mid_release);
        high_envelope = follow_visual_envelope(high_envelope, high_band.abs(), high_release);
        bands.push(low_envelope);
        bands.push(mid_envelope);
        bands.push(high_envelope);
        bands.push(sample);
    }
    normalize_visual_band_peaks(&mut bands);
    bands.into()
}

fn normalize_visual_band_peaks(bands: &mut [f32]) {
    let raw_peak = bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[3].abs())
        .fold(0.0_f32, f32::max);
    if raw_peak <= 0.000_01 || !raw_peak.is_finite() {
        return;
    }
    let peaks = [
        visual_band_peak(bands, 0),
        visual_band_peak(bands, 1),
        visual_band_peak(bands, 2),
    ];
    let spectral_total = peaks.iter().copied().sum::<f32>().max(0.000_01);
    let targets = [raw_peak * 0.98, raw_peak * 0.74, raw_peak * 0.34];
    let boost_thresholds = [raw_peak * 0.08, raw_peak * 0.05, raw_peak * 0.035];
    let max_gains = [2.6_f32, 2.8, 2.4];
    for band in 0..3 {
        let peak = peaks[band];
        if peak <= 0.000_01 || !peak.is_finite() {
            continue;
        }
        let energy_share = peak / spectral_total;
        let target = targets[band] * smoothstep_scalar(0.12, 0.55, energy_share);
        let max_gain = if peak < boost_thresholds[band] {
            1.0
        } else {
            max_gains[band]
        };
        let gain = (target / peak).clamp(0.25, max_gain);
        for frame in bands.chunks_exact_mut(BAND_COUNT) {
            frame[band] = (frame[band] * gain).clamp(-1.0, 1.0);
        }
    }
}

fn visual_band_peak(bands: &[f32], band: usize) -> f32 {
    bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[band].abs())
        .fold(0.0_f32, f32::max)
}

fn smoothstep_scalar(edge0: f32, edge1: f32, value: f32) -> f32 {
    let range = (edge1 - edge0).abs().max(0.000_01);
    let t = ((value - edge0) / range).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn follow_visual_envelope(previous: f32, value: f32, release_alpha: f32) -> f32 {
    if value >= previous {
        value
    } else {
        previous + release_alpha * (value - previous)
    }
}

fn envelope_release_alpha(sample_rate: u32, release_ms: f32) -> f32 {
    let samples = sample_rate.max(1) as f32 * (release_ms.max(0.1) / 1_000.0);
    (1.0 - (-1.0 / samples).exp()).clamp(0.0, 1.0)
}

fn lowpass_alpha(sample_rate: u32, cutoff_hz: f32) -> f32 {
    (1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate.max(1) as f32).exp()).clamp(0.0, 1.0)
}

fn downmix_to_mono(samples: &[f32], channels: usize, frames: usize) -> Vec<f32> {
    let channels = channels.max(1);
    (0..frames)
        .map(|frame| {
            let start = frame * channels;
            let mut peak = 0.0_f32;
            for sample in samples[start..start + channels].iter().copied() {
                if sample.abs() > peak.abs() {
                    peak = sample;
                }
            }
            peak.clamp(-1.0, 1.0)
        })
        .collect()
}

#[derive(Clone, Debug)]
struct WaveformSignalWidget {
    common: WidgetCommon,
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    edit_selection: Option<wavecrate::selection::SelectionRange>,
}

impl WaveformSignalWidget {
    fn new(
        file: Arc<WaveformFile>,
        viewport: WaveformViewport,
        edit_selection: Option<wavecrate::selection::SelectionRange>,
        _active_drag_kind: Option<WaveformActiveDragKind>,
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
        Arc::clone(&self.file.gpu_signal_summary)
    }

    fn signal_revision(&self) -> u64 {
        self.file.content_revision()
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
                gain_preview: gain_preview_for_selection(self.edit_selection),
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
    extraction_history: Vec<wavecrate::selection::SelectionRange>,
    play_selection_flash_frames: u8,
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
        extraction_history: &[wavecrate::selection::SelectionRange],
        play_selection_flash_frames: u8,
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
            extraction_history: extraction_history.to_vec(),
            play_selection_flash_frames,
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
                ..
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
                if self.selection_move_handle_at(bounds, position, WaveformSelectionKind::Play) {
                    return Some(WidgetOutput::typed(
                        WaveformInteraction::BeginSelectionMove {
                            kind: WaveformSelectionKind::Play,
                            visible_ratio: self.ratio_from_position(bounds, position),
                        },
                    ));
                }
                if self.selection_move_handle_at(bounds, position, WaveformSelectionKind::Edit) {
                    return Some(WidgetOutput::typed(
                        WaveformInteraction::BeginSelectionMove {
                            kind: WaveformSelectionKind::Edit,
                            visible_ratio: self.ratio_from_position(bounds, position),
                        },
                    ));
                }
                Some(WidgetOutput::typed(WaveformInteraction::BeginSelection {
                    kind: WaveformSelectionKind::Play,
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                if let Some(
                    handle @ (WaveformEditFadeHandle::FadeInOuterStart
                    | WaveformEditFadeHandle::FadeOutOuterEnd),
                ) = self.edit_fade_handle_at(bounds, position)
                {
                    return Some(WidgetOutput::typed(
                        WaveformInteraction::ClearEditFadeSilence { handle },
                    ));
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Secondary,
                ..
            } if bounds.contains(position) => {
                if let Some(handle) = self.edit_fade_handle_at(bounds, position) {
                    return Some(WidgetOutput::typed(WaveformInteraction::BeginEditFade {
                        handle,
                        visible_ratio: self.ratio_from_position(bounds, position),
                    }));
                }
                if self.selection_move_handle_at(bounds, position, WaveformSelectionKind::Edit) {
                    return Some(WidgetOutput::typed(
                        WaveformInteraction::BeginSelectionMove {
                            kind: WaveformSelectionKind::Edit,
                            visible_ratio: self.ratio_from_position(bounds, position),
                        },
                    ));
                }
                Some(WidgetOutput::typed(WaveformInteraction::BeginSelection {
                    kind: WaveformSelectionKind::Edit,
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Auxiliary,
                ..
            } if bounds.contains(position) => {
                Some(WidgetOutput::typed(WaveformInteraction::BeginPan {
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
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
                            | WaveformActiveDragKind::SelectionMove(_)
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
                ..
            } if self.active_drag_kind
                == Some(WaveformActiveDragKind::Selection(
                    WaveformSelectionKind::Edit,
                ))
                || matches!(
                    self.active_drag_kind,
                    Some(
                        WaveformActiveDragKind::EditFade(_)
                            | WaveformActiveDragKind::SelectionMove(WaveformSelectionKind::Edit)
                    )
                ) =>
            {
                Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Auxiliary,
                ..
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
        self.append_extraction_history_paint(primitives, bounds);
        self.append_selection_and_marker_paint(primitives, bounds);
        self.append_edit_fade_paint(primitives, bounds);
    }
}

impl WaveformWidget {
    fn append_extraction_history_paint(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        for selection in &self.extraction_history {
            let Some((start, end)) = self.visible_range_for_selection(Some(*selection)) else {
                continue;
            };
            self.push_visible_range_fill(
                primitives,
                bounds,
                start,
                end,
                Rgba8 {
                    r: 155,
                    g: 155,
                    b: 155,
                    a: 54,
                },
            );
            self.append_selection_boundary_cursors(
                primitives,
                bounds,
                Some(*selection),
                Rgba8 {
                    r: 185,
                    g: 185,
                    b: 185,
                    a: 105,
                },
                1.0,
            );
        }
    }

    fn append_selection_and_marker_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        if let Some((start, end)) = self.visible_range_for_selection(self.play_selection) {
            let flash_active = self.play_selection_flash_frames > 0;
            let cursor_color = Rgba8 {
                r: 255,
                g: 142,
                b: 92,
                a: if flash_active { 255 } else { 230 },
            };
            self.push_visible_range_fill(
                primitives,
                bounds,
                start,
                end,
                Rgba8 {
                    r: 255,
                    g: 142,
                    b: 92,
                    a: if flash_active { 118 } else { 48 },
                },
            );
            self.append_selection_boundary_cursors(
                primitives,
                bounds,
                self.play_selection,
                cursor_color,
                1.25,
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
                    a: if flash_active { 255 } else { 220 },
                },
            );
            self.append_selection_move_handle(
                primitives,
                bounds,
                start,
                end,
                Rgba8 {
                    r: 255,
                    g: 142,
                    b: 92,
                    a: if flash_active { 245 } else { 185 },
                },
            );
        }
        if let Some((start, end)) = self.visible_range_for_selection(self.edit_selection) {
            let cursor_color = Rgba8 {
                r: 82,
                g: 168,
                b: 255,
                a: 230,
            };
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
            self.append_selection_boundary_cursors(
                primitives,
                bounds,
                self.edit_selection,
                cursor_color,
                1.25,
            );
            self.append_selection_move_handle(
                primitives,
                bounds,
                start,
                end,
                Rgba8 {
                    r: 82,
                    g: 168,
                    b: 255,
                    a: 180,
                },
            );
        }
        if self.play_selection.is_none()
            && let Some(play_mark_ratio) = self.visible_ratio_for_absolute(self.play_mark_ratio)
        {
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
        if self.edit_selection.is_none()
            && let Some(edit_mark_ratio) = self.visible_ratio_for_absolute(self.edit_mark_ratio)
        {
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

    fn append_selection_boundary_cursors(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        selection: Option<wavecrate::selection::SelectionRange>,
        color: Rgba8,
        width: f32,
    ) {
        let Some(selection) = selection else {
            return;
        };
        for ratio in [selection.start(), selection.end()] {
            if let Some(visible_ratio) = self.visible_ratio_for_absolute(Some(ratio)) {
                self.push_visible_cursor(primitives, bounds, visible_ratio, color, width);
            }
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

    fn append_selection_move_handle(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        start: f32,
        end: f32,
        color: Rgba8,
    ) {
        if let Some(rect) = self.selection_move_handle_rect(bounds, start, end) {
            self.push_fill(primitives, rect, color);
        }
    }

    fn selection_move_handle_at(
        &self,
        bounds: Rect,
        position: Point,
        kind: WaveformSelectionKind,
    ) -> bool {
        let range = match kind {
            WaveformSelectionKind::Play => self.play_selection,
            WaveformSelectionKind::Edit => self.edit_selection,
        };
        let Some((start, end)) = self.visible_range_for_selection(range) else {
            return false;
        };
        self.selection_move_handle_rect(bounds, start, end)
            .is_some_and(|rect| rect.contains(position))
    }

    fn selection_move_handle_rect(&self, bounds: Rect, start: f32, end: f32) -> Option<Rect> {
        let left = bounds.min.x + bounds.width() * start.min(end).clamp(0.0, 1.0);
        let right = bounds.min.x + bounds.width() * start.max(end).clamp(0.0, 1.0);
        if right <= left {
            return None;
        }
        let width = right - left;
        let inset = SELECTION_MOVE_HANDLE_END_INSET.min(width * 0.28);
        let handle_left = if width > inset * 2.0 + 1.0 {
            left + inset
        } else {
            left
        };
        let handle_right = if width > inset * 2.0 + 1.0 {
            right - inset
        } else {
            right
        };
        let height = SELECTION_MOVE_HANDLE_HEIGHT
            .min(bounds.height().max(1.0))
            .max(1.0);
        let handle_right = handle_right.max(handle_left + 1.0).min(bounds.max.x);
        if handle_right <= handle_left {
            return None;
        }
        Some(Rect::from_min_max(
            Point::new(handle_left, bounds.min.y),
            Point::new(handle_right, bounds.min.y + height),
        ))
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
        let cursor_width = width.ceil().max(2.0).min(bounds.width().max(1.0));
        let x = (bounds.min.x + bounds.width() * ratio.clamp(0.0, 1.0))
            .round()
            .clamp(bounds.min.x, bounds.max.x);
        let left = (x - cursor_width * 0.5).clamp(
            bounds.min.x,
            (bounds.max.x - cursor_width).max(bounds.min.x),
        );
        let right = (left + cursor_width).min(bounds.max.x);
        if right <= left {
            return;
        }
        self.push_fill(
            primitives,
            Rect::from_min_max(
                Point::new(left, bounds.min.y),
                Point::new(right, bounds.max.y),
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
        let visible_width = self.viewport.visible_items() as f32;
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
        let visible_width = self.viewport.visible_items() as f32;
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
        BAND_COUNT, WaveformActiveDragKind, WaveformEditFadeHandle, WaveformInteraction,
        WaveformSelectionEdge, WaveformSelectionKind, WaveformSignalWidget, WaveformState,
        WaveformWidget, split_frequency_bands, waveform_file_from_mono_samples,
    };
    use radiant::{
        gui::types::{Point, Rect, Vector2},
        runtime::{GpuSurfaceContent, PaintFillRect, PaintPrimitive},
        theme::ThemeTokens,
        widgets::{PointerButton, Widget, WidgetInput},
    };
    use std::{fs, sync::Arc};

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
    fn stereo_downmix_preserves_per_frame_peak_height_for_normalized_files() {
        let samples = vec![1.0, 0.0, -0.25, 0.25, 0.0, -1.0, 0.5, -0.75];

        assert_eq!(
            super::downmix_to_mono(&samples, 2, 4),
            vec![1.0, -0.25, -1.0, -0.75]
        );
    }

    #[test]
    fn stereo_downmix_avoids_phase_cancellation_in_visual_projection() {
        let samples = vec![1.0, -1.0, 0.35, -0.2];

        assert_eq!(super::downmix_to_mono(&samples, 2, 2), vec![1.0, 0.35]);
    }

    #[test]
    fn playmark_extraction_writes_sibling_wav_range() {
        let root = std::env::temp_dir().join(format!(
            "wavecrate-playmark-extract-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let source = root.join("source.wav");
        write_test_wav_i16(&source, &[0, 100, 200, 300, 400, 500]);
        let mut state = WaveformState::load_path(source).expect("load source");
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.75));

        let output = state
            .extract_play_selection_to_sibling()
            .expect("extract range");

        assert_eq!(output.file_name().unwrap(), "source_extraction.wav");
        assert_eq!(read_test_wav_i16(&output), vec![100, 200, 300, 400]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn playmark_extraction_uses_channel_independent_frame_bounds() {
        let root = std::env::temp_dir().join(format!(
            "wavecrate-playmark-extract-stereo-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("create temp root");
        let source = root.join("source.wav");
        write_test_wav_i16_stereo(
            &source,
            &[
                (0, 1),
                (100, 101),
                (200, 201),
                (300, 301),
                (400, 401),
                (500, 501),
            ],
        );
        let mut state = WaveformState::load_path(source).expect("load source");
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.75));

        let output = state
            .extract_play_selection_to_sibling()
            .expect("extract range");

        assert_eq!(
            read_test_wav_i16(&output),
            vec![100, 101, 200, 201, 300, 301, 400, 401]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn empty_waveform_rejects_playmark_extraction() {
        let mut state = WaveformState::empty();
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.1, 0.2));

        assert_eq!(
            state.extract_play_selection_to_sibling(),
            Err(String::from("Load a sample before extracting"))
        );
    }

    #[test]
    fn extraction_history_records_and_clears_play_selection() {
        let mut state = WaveformState::synthetic_for_tests();
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));

        state.record_current_play_selection_extracted();

        assert!(state.has_extraction_history());
        assert_eq!(
            state.extraction_history(),
            &[wavecrate::selection::SelectionRange::new(0.2, 0.6)]
        );

        state.clear_extraction_history();

        assert!(!state.has_extraction_history());
        assert!(state.extraction_history().is_empty());
    }

    #[test]
    fn extraction_history_merges_overlapping_ranges() {
        let mut state = WaveformState::synthetic_for_tests();

        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.1, 0.3));
        state.record_current_play_selection_extracted();
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.5));
        state.record_current_play_selection_extracted();
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.7, 0.8));
        state.record_current_play_selection_extracted();

        assert_eq!(
            state.extraction_history(),
            &[
                wavecrate::selection::SelectionRange::new(0.1, 0.5),
                wavecrate::selection::SelectionRange::new(0.7, 0.8),
            ]
        );
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
        assert!(raw_peak > 0.69);
    }

    #[test]
    fn frequency_bands_raw_lane_preserves_visual_peak_for_normalized_content() {
        let sample_rate = 48_000;
        let low = (0..sample_rate / 100)
            .map(|frame| {
                let t = frame as f32 / sample_rate as f32;
                (std::f32::consts::TAU * 70.0 * t).sin()
            })
            .collect::<Vec<_>>();
        let high = (0..sample_rate / 100)
            .map(|frame| {
                let t = frame as f32 / sample_rate as f32;
                (std::f32::consts::TAU * 4_000.0 * t).sin()
            })
            .collect::<Vec<_>>();

        for samples in [low, high] {
            let bands = split_frequency_bands(&samples, sample_rate);
            let raw_peak = bands
                .chunks_exact(BAND_COUNT)
                .map(|frame| frame[3].abs())
                .fold(0.0_f32, f32::max);

            assert!(
                (raw_peak - 1.0).abs() < 0.000_01,
                "raw display peak should track normalized sample peak, got {raw_peak}"
            );
        }
    }

    #[test]
    fn frequency_bands_normalize_short_low_content_to_raw_visual_peak() {
        let sample_rate = 48_000;
        let samples = (0..2_656)
            .map(|frame| {
                let t = frame as f32 / sample_rate as f32;
                (std::f32::consts::TAU * 72.0 * t).sin()
            })
            .collect::<Vec<_>>();

        let bands = split_frequency_bands(&samples, sample_rate);
        let low_peak = bands
            .chunks_exact(BAND_COUNT)
            .map(|frame| frame[0].abs())
            .fold(0.0_f32, f32::max);
        let raw_peak = bands
            .chunks_exact(BAND_COUNT)
            .map(|frame| frame[3].abs())
            .fold(0.0_f32, f32::max);

        assert!(raw_peak > 0.99, "raw peak was {raw_peak}");
        assert!(
            low_peak > raw_peak * 0.94,
            "short low content should not render visually undersized: low={low_peak}, raw={raw_peak}"
        );
    }

    #[test]
    fn frequency_bands_use_envelopes_to_avoid_low_zero_crossing_gaps() {
        let sample_rate = 48_000;
        let samples = (0..sample_rate / 20)
            .map(|frame| {
                let t = frame as f32 / sample_rate as f32;
                (std::f32::consts::TAU * 60.0 * t).sin()
            })
            .collect::<Vec<_>>();

        let bands = split_frequency_bands(&samples, sample_rate);
        let low_values = bands
            .chunks_exact(BAND_COUNT)
            .skip(sample_rate as usize / 50)
            .map(|frame| frame[0].abs())
            .collect::<Vec<_>>();
        let low_peak = low_values.iter().copied().fold(0.0_f32, f32::max);
        let low_floor = low_values.iter().copied().fold(f32::INFINITY, f32::min);

        assert!(low_peak > 0.94, "low envelope peak was {low_peak}");
        assert!(
            low_floor > low_peak * 0.55,
            "sustained low envelope should not collapse at zero crossings: floor={low_floor}, peak={low_peak}"
        );
    }

    #[test]
    fn frequency_bands_do_not_inflate_low_color_for_high_frequency_content() {
        let sample_rate = 48_000;
        let samples = (0..sample_rate / 100)
            .map(|frame| {
                let t = frame as f32 / sample_rate as f32;
                (std::f32::consts::TAU * 7_200.0 * t).sin()
            })
            .collect::<Vec<_>>();

        let bands = split_frequency_bands(&samples, sample_rate);
        let low_peak = bands
            .chunks_exact(BAND_COUNT)
            .map(|frame| frame[0].abs())
            .fold(0.0_f32, f32::max);
        let high_peak = bands
            .chunks_exact(BAND_COUNT)
            .map(|frame| frame[2].abs())
            .fold(0.0_f32, f32::max);

        assert!(high_peak > 0.30, "high peak was {high_peak}");
        assert!(
            low_peak < high_peak * 0.35,
            "mostly high-frequency content should not be painted as low-end blue: low={low_peak}, high={high_peak}"
        );
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
            state.extraction_history(),
            state.play_selection_flash_frames(),
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
    fn playhead_cursor_paints_pixel_stable_rect_when_progress_is_subpixel() {
        let mut state = WaveformState::synthetic_for_tests();
        state.start_playback(0.0);
        state.set_playhead_ratio(0.12345);
        let widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(400.0, 80.0)),
            &Default::default(),
            &ThemeTokens::default(),
        );

        let playhead = fill_rects(&primitives)
            .into_iter()
            .find(|fill| {
                (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (71, 220, 255, 245)
            })
            .expect("playhead fill paints");
        assert_eq!(playhead.rect.width(), 2.0);
        assert_eq!(playhead.rect.min.x.fract(), 0.0);
        assert_eq!(playhead.rect.max.x.fract(), 0.0);
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
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));
        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(100.0, 40.0),
                    button: PointerButton::Auxiliary,
                    modifiers: Default::default(),
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
        assert_eq!(state.viewport().visible_items(), 24_000);
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
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(60.0, 40.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
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
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(160.0, 40.0),
                    button: PointerButton::Secondary,
                    modifiers: Default::default(),
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
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
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
    fn playmark_top_handle_moves_range_without_resizing() {
        let mut state = WaveformState::synthetic_for_tests();
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
        state.play_mark_ratio = Some(0.2);

        state.apply_interaction(WaveformInteraction::BeginSelectionMove {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.4,
        });
        state.apply_interaction(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.55,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection {
            visible_ratio: 0.55,
        });

        let selection = state.play_selection().expect("moved playmark selection");
        assert!((selection.start() - 0.35).abs() < 0.001);
        assert!((selection.end() - 0.75).abs() < 0.001);
        assert!((selection.width() - 0.4).abs() < 0.001);
        assert_eq!(state.play_mark_ratio(), Some(selection.start()));
        assert!(!state.is_playing());
    }

    #[test]
    fn edit_top_handle_moves_range_and_preserves_edit_effects() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_in(0.25, 0.2)
                .with_fade_out(0.25, 0.7),
        );
        state.edit_mark_ratio = Some(0.2);

        state.apply_interaction(WaveformInteraction::BeginSelectionMove {
            kind: WaveformSelectionKind::Edit,
            visible_ratio: 0.4,
        });
        state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.1 });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.1 });

        let selection = state.edit_selection().expect("moved edit selection");
        assert!((selection.start() - 0.0).abs() < 0.001);
        assert!((selection.end() - 0.4).abs() < 0.001);
        assert_eq!(state.edit_mark_ratio(), Some(selection.start()));
        assert_eq!(selection.fade_in().map(|fade| fade.length), Some(0.25));
        assert_eq!(selection.fade_out().map(|fade| fade.length), Some(0.25));
    }

    #[test]
    fn primary_press_on_playmark_handle_starts_resize_instead_of_new_selection() {
        let mut state = WaveformState::synthetic_for_tests();
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
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
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(120.0, 8.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
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
    fn primary_press_on_playmark_top_handle_starts_move() {
        let mut state = WaveformState::synthetic_for_tests();
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
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
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(80.0, 3.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                },
            )
            .expect("playmark move interaction");
        let interaction = output
            .typed_ref::<WaveformInteraction>()
            .copied()
            .expect("waveform interaction");

        assert_eq!(
            interaction,
            WaveformInteraction::BeginSelectionMove {
                kind: WaveformSelectionKind::Play,
                visible_ratio: 0.4
            }
        );
    }

    #[test]
    fn secondary_press_on_edit_top_handle_starts_move() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
        state.edit_mark_ratio = Some(0.2);
        let mut widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(80.0, 3.0),
                    button: PointerButton::Secondary,
                    modifiers: Default::default(),
                },
            )
            .expect("edit move interaction");
        let interaction = output
            .typed_ref::<WaveformInteraction>()
            .copied()
            .expect("waveform interaction");

        assert_eq!(
            interaction,
            WaveformInteraction::BeginSelectionMove {
                kind: WaveformSelectionKind::Edit,
                visible_ratio: 0.4
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
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
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
            Some(wavecrate::selection::SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2));

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
            Some(wavecrate::selection::SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.7));
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
            Some(wavecrate::selection::SelectionRange::new(0.2, 0.6).with_fade_in(0.25, 0.2));
        let mut widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(40.0, 40.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
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
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
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
    fn edit_fade_out_bottom_handle_keeps_crossfade_handles_stable() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_in(0.25, 0.2)
                .with_fade_in_mute(0.25)
                .with_fade_out(0.25, 0.7)
                .with_fade_out_mute(0.25),
        );

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeOutEnd,
            visible_ratio: 0.6,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.7 });

        let selection = state.edit_selection().expect("edit selection");
        let fade_in = selection.fade_in().expect("fade-in should remain");
        let fade_out = selection.fade_out().expect("fade-out should remain");
        let fade_in_end = selection.start() + selection.width() * fade_in.length;
        let fade_in_outer_start = selection.start() - selection.width() * fade_in.mute;
        let fade_out_start = selection.end() - selection.width() * fade_out.length;
        let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

        assert!((selection.start() - 0.2).abs() < 0.001);
        assert!((selection.end() - 0.7).abs() < 0.001);
        assert!((fade_in_end - 0.3).abs() < 0.001);
        assert!((fade_in_outer_start - 0.1).abs() < 0.001);
        assert!((fade_out_start - 0.5).abs() < 0.001);
        assert!((fade_out_outer_end - 0.7).abs() < 0.001);
        assert!((fade_in.curve - 0.2).abs() < 0.001);
        assert!((fade_out.curve - 0.7).abs() < 0.001);
    }

    #[test]
    fn edit_fade_out_bottom_handle_preserves_crossfade_when_fade_collapses() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_out(0.25, 0.7)
                .with_fade_out_mute(1.0),
        );

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeOutEnd,
            visible_ratio: 0.6,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.5 });

        let selection = state.edit_selection().expect("edit selection");
        let fade_out = selection
            .fade_out()
            .expect("fade-out silence handle should remain");
        let fade_out_start = selection.end() - selection.width() * fade_out.length;
        let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

        assert!((selection.start() - 0.2).abs() < 0.001);
        assert!((selection.end() - 0.5).abs() < 0.001);
        assert!((fade_out_start - 0.5).abs() < 0.001);
        assert!((fade_out_outer_end - 1.0).abs() < 0.001);
        assert!(fade_out.length.abs() < 0.001);
        assert!((fade_out.curve - 0.7).abs() < 0.001);
    }

    #[test]
    fn edit_fade_out_bottom_handle_does_not_pick_up_silence_during_same_drag() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_out(0.25, 0.7)
                .with_fade_out_mute(1.0),
        );

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeOutEnd,
            visible_ratio: 0.6,
        });
        state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 1.0 });
        state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: 0.7 });

        let selection = state.edit_selection().expect("edit selection");
        let fade_out = selection
            .fade_out()
            .expect("fade-out silence handle should remain");
        let fade_out_start = selection.end() - selection.width() * fade_out.length;
        let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

        assert!((selection.end() - 0.7).abs() < 0.001);
        assert!((fade_out_start - 0.5).abs() < 0.001);
        assert!((fade_out_outer_end - 1.0).abs() < 0.000_001);
    }

    #[test]
    fn edit_fade_out_bottom_handle_keeps_collapsed_silence_after_release() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_out(0.25, 0.7)
                .with_fade_out_mute(1.0),
        );

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeOutEnd,
            visible_ratio: 0.6,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 1.0 });
        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeOutEnd,
            visible_ratio: 1.0,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.7 });

        let selection = state.edit_selection().expect("edit selection");
        let fade_out = selection.fade_out().expect("fade-out should remain");
        let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

        assert!((selection.end() - 0.7).abs() < 0.001);
        assert!((fade_out_outer_end - 0.7).abs() < 0.000_001);
    }

    #[test]
    fn double_click_outer_fade_handles_collapses_silence_without_clearing_fade() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_in(0.25, 0.2)
                .with_fade_in_mute(0.5)
                .with_fade_out(0.25, 0.7)
                .with_fade_out_mute(0.75),
        );

        state.apply_interaction(WaveformInteraction::ClearEditFadeSilence {
            handle: WaveformEditFadeHandle::FadeInOuterStart,
        });
        state.apply_interaction(WaveformInteraction::ClearEditFadeSilence {
            handle: WaveformEditFadeHandle::FadeOutOuterEnd,
        });

        let selection = state.edit_selection().expect("edit selection");
        let fade_in = selection.fade_in().expect("fade-in should remain");
        let fade_out = selection.fade_out().expect("fade-out should remain");
        assert!((fade_in.length - 0.25).abs() < 0.001);
        assert!((fade_in.curve - 0.2).abs() < 0.001);
        assert!(fade_in.mute.abs() < 0.001);
        assert!((fade_out.length - 0.25).abs() < 0.001);
        assert!((fade_out.curve - 0.7).abs() < 0.001);
        assert!(fade_out.mute.abs() < 0.001);
    }

    #[test]
    fn double_click_on_outer_fade_handle_emits_silence_clear_interaction() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_out(0.25, 0.7)
                .with_fade_out_mute(0.25),
        );
        let mut widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerDoubleClick {
                    position: Point::new(140.0, 40.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                },
            )
            .expect("outer fade double-click interaction");
        let interaction = output
            .typed_ref::<WaveformInteraction>()
            .copied()
            .expect("waveform interaction");

        assert_eq!(
            interaction,
            WaveformInteraction::ClearEditFadeSilence {
                handle: WaveformEditFadeHandle::FadeOutOuterEnd
            }
        );
    }

    #[test]
    fn edit_fade_out_top_handle_preserves_silence_after_bottom_handle_collapse() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_out(0.25, 0.7)
                .with_fade_out_mute(1.0),
        );

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeOutEnd,
            visible_ratio: 0.6,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.5 });
        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeOutStart,
            visible_ratio: 0.5,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection {
            visible_ratio: 0.45,
        });

        let selection = state.edit_selection().expect("edit selection");
        let fade_out = selection
            .fade_out()
            .expect("fade-out silence handle should remain");
        let fade_out_start = selection.end() - selection.width() * fade_out.length;
        let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

        assert!((selection.end() - 0.5).abs() < 0.001);
        assert!((fade_out_start - 0.45).abs() < 0.001);
        assert!((fade_out_outer_end - 1.0).abs() < 0.000_001);
    }

    #[test]
    fn edit_fade_out_bottom_handle_keeps_left_crossfade_pinned_to_sample_edge() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_in(0.25, 0.2)
                .with_fade_in_mute(0.5)
                .with_fade_out(0.25, 0.7),
        );

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeOutEnd,
            visible_ratio: 0.6,
        });
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.7 });

        let selection = state.edit_selection().expect("edit selection");
        let fade_in = selection.fade_in().expect("fade-in should remain");
        let fade_in_outer_start = selection.start() - selection.width() * fade_in.mute;

        assert!(fade_in_outer_start.abs() < 0.000_001);
    }

    #[test]
    fn edit_fade_out_bottom_handle_keeps_left_crossfade_pinned_across_wiggles() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_in(0.25, 0.2)
                .with_fade_in_mute(0.5)
                .with_fade_out(0.25, 0.7),
        );

        state.apply_interaction(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::FadeOutEnd,
            visible_ratio: 0.6,
        });
        for visible_ratio in [0.7, 0.69, 0.71, 0.705, 0.7] {
            state.apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio });
            let selection = state.edit_selection().expect("edit selection");
            let fade_in = selection.fade_in().expect("fade-in should remain");
            let fade_in_outer_start = selection.start() - selection.width() * fade_in.mute;
            assert!(
                fade_in_outer_start.abs() < 0.000_001,
                "left silence handle drifted to {fade_in_outer_start}"
            );
        }
        state.apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: 0.7 });
    }

    #[test]
    fn edit_fade_in_bottom_handle_keeps_opposite_fade_boundary_stable() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
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
    fn edit_fade_in_bottom_handle_keeps_crossfade_handles_stable() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
                .with_fade_in(0.25, 0.2)
                .with_fade_in_mute(0.25)
                .with_fade_out(0.25, 0.7)
                .with_fade_out_mute(0.25),
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
        let fade_in_outer_start = selection.start() - selection.width() * fade_in.mute;
        let fade_out_start = selection.end() - selection.width() * fade_out.length;
        let fade_out_outer_end = selection.end() + selection.width() * fade_out.mute;

        assert!((selection.start() - 0.1).abs() < 0.001);
        assert!((selection.end() - 0.6).abs() < 0.001);
        assert!((fade_in_end - 0.3).abs() < 0.001);
        assert!((fade_in_outer_start - 0.1).abs() < 0.001);
        assert!((fade_out_start - 0.5).abs() < 0.001);
        assert!((fade_out_outer_end - 0.7).abs() < 0.001);
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
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0));

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(40.0, 4.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
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
            state.extraction_history(),
            state.play_selection_flash_frames(),
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
            state.extraction_history(),
            state.play_selection_flash_frames(),
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
        assert!(fills.iter().any(|fill| {
            (fill.rect.center().x - 120.0).abs() < 1.0
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (255, 142, 92, 230)
        }));
    }

    #[test]
    fn extraction_history_paints_soft_gray_range_under_active_selection() {
        let mut state = WaveformState::synthetic_for_tests();
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.3, 0.5));
        state.record_current_play_selection_extracted();
        state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
        let widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
            &Default::default(),
            &ThemeTokens::default(),
        );

        let fills = fill_rects(&primitives);
        let history_index = fills
            .iter()
            .position(|fill| {
                (fill.rect.min.x - 60.0).abs() < 0.001
                    && (fill.rect.max.x - 100.0).abs() < 0.001
                    && (fill.color.r, fill.color.g, fill.color.b, fill.color.a)
                        == (155, 155, 155, 54)
            })
            .expect("history fill");
        let active_index = fills
            .iter()
            .position(|fill| {
                (fill.rect.min.x - 40.0).abs() < 0.001
                    && (fill.rect.max.x - 120.0).abs() < 0.001
                    && (fill.color.r, fill.color.g, fill.color.b, fill.color.a)
                        == (255, 142, 92, 48)
            })
            .expect("active play selection fill");
        assert!(history_index < active_index);
    }

    #[test]
    fn edit_selection_paints_start_and_end_boundary_lines() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
        let widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
            state.play_mark_ratio(),
            state.edit_mark_ratio(),
            state.play_selection(),
            state.edit_selection(),
            state.extraction_history(),
            state.play_selection_flash_frames(),
            state.active_drag_kind(),
        );
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
            &Default::default(),
            &ThemeTokens::default(),
        );

        let fills = fill_rects(&primitives);
        assert!(fills.iter().any(|fill| {
            (fill.rect.center().x - 40.0).abs() < 1.0
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 230)
        }));
        assert!(fills.iter().any(|fill| {
            (fill.rect.center().x - 120.0).abs() < 1.0
                && (fill.color.r, fill.color.g, fill.color.b, fill.color.a) == (82, 168, 255, 230)
        }));
    }

    #[test]
    fn edit_fade_curve_paints_volume_trace_as_overlay_rects() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(
            wavecrate::selection::SelectionRange::new(0.2, 0.6)
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
            state.extraction_history(),
            state.play_selection_flash_frames(),
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
        let widget = WaveformSignalWidget::new(
            state.file(),
            state.viewport(),
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
    fn signal_widget_attaches_active_edit_fade_gain_preview() {
        let file = Arc::new(waveform_file_from_mono_samples(
            "fade-preview.wav".into(),
            Arc::from([]),
            48_000,
            1,
            vec![1.0; 16],
        ));
        let viewport = super::WaveformViewport::full(file.frames);
        let edit_selection =
            Some(wavecrate::selection::SelectionRange::new(0.0, 1.0).with_fade_in(1.0, 0.0));
        let widget = WaveformSignalWidget::new(Arc::clone(&file), viewport, edit_selection, None);
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
        let GpuSurfaceContent::SignalSummaryBands {
            summary,
            gain_preview,
            ..
        } = &surface.content
        else {
            panic!("expected signal summary bands");
        };
        assert!(Arc::ptr_eq(summary, &file.gpu_signal_summary));
        let preview = gain_preview.expect("edit fade gain preview");
        assert_eq!(preview.start, 0.0);
        assert_eq!(preview.end, 1.0);
        assert_eq!(preview.fade_in_length, 1.0);
        assert_eq!(preview.fade_in_curve, 0.0);
    }

    #[test]
    fn signal_widget_revision_changes_when_same_path_audio_bytes_change() {
        let first = Arc::new(waveform_file_from_mono_samples(
            "same-path.wav".into(),
            Arc::from([1_u8, 2, 3, 4]),
            48_000,
            1,
            vec![0.25; 16],
        ));
        let second = Arc::new(waveform_file_from_mono_samples(
            "same-path.wav".into(),
            Arc::from([4_u8, 3, 2, 1]),
            48_000,
            1,
            vec![1.0; 16],
        ));

        let first_revision = gpu_surface_revision_for_file(first);
        let second_revision = gpu_surface_revision_for_file(second);

        assert_ne!(first_revision, second_revision);
    }

    #[test]
    fn signal_widget_keeps_summary_cached_during_live_edit_fade_drag() {
        let file = Arc::new(waveform_file_from_mono_samples(
            "fade-preview.wav".into(),
            Arc::from([]),
            48_000,
            1,
            vec![1.0; 16],
        ));
        let viewport = super::WaveformViewport::full(file.frames);
        let edit_selection =
            Some(wavecrate::selection::SelectionRange::new(0.0, 1.0).with_fade_in(1.0, 0.0));
        let widget = WaveformSignalWidget::new(
            Arc::clone(&file),
            viewport,
            edit_selection,
            Some(WaveformActiveDragKind::EditFade(
                WaveformEditFadeHandle::FadeInEnd,
            )),
        );
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
        let GpuSurfaceContent::SignalSummaryBands {
            summary,
            gain_preview,
            ..
        } = &surface.content
        else {
            panic!("expected signal summary bands");
        };
        assert!(Arc::ptr_eq(summary, &file.gpu_signal_summary));
        assert!(gain_preview.is_some());
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

    fn gpu_surface_revision_for_file(file: Arc<super::WaveformFile>) -> u64 {
        let viewport = super::WaveformViewport::full(file.frames);
        let widget = WaveformSignalWidget::new(file, viewport, None, None);
        let mut primitives = Vec::new();
        widget.append_paint(
            &mut primitives,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
            &Default::default(),
            &ThemeTokens::default(),
        );
        primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::GpuSurface(surface) => Some(surface.revision),
                _ => None,
            })
            .expect("waveform gpu surface")
    }

    fn write_test_wav_i16(path: &std::path::Path, samples: &[i16]) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
        for sample in samples {
            writer.write_sample(*sample).expect("write sample");
        }
        writer.finalize().expect("finalize wav");
    }

    fn write_test_wav_i16_stereo(path: &std::path::Path, frames: &[(i16, i16)]) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
        for (left, right) in frames {
            writer.write_sample(*left).expect("write left sample");
            writer.write_sample(*right).expect("write right sample");
        }
        writer.finalize().expect("finalize wav");
    }

    fn read_test_wav_i16(path: &std::path::Path) -> Vec<i16> {
        let mut reader = hound::WavReader::open(path).expect("open wav");
        reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .expect("read samples")
    }
}
