//! Incremental waveform aggregation state and retained recording decode buffers.

use super::*;
use crate::waveform::peak_analysis::{PeakAnalysisAccumulator, PeakAnalysisLayout, clamp_sample};

#[derive(Clone, Hash, PartialEq, Eq)]
pub(super) struct RecordingWaveformKey {
    pub(super) source_id: SourceId,
    pub(super) relative_path: PathBuf,
}

pub(super) enum RecordingWaveformMode {
    Full {
        samples: Vec<f32>,
    },
    Peaks {
        peak_analysis: PeakAnalysisAccumulator,
    },
}

pub(super) struct RecordingWaveformState {
    pub(super) data_offset: usize,
    pub(super) bytes_read: u64,
    pub(super) tail: Vec<u8>,
    pub(super) total_frames: usize,
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
    pub(super) mode: RecordingWaveformMode,
}

impl RecordingWaveformState {
    pub(super) fn new(sample_rate: u32, channels: u16, data_offset: usize) -> Self {
        Self {
            data_offset,
            bytes_read: 0,
            tail: Vec::new(),
            total_frames: 0,
            sample_rate,
            channels: channels.max(1),
            mode: RecordingWaveformMode::Full {
                samples: Vec::new(),
            },
        }
    }

    pub(super) fn prepare_for_total_frames(&mut self, total_frames: usize) {
        if total_frames <= RECORDING_MAX_FULL_FRAMES {
            let total_samples = total_frames.saturating_mul(self.channels as usize);
            self.mode = RecordingWaveformMode::Full {
                samples: Vec::with_capacity(total_samples),
            };
            return;
        }
        self.mode = RecordingWaveformMode::Peaks {
            peak_analysis: PeakAnalysisAccumulator::new(
                self.sample_rate,
                self.channels,
                total_frames,
            ),
        };
    }

    pub(super) fn requires_rebuild(&self, total_frames: usize) -> bool {
        let RecordingWaveformMode::Peaks { peak_analysis } = &self.mode else {
            return false;
        };
        peak_analysis.layout() != PeakAnalysisLayout::for_frames(self.sample_rate, total_frames)
    }

    pub(super) fn convert_full_to_peaks(&mut self) {
        let RecordingWaveformMode::Full { samples } = &self.mode else {
            return;
        };
        let channels = self.channels as usize;
        let mut peak_analysis =
            PeakAnalysisAccumulator::new(self.sample_rate, self.channels, self.total_frames);
        for frame in 0..self.total_frames {
            let frame_start = frame.saturating_mul(channels);
            let mut frame_min = 1.0_f32;
            let mut frame_max = -1.0_f32;
            let mut frame_sum = 0.0_f32;
            let mut left_sample = None;
            let mut right_sample = None;
            for ch in 0..channels {
                if let Some(sample) = samples.get(frame_start + ch) {
                    let sample = clamp_sample(*sample);
                    frame_min = frame_min.min(sample);
                    frame_max = frame_max.max(sample);
                    frame_sum += sample;
                    if ch == 0 {
                        left_sample = Some(sample);
                    } else if ch == 1 {
                        right_sample = Some(sample);
                    }
                }
            }
            let frame_avg = if channels > 0 {
                frame_sum / channels as f32
            } else {
                0.0
            };
            peak_analysis.push_frame(frame_min, frame_max, frame_avg, left_sample, right_sample);
        }
        self.mode = RecordingWaveformMode::Peaks { peak_analysis };
    }

    pub(super) fn consume_data_bytes(&mut self, bytes: &[u8]) -> usize {
        let frame_bytes = 4usize * self.channels as usize;
        if frame_bytes == 0 {
            return 0;
        }
        let frames_before = self.total_frames;
        self.tail.extend_from_slice(bytes);
        let usable = (self.tail.len() / frame_bytes) * frame_bytes;
        let mut offset = 0usize;
        while offset < usable {
            self.total_frames = self.total_frames.saturating_add(1);
            let mut frame_min = 1.0_f32;
            let mut frame_max = -1.0_f32;
            let mut frame_sum = 0.0f32;
            let mut left_sample = None;
            let mut right_sample = None;
            for ch in 0..self.channels as usize {
                let sample = f32::from_le_bytes(
                    self.tail[offset..offset + 4].try_into().unwrap_or_default(),
                );
                let sample = clamp_sample(sample);
                frame_min = frame_min.min(sample);
                frame_max = frame_max.max(sample);
                frame_sum += sample;
                if ch == 0 {
                    left_sample = Some(sample);
                } else if ch == 1 {
                    right_sample = Some(sample);
                }
                if let RecordingWaveformMode::Full { samples } = &mut self.mode {
                    samples.push(sample);
                }
                offset += 4;
            }
            if let RecordingWaveformMode::Peaks { peak_analysis } = &mut self.mode {
                peak_analysis.push_frame(
                    frame_min,
                    frame_max,
                    frame_sum / self.channels.max(1) as f32,
                    left_sample,
                    right_sample,
                );
            }
        }
        self.bytes_read = self.bytes_read.saturating_add(bytes.len() as u64);
        self.tail.drain(..usable);
        self.total_frames.saturating_sub(frames_before)
    }

    pub(super) fn to_decoded(&self) -> DecodedWaveform {
        let duration_seconds = self.total_frames as f32 / self.sample_rate.max(1) as f32;
        match &self.mode {
            RecordingWaveformMode::Full { samples } => DecodedWaveform {
                cache_token: next_recording_cache_token(),
                samples: Arc::from(samples.clone()),
                analysis_samples: Arc::from(Vec::new()),
                analysis_sample_rate: 0,
                analysis_stride: 1,
                peaks: None,
                duration_seconds,
                sample_rate: self.sample_rate,
                channels: self.channels,
            },
            RecordingWaveformMode::Peaks { peak_analysis } => {
                let output = peak_analysis.output();
                debug_assert_eq!(peak_analysis.total_frames(), self.total_frames);
                DecodedWaveform {
                    cache_token: next_recording_cache_token(),
                    samples: Arc::from(Vec::new()),
                    analysis_samples: Arc::from(output.analysis_samples),
                    analysis_sample_rate: output.analysis_sample_rate,
                    analysis_stride: output.analysis_stride,
                    peaks: Some(Arc::new(output.peaks)),
                    duration_seconds,
                    sample_rate: self.sample_rate,
                    channels: self.channels,
                }
            }
        }
    }
}

pub(super) fn recording_state_map()
-> &'static Mutex<HashMap<RecordingWaveformKey, RecordingWaveformState>> {
    static RECORDING_STATE: OnceLock<Mutex<HashMap<RecordingWaveformKey, RecordingWaveformState>>> =
        OnceLock::new();
    RECORDING_STATE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn empty_recording_waveform(sample_rate: u32, channels: u16) -> DecodedWaveform {
    DecodedWaveform {
        cache_token: next_recording_cache_token(),
        samples: Arc::from(Vec::new()),
        analysis_samples: Arc::from(Vec::new()),
        analysis_sample_rate: 0,
        analysis_stride: 1,
        peaks: None,
        duration_seconds: 0.0,
        sample_rate,
        channels,
    }
}

pub(super) fn next_recording_cache_token() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_CACHE_TOKEN: AtomicU64 = AtomicU64::new(1);
    NEXT_CACHE_TOKEN.fetch_add(1, Ordering::Relaxed)
}
