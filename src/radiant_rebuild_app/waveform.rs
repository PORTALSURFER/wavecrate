#![allow(missing_docs)]

use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{
        GpuHoverCursor, GpuSurfaceCapabilities, GpuSurfaceContent, PaintGpuSurface, PaintPrimitive,
    },
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, PaintBounds, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

use super::RebuildMessage;

const WAVEFORM_WIDTH: usize = 1200;
const WAVEFORM_HEIGHT: usize = 320;
const MIN_VISIBLE_FRAMES: usize = 256;
const BAND_COUNT: usize = 4;
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

    pub(super) fn start_playback(&mut self) {
        self.playing = true;
        self.playhead_ratio = Some(0.0);
        self.zoom_anchor_ratio = 0.0;
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
            WaveformInteraction::Frame => {
                // Playback progress is driven by the audio engine; frames only keep repainting.
            }
        }
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum WaveformInteraction {
    Wheel { delta: Vector2, anchor_ratio: f32 },
    ScrollTo { offset_fraction: f32 },
    Frame,
}

#[derive(Clone, Debug)]
pub(super) struct WaveformFile {
    path: PathBuf,
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

pub(super) fn waveform_viewport_view(state: &WaveformState) -> ui::View<super::RebuildMessage> {
    ui::custom_widget(
        WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
        ),
        |output| {
            output
                .typed_ref::<WaveformInteraction>()
                .copied()
                .map(RebuildMessage::Waveform)
        },
    )
    .id(10)
    .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)
}

fn load_waveform_file(path: PathBuf) -> Result<WaveformFile, String> {
    if is_wav_path(&path) {
        if let Ok(file) = load_wav_waveform_file(path.clone()) {
            return Ok(file);
        }
    }
    let bytes = fs::read(&path).map_err(|err| format!("failed to read audio file: {err}"))?;
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
        SYNTHETIC_SAMPLE_RATE,
        1,
        samples,
    )
}

fn waveform_file_from_mono_samples(
    path: PathBuf,
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

fn load_wav_waveform_file(path: PathBuf) -> Result<WaveformFile, String> {
    let mut reader =
        hound::WavReader::open(&path).map_err(|err| format!("failed to open WAV: {err}"))?;
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
}

impl WaveformWidget {
    fn new(
        file: Arc<WaveformFile>,
        viewport: WaveformViewport,
        _cursor_ratio: Option<f32>,
        playhead_ratio: Option<f32>,
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
            WidgetInput::PointerMove { position } if bounds.contains(position) => {
                self.common.state.hovered = true;
                None
            }
            WidgetInput::PointerMove { .. } => {
                self.common.state.hovered = false;
                None
            }
            WidgetInput::Wheel { position, delta } if bounds.contains(position) => {
                Some(WidgetOutput::typed(WaveformInteraction::Wheel {
                    delta,
                    anchor_ratio: self.ratio_from_position(bounds, position),
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
                native_hover_cursor: Some(GpuHoverCursor {
                    color: Rgba8 {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 235,
                    },
                    width: 1.5,
                }),
            },
            overlays: self.playhead_overlay(),
        }));
    }
}

impl WaveformWidget {
    fn playhead_overlay(&self) -> Vec<radiant::runtime::GpuSurfaceOverlay> {
        let Some(playhead_ratio) = self.playhead_ratio else {
            return Vec::new();
        };
        let playhead_frame = playhead_ratio.clamp(0.0, 1.0) * self.file.frames.max(1) as f32;
        let visible_start = self.viewport.start as f32;
        let visible_width = self.viewport.visible_frames() as f32;
        let visible_ratio = (playhead_frame - visible_start) / visible_width.max(1.0);
        if !(0.0..=1.0).contains(&visible_ratio) {
            return Vec::new();
        }
        vec![radiant::runtime::GpuSurfaceOverlay::VerticalCursor {
            ratio: visible_ratio,
            color: Rgba8 {
                r: 71,
                g: 220,
                b: 255,
                a: 245,
            },
            width: 1.75,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::{
        split_frequency_bands, waveform_file_from_mono_samples, WaveformState, WaveformWidget,
        BAND_COUNT,
    };

    #[test]
    fn waveform_summary_preserves_raw_transient_detail() {
        let samples = vec![0.0, 0.12, -0.9, 0.08, 0.0, 0.42, -0.18, 0.0];

        let file = waveform_file_from_mono_samples("test.wav".into(), 48_000, 1, samples.clone());

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

        state.start_playback();
        assert!(state.is_playing());
        assert_eq!(state.playhead_ratio(), Some(0.0));

        state.set_playhead_ratio(0.375);
        assert_eq!(state.playhead_ratio(), Some(0.375));

        state.stop_playback();
        assert!(!state.is_playing());
        assert_eq!(state.playhead_ratio(), None);
    }

    #[test]
    fn playhead_overlay_projects_visible_playback_ratio() {
        let mut state = WaveformState::synthetic_for_tests();
        state.start_playback();
        state.set_playhead_ratio(0.25);

        let widget = WaveformWidget::new(
            state.file(),
            state.viewport(),
            state.cursor_ratio(),
            state.playhead_ratio(),
        );

        let overlays = widget.playhead_overlay();
        assert_eq!(overlays.len(), 1);
        match overlays[0] {
            radiant::runtime::GpuSurfaceOverlay::VerticalCursor {
                ratio,
                color,
                width,
            } => {
                assert!((ratio - 0.25).abs() < 0.001);
                assert_eq!((color.r, color.g, color.b), (71, 220, 255));
                assert_eq!(width, 1.75);
            }
        }
    }
}
