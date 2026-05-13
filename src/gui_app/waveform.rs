#![allow(missing_docs)]

use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle, GpuSurfaceRuntimeOverlays,
        PaintGpuSurface, PaintPrimitive,
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
    play_selection: Option<sempal::selection::SelectionRange>,
    edit_selection: Option<sempal::selection::SelectionRange>,
    active_drag: Option<WaveformSelectionDrag>,
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

    pub(super) fn play_selection(&self) -> Option<sempal::selection::SelectionRange> {
        self.play_selection
    }

    pub(super) fn edit_selection(&self) -> Option<sempal::selection::SelectionRange> {
        self.edit_selection
    }

    fn active_drag_kind(&self) -> Option<WaveformSelectionKind> {
        self.active_drag.map(|drag| drag.kind)
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
                self.active_drag = Some(WaveformSelectionDrag::new(kind, ratio));
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
            WaveformInteraction::UpdateSelection { visible_ratio } => {
                self.update_active_selection(visible_ratio);
            }
            WaveformInteraction::FinishSelection { visible_ratio } => {
                self.finish_active_selection(visible_ratio);
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

    fn update_active_selection(&mut self, visible_ratio: f32) {
        let ratio = self.absolute_ratio_from_visible(visible_ratio);
        let Some(mut drag) = self.active_drag else {
            return;
        };
        drag.update(ratio);
        self.active_drag = Some(drag);
        if drag.moved {
            self.set_selection_for_drag(drag);
        }
    }

    fn finish_active_selection(&mut self, visible_ratio: f32) {
        let ratio = self.absolute_ratio_from_visible(visible_ratio);
        let Some(mut drag) = self.active_drag.take() else {
            return;
        };
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

    fn set_selection_for_drag(&mut self, drag: WaveformSelectionDrag) {
        let range = sempal::selection::SelectionRange::new(drag.anchor_ratio, drag.current_ratio);
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

#[derive(Clone, Debug)]
pub(super) struct WaveformFile {
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
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
        sempal::waveform::WaveformRenderer::new(WAVEFORM_WIDTH as u32, WAVEFORM_HEIGHT as u32)
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
        gpu_signal_summary,
    }
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
struct WaveformWidget {
    common: WidgetCommon,
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    playhead_ratio: Option<f32>,
    play_mark_ratio: Option<f32>,
    edit_mark_ratio: Option<f32>,
    play_selection: Option<sempal::selection::SelectionRange>,
    edit_selection: Option<sempal::selection::SelectionRange>,
    active_drag_kind: Option<WaveformSelectionKind>,
}

impl WaveformWidget {
    fn new(
        file: Arc<WaveformFile>,
        viewport: WaveformViewport,
        _cursor_ratio: Option<f32>,
        playhead_ratio: Option<f32>,
        play_mark_ratio: Option<f32>,
        edit_mark_ratio: Option<f32>,
        play_selection: Option<sempal::selection::SelectionRange>,
        edit_selection: Option<sempal::selection::SelectionRange>,
        active_drag_kind: Option<WaveformSelectionKind>,
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
                Some(WidgetOutput::typed(WaveformInteraction::BeginSelection {
                    kind: WaveformSelectionKind::Play,
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Secondary,
            } if bounds.contains(position) => {
                Some(WidgetOutput::typed(WaveformInteraction::BeginSelection {
                    kind: WaveformSelectionKind::Edit,
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } if self.active_drag_kind == Some(WaveformSelectionKind::Play) => {
                Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Secondary,
            } if self.active_drag_kind == Some(WaveformSelectionKind::Edit) => {
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
        primitives.push(PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: self.common.id,
            key: self.file.path_hash(),
            revision: 0,
            rect: bounds,
            content: GpuSurfaceContent::SignalSummaryBands {
                frames: self.file.frames,
                band_count: BAND_COUNT,
                frame_range: [self.viewport.start as f32, self.viewport.end as f32],
                summary: Arc::clone(&self.file.gpu_signal_summary),
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
            overlays: self.cursor_overlays(),
        }));
    }
}

impl WaveformWidget {
    fn cursor_overlays(&self) -> Vec<radiant::runtime::GpuSurfaceOverlay> {
        let mut overlays = Vec::new();
        if let Some((start, end)) = self.visible_range_for_selection(self.play_selection) {
            overlays.push(radiant::runtime::GpuSurfaceOverlay::HorizontalRange {
                start,
                end,
                color: Rgba8 {
                    r: 255,
                    g: 142,
                    b: 92,
                    a: 48,
                },
            });
        }
        if let Some((start, end)) = self.visible_range_for_selection(self.edit_selection) {
            overlays.push(radiant::runtime::GpuSurfaceOverlay::HorizontalRange {
                start,
                end,
                color: Rgba8 {
                    r: 82,
                    g: 168,
                    b: 255,
                    a: 46,
                },
            });
        }
        if let Some(play_mark_ratio) = self.visible_ratio_for_absolute(self.play_mark_ratio) {
            overlays.push(radiant::runtime::GpuSurfaceOverlay::VerticalCursor {
                ratio: play_mark_ratio,
                color: Rgba8 {
                    r: 255,
                    g: 142,
                    b: 92,
                    a: 230,
                },
                width: 1.25,
            });
        }
        if let Some(edit_mark_ratio) = self.visible_ratio_for_absolute(self.edit_mark_ratio) {
            overlays.push(radiant::runtime::GpuSurfaceOverlay::VerticalCursor {
                ratio: edit_mark_ratio,
                color: Rgba8 {
                    r: 82,
                    g: 168,
                    b: 255,
                    a: 230,
                },
                width: 1.25,
            });
        }
        if let Some(playhead_ratio) = self.visible_ratio_for_absolute(self.playhead_ratio) {
            overlays.push(radiant::runtime::GpuSurfaceOverlay::VerticalCursor {
                ratio: playhead_ratio,
                color: Rgba8 {
                    r: 71,
                    g: 220,
                    b: 255,
                    a: 245,
                },
                width: 1.75,
            });
        }
        overlays
    }

    fn visible_range_for_selection(
        &self,
        range: Option<sempal::selection::SelectionRange>,
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
        BAND_COUNT, WaveformInteraction, WaveformSelectionKind, WaveformState, WaveformWidget,
        split_frequency_bands, waveform_file_from_mono_samples,
    };
    use radiant::{
        gui::types::{Point, Rect, Vector2},
        runtime::{GpuSurfaceContent, PaintPrimitive},
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
    fn cursor_overlays_project_play_edit_and_playhead_ratios() {
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

        let overlays = widget.cursor_overlays();
        assert_eq!(overlays.len(), 3);
        match overlays[0] {
            radiant::runtime::GpuSurfaceOverlay::VerticalCursor {
                ratio,
                color,
                width,
            } => {
                assert!((ratio - 0.125).abs() < 0.001);
                assert_eq!((color.r, color.g, color.b), (255, 142, 92));
                assert_eq!(width, 1.25);
            }
            radiant::runtime::GpuSurfaceOverlay::RuntimeVerticalLine { .. } => {
                panic!("play mark overlay should be app-owned");
            }
            radiant::runtime::GpuSurfaceOverlay::HorizontalRange { .. } => {
                panic!("play mark overlay should be a vertical line");
            }
        }
        match overlays[1] {
            radiant::runtime::GpuSurfaceOverlay::VerticalCursor {
                ratio,
                color,
                width,
            } => {
                assert!((ratio - 0.375).abs() < 0.001);
                assert_eq!((color.r, color.g, color.b), (82, 168, 255));
                assert_eq!(width, 1.25);
            }
            radiant::runtime::GpuSurfaceOverlay::RuntimeVerticalLine { .. } => {
                panic!("edit mark overlay should be app-owned");
            }
            radiant::runtime::GpuSurfaceOverlay::HorizontalRange { .. } => {
                panic!("edit mark overlay should be a vertical line");
            }
        }
        match overlays[2] {
            radiant::runtime::GpuSurfaceOverlay::VerticalCursor {
                ratio,
                color,
                width,
            } => {
                assert!((ratio - 0.25).abs() < 0.001);
                assert_eq!((color.r, color.g, color.b), (71, 220, 255));
                assert_eq!(width, 1.75);
            }
            radiant::runtime::GpuSurfaceOverlay::RuntimeVerticalLine { .. } => {
                panic!("playhead overlay should be app-owned");
            }
            radiant::runtime::GpuSurfaceOverlay::HorizontalRange { .. } => {
                panic!("playhead overlay should be a vertical line");
            }
        }
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
    fn selection_fill_paints_as_gpu_surface_overlay() {
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

        assert!(surface.overlays.iter().any(|overlay| matches!(
            overlay,
            radiant::runtime::GpuSurfaceOverlay::HorizontalRange { start, end, .. }
                if (*start - 0.2).abs() < 0.001 && (*end - 0.6).abs() < 0.001
        )));
        assert!(surface.overlays.iter().any(|overlay| matches!(
            overlay,
            radiant::runtime::GpuSurfaceOverlay::VerticalCursor { ratio, .. }
                if (*ratio - 0.2).abs() < 0.001
        )));
    }
}
