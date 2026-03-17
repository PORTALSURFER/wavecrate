//! Incremental waveform aggregation state and peak/analysis helper math.

use super::*;

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
        bucket_size_frames: usize,
        mono: Vec<(f32, f32)>,
        left: Option<Vec<(f32, f32)>>,
        right: Option<Vec<(f32, f32)>>,
    },
}

pub(super) struct RecordingWaveformState {
    pub(super) data_offset: usize,
    pub(super) bytes_read: u64,
    pub(super) tail: Vec<u8>,
    pub(super) total_frames: usize,
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
    pub(super) analysis_stride: usize,
    pub(super) analysis_sum: f32,
    pub(super) analysis_count: usize,
    pub(super) analysis_samples: Vec<f32>,
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
            analysis_stride: 1,
            analysis_sum: 0.0,
            analysis_count: 0,
            analysis_samples: Vec::new(),
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
        self.analysis_stride = analysis_stride(self.sample_rate, total_frames);
        self.analysis_samples =
            Vec::with_capacity(total_frames.div_ceil(self.analysis_stride).max(1));
        let bucket_size_frames = peak_bucket_size(total_frames);
        let bucket_count = total_frames.div_ceil(bucket_size_frames).max(1);
        let mono = vec![(1.0_f32, -1.0_f32); bucket_count];
        let left = if self.channels >= 2 {
            Some(vec![(1.0_f32, -1.0_f32); bucket_count])
        } else {
            None
        };
        let right = if self.channels >= 2 {
            Some(vec![(1.0_f32, -1.0_f32); bucket_count])
        } else {
            None
        };
        self.mode = RecordingWaveformMode::Peaks {
            bucket_size_frames,
            mono,
            left,
            right,
        };
    }

    pub(super) fn requires_rebuild(&self, total_frames: usize) -> bool {
        if let RecordingWaveformMode::Peaks {
            bucket_size_frames, ..
        } = self.mode
        {
            let next_bucket = peak_bucket_size(total_frames);
            let next_stride = analysis_stride(self.sample_rate, total_frames);
            return next_bucket != bucket_size_frames || next_stride != self.analysis_stride;
        }
        false
    }

    pub(super) fn convert_full_to_peaks(&mut self) {
        let RecordingWaveformMode::Full { samples } = &self.mode else {
            return;
        };
        let total_frames = self.total_frames;
        self.analysis_stride = analysis_stride(self.sample_rate, total_frames);
        self.analysis_samples =
            Vec::with_capacity(total_frames.div_ceil(self.analysis_stride).max(1));
        let bucket_size_frames = peak_bucket_size(total_frames);
        let bucket_count = total_frames.div_ceil(bucket_size_frames).max(1);
        let mut mono = vec![(1.0_f32, -1.0_f32); bucket_count];
        let mut left = if self.channels >= 2 {
            Some(vec![(1.0_f32, -1.0_f32); bucket_count])
        } else {
            None
        };
        let mut right = if self.channels >= 2 {
            Some(vec![(1.0_f32, -1.0_f32); bucket_count])
        } else {
            None
        };
        let channels = self.channels as usize;
        let mut analysis_sum = 0.0f32;
        let mut analysis_count = 0usize;
        for frame in 0..total_frames {
            let frame_start = frame.saturating_mul(channels);
            let mut frame_min = 1.0_f32;
            let mut frame_max = -1.0_f32;
            let mut frame_sum = 0.0_f32;
            for ch in 0..channels {
                if let Some(sample) = samples.get(frame_start + ch) {
                    let sample = clamp_sample(*sample);
                    frame_sum += sample;
                    frame_min = frame_min.min(sample);
                    frame_max = frame_max.max(sample);
                    if ch == 0 {
                        if let Some(left_peaks) = left.as_mut() {
                            let bucket = frame / bucket_size_frames;
                            let (min, max) = &mut left_peaks[bucket];
                            *min = (*min).min(sample);
                            *max = (*max).max(sample);
                        }
                    } else if ch == 1
                        && let Some(right_peaks) = right.as_mut()
                    {
                        let bucket = frame / bucket_size_frames;
                        let (min, max) = &mut right_peaks[bucket];
                        *min = (*min).min(sample);
                        *max = (*max).max(sample);
                    }
                }
            }
            let frame_avg = if channels > 0 {
                frame_sum / channels as f32
            } else {
                0.0
            };
            let bucket = frame / bucket_size_frames;
            let (min, max) = &mut mono[bucket];
            *min = (*min).min(frame_min);
            *max = (*max).max(frame_max);
            analysis_sum += frame_avg;
            analysis_count += 1;
            if analysis_count >= self.analysis_stride {
                self.analysis_samples
                    .push(analysis_sum / analysis_count as f32);
                analysis_sum = 0.0;
                analysis_count = 0;
            }
        }
        if analysis_count > 0 {
            self.analysis_samples
                .push(analysis_sum / analysis_count as f32);
        }
        self.mode = RecordingWaveformMode::Peaks {
            bucket_size_frames,
            mono,
            left,
            right,
        };
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
            let frame_index = self.total_frames;
            self.total_frames = self.total_frames.saturating_add(1);
            let mut frame_min = 1.0_f32;
            let mut frame_max = -1.0_f32;
            let mut frame_sum = 0.0f32;
            for ch in 0..self.channels as usize {
                let sample = f32::from_le_bytes(
                    self.tail[offset..offset + 4].try_into().unwrap_or_default(),
                );
                let sample = clamp_sample(sample);
                frame_min = frame_min.min(sample);
                frame_max = frame_max.max(sample);
                if let RecordingWaveformMode::Full { samples } = &mut self.mode {
                    samples.push(sample);
                } else if let RecordingWaveformMode::Peaks {
                    bucket_size_frames,
                    left,
                    right,
                    ..
                } = &mut self.mode
                {
                    let bucket_size = *bucket_size_frames;
                    let bucket = frame_index / bucket_size;
                    if ch == 0 {
                        if let Some(left_peaks) = left.as_mut() {
                            if bucket >= left_peaks.len() {
                                left_peaks.resize(bucket + 1, (1.0, -1.0));
                            }
                            let (min, max) = &mut left_peaks[bucket];
                            *min = (*min).min(sample);
                            *max = (*max).max(sample);
                        }
                    } else if ch == 1
                        && let Some(right_peaks) = right.as_mut()
                    {
                        if bucket >= right_peaks.len() {
                            right_peaks.resize(bucket + 1, (1.0, -1.0));
                        }
                        let (min, max) = &mut right_peaks[bucket];
                        *min = (*min).min(sample);
                        *max = (*max).max(sample);
                    }
                }
                frame_sum += sample;
                offset += 4;
            }
            let frame_avg = frame_sum / self.channels.max(1) as f32;
            if let RecordingWaveformMode::Peaks {
                bucket_size_frames,
                mono,
                ..
            } = &mut self.mode
            {
                let bucket_size = *bucket_size_frames;
                let bucket = frame_index / bucket_size;
                if bucket >= mono.len() {
                    mono.resize(bucket + 1, (1.0, -1.0));
                }
                let (min, max) = &mut mono[bucket];
                *min = (*min).min(frame_min);
                *max = (*max).max(frame_max);
                self.analysis_sum += frame_avg;
                self.analysis_count += 1;
                if self.analysis_count >= self.analysis_stride {
                    self.analysis_samples
                        .push(self.analysis_sum / self.analysis_count as f32);
                    self.analysis_sum = 0.0;
                    self.analysis_count = 0;
                }
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
            RecordingWaveformMode::Peaks {
                bucket_size_frames,
                mono,
                left,
                right,
            } => {
                let analysis_sample_rate = ((self.sample_rate as f32) / self.analysis_stride as f32)
                    .round()
                    .max(1.0) as u32;
                let mut analysis_samples = self.analysis_samples.clone();
                if self.analysis_count > 0 {
                    analysis_samples.push(self.analysis_sum / self.analysis_count as f32);
                }
                DecodedWaveform {
                    cache_token: next_recording_cache_token(),
                    samples: Arc::from(Vec::new()),
                    analysis_samples: Arc::from(analysis_samples),
                    analysis_sample_rate,
                    analysis_stride: self.analysis_stride.max(1),
                    peaks: Some(Arc::new(WaveformPeaks {
                        total_frames: self.total_frames,
                        channels: self.channels,
                        bucket_size_frames: *bucket_size_frames,
                        mono: mono.clone(),
                        left: left.clone(),
                        right: right.clone(),
                    })),
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

pub(super) fn peak_bucket_size(frames: usize) -> usize {
    frames.div_ceil(RECORDING_MAX_PEAK_BUCKETS).max(1)
}

pub(super) fn analysis_stride(sample_rate: u32, total_frames: usize) -> usize {
    const MIN_ANALYSIS_SAMPLE_RATE: u32 = 8_000;
    const MAX_ANALYSIS_SAMPLES: usize = 5_000_000;

    let sample_rate = sample_rate.max(1);
    let min_stride = (sample_rate / MIN_ANALYSIS_SAMPLE_RATE).max(1) as usize;
    let max_samples_stride = total_frames.div_ceil(MAX_ANALYSIS_SAMPLES).max(1);
    min_stride.max(max_samples_stride).max(1)
}

pub(super) fn clamp_sample(sample: f32) -> f32 {
    sample.clamp(-1.0, 1.0)
}

pub(super) fn next_recording_cache_token() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_CACHE_TOKEN: AtomicU64 = AtomicU64::new(1);
    NEXT_CACHE_TOKEN.fetch_add(1, Ordering::Relaxed)
}
