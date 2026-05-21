#![allow(missing_docs)]

use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    gui::{range::NormalizedRange, visualization::TimelineEditPreview},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{PaintFillRect, PaintPrimitive},
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, PaintBounds, PointerButton, Widget, WidgetCommon, WidgetInput, WidgetOutput,
        WidgetSizing,
    },
};
use std::{path::PathBuf, sync::Arc};

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

mod types;
pub(super) use types::{
    WaveformActiveDragKind, WaveformEditFadeHandle, WaveformInteraction, WaveformSelectionEdge,
    WaveformSelectionKind,
};

mod interaction;
use interaction::{
    WaveformDrag, WaveformEditFadeDrag, WaveformPanDrag, WaveformSelectionDrag,
    WaveformSelectionMoveDrag, WaveformSelectionResizeDrag, edit_preview_for_selection,
};

mod audio_file;
use audio_file::{
    WaveformFile, empty_waveform_file, extract_wav_range_to_sibling, is_wav_path,
    load_waveform_file,
};
#[cfg(test)]
use audio_file::{
    downmix_to_mono, split_frequency_bands, synthetic_waveform_file,
    waveform_file_from_mono_samples,
};

mod signal_widget;
use signal_widget::WaveformSignalWidget;

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
            WaveformWidget::new(WaveformWidgetProps::from_state(state)),
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

#[derive(Clone, Debug)]
struct WaveformWidgetProps {
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    playhead_ratio: Option<f32>,
    play_mark_ratio: Option<f32>,
    edit_mark_ratio: Option<f32>,
    play_selection: Option<wavecrate::selection::SelectionRange>,
    edit_selection: Option<wavecrate::selection::SelectionRange>,
    play_selection_flash_frames: u8,
    active_drag_kind: Option<WaveformActiveDragKind>,
}

impl WaveformWidgetProps {
    fn from_state(state: &WaveformState) -> Self {
        Self {
            file: state.file(),
            viewport: state.viewport(),
            playhead_ratio: state.playhead_ratio(),
            play_mark_ratio: state.play_mark_ratio(),
            edit_mark_ratio: state.edit_mark_ratio(),
            play_selection: state.play_selection(),
            edit_selection: state.edit_selection(),
            play_selection_flash_frames: state.play_selection_flash_frames(),
            active_drag_kind: state.active_drag_kind(),
        }
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
    play_selection_flash_frames: u8,
    edit_preview: TimelineEditPreview,
    active_drag_kind: Option<WaveformActiveDragKind>,
}

impl WaveformWidget {
    fn new(props: WaveformWidgetProps) -> Self {
        let WaveformWidgetProps {
            file,
            viewport,
            playhead_ratio,
            play_mark_ratio,
            edit_mark_ratio,
            play_selection,
            edit_selection,
            play_selection_flash_frames,
            active_drag_kind,
        } = props;
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
        WaveformWidget, WaveformWidgetProps, split_frequency_bands,
        waveform_file_from_mono_samples,
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

        let widget = waveform_widget_for_state(&state);
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
        let widget = waveform_widget_for_state(&state);
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
        let mut widget = waveform_widget_for_state(&state);
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
        let mut widget = waveform_widget_for_state(&state);
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
        let mut widget = waveform_widget_for_state(&state);
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
        let mut widget = waveform_widget_for_state(&state);
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
        let mut widget = waveform_widget_for_state(&state);
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
        let mut widget = waveform_widget_for_state(&state);
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
        let mut widget = waveform_widget_for_state(&state);
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
        let mut widget = waveform_widget_for_state(&state);
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
        let mut widget = waveform_widget_for_state(&state);
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
        let widget = waveform_widget_for_state(&state);
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
        let widget = waveform_widget_for_state(&state);
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
    fn edit_selection_paints_start_and_end_boundary_lines() {
        let mut state = WaveformState::synthetic_for_tests();
        state.edit_selection = Some(wavecrate::selection::SelectionRange::new(0.2, 0.6));
        let widget = waveform_widget_for_state(&state);
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
        let widget = waveform_widget_for_state(&state);
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

    fn waveform_widget_for_state(state: &WaveformState) -> WaveformWidget {
        WaveformWidget::new(WaveformWidgetProps::from_state(state))
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
