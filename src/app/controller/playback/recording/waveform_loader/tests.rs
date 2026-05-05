use super::*;
use crate::waveform::WaveformRenderer;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::io::{Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::NamedTempFile;

static RECORDING_STATE_LOCK: Mutex<()> = Mutex::new(());

fn build_minimal_wav(sample: f32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let data_bytes = sample.to_le_bytes();
    let chunk_size = 4u32 + 8u32 + data_bytes.len() as u32;
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&chunk_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_bytes.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&data_bytes);
    bytes
}

fn build_wav_bytes(data_bytes: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let chunk_size = 4u32 + 8u32 + data_bytes.len() as u32;
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&chunk_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_bytes.len() as u32).to_le_bytes());
    bytes.extend_from_slice(data_bytes);
    bytes
}

fn build_wav_samples(samples: &[f32]) -> Vec<u8> {
    let mut data = Vec::with_capacity(samples.len() * 4);
    for sample in samples {
        data.extend_from_slice(&sample.to_le_bytes());
    }
    build_wav_bytes(&data)
}

fn generated_sample(frame: usize, channel: usize) -> f32 {
    let lane_scale = if channel == 0 { 0.82 } else { -0.67 };
    let base = (((frame * (channel + 3)) % 257) as f32 / 96.0) - 1.2;
    let accent = if frame % 4096 == (channel * 137) {
        1.25
    } else if frame % 4096 == (2048 + channel * 29) {
        -1.25
    } else {
        0.0
    };
    (base * lane_scale) + accent
}

fn build_long_recording_fixture(frame_count: usize, channels: u16, sample_rate: u32) -> Vec<u8> {
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let temp = NamedTempFile::new().expect("tempfile");
    {
        let file = temp.reopen().expect("reopen wav");
        let mut writer = WavWriter::new(file, spec).expect("wav writer");
        for frame in 0..frame_count {
            for channel in 0..channels as usize {
                writer
                    .write_sample(generated_sample(frame, channel))
                    .expect("write sample");
            }
        }
        writer.finalize().expect("finalize wav");
    }
    std::fs::read(temp.path()).expect("read wav bytes")
}

fn recording_waveform_from_wav_bytes(
    wav_bytes: &[u8],
    sample_rate: u32,
    channels: u16,
) -> DecodedWaveform {
    let data_offset = find_wav_data_chunk(wav_bytes).expect("data chunk");
    let total_frames =
        total_frames_for_data(wav_bytes.len().saturating_sub(data_offset) as u64, channels);
    let mut state = RecordingWaveformState::new(sample_rate, channels, data_offset);
    state.prepare_for_total_frames(total_frames);
    let consumed = state.consume_data_bytes(&wav_bytes[data_offset..]);
    assert_eq!(consumed, total_frames);
    if matches!(state.mode, RecordingWaveformMode::Full { .. })
        && state.total_frames > RECORDING_MAX_FULL_FRAMES
    {
        state.convert_full_to_peaks();
    }
    state.to_decoded()
}

fn decode_reference_waveform(wav_bytes: &[u8]) -> DecodedWaveform {
    WaveformRenderer::new(8, 8)
        .decode_from_bytes(wav_bytes)
        .expect("reference decode")
}

fn assert_slice_approx_eq(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(
            (actual - expected).abs() < 1e-6,
            "sample mismatch at {index}: {actual} vs {expected}"
        );
    }
}

fn assert_peak_pairs_approx_eq(actual: &[(f32, f32)], expected: &[(f32, f32)]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(
            (actual.0 - expected.0).abs() < 1e-6,
            "min peak mismatch at {index}: {} vs {}",
            actual.0,
            expected.0
        );
        assert!(
            (actual.1 - expected.1).abs() < 1e-6,
            "max peak mismatch at {index}: {} vs {}",
            actual.1,
            expected.1
        );
    }
}

fn assert_decoded_peak_parity(actual: &DecodedWaveform, expected: &DecodedWaveform) {
    assert_eq!(actual.frame_count(), expected.frame_count());
    assert_eq!(actual.sample_rate, expected.sample_rate);
    assert_eq!(actual.channels, expected.channels);
    assert_eq!(actual.analysis_stride, expected.analysis_stride);
    assert_eq!(actual.analysis_sample_rate, expected.analysis_sample_rate);
    assert!(
        (actual.duration_seconds - expected.duration_seconds).abs() < 1e-6,
        "duration mismatch: {} vs {}",
        actual.duration_seconds,
        expected.duration_seconds
    );
    assert_slice_approx_eq(&actual.samples, &expected.samples);
    assert_slice_approx_eq(&actual.analysis_samples, &expected.analysis_samples);

    let actual_peaks = actual.peaks.as_deref().expect("recording peaks");
    let expected_peaks = expected.peaks.as_deref().expect("reference peaks");
    assert_eq!(actual_peaks.total_frames, expected_peaks.total_frames);
    assert_eq!(actual_peaks.channels, expected_peaks.channels);
    assert_eq!(
        actual_peaks.bucket_size_frames,
        expected_peaks.bucket_size_frames
    );
    assert_peak_pairs_approx_eq(&actual_peaks.mono, &expected_peaks.mono);
    match (&actual_peaks.left, &expected_peaks.left) {
        (Some(actual), Some(expected)) => assert_peak_pairs_approx_eq(actual, expected),
        (None, None) => {}
        _ => panic!("left-channel peak parity mismatch"),
    }
    match (&actual_peaks.right, &expected_peaks.right) {
        (Some(actual), Some(expected)) => assert_peak_pairs_approx_eq(actual, expected),
        (None, None) => {}
        _ => panic!("right-channel peak parity mismatch"),
    }
}

mod incremental_loading;
mod queue;
mod waveform_parity;
