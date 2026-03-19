use super::normalize::clamp_sample;
use super::peaks;
use crate::audio::Source;
use crate::audio::decoder::SymphoniaDecoder;
use crate::waveform::{DecodedWaveform, WaveformDecodeError, WaveformPeaks, WaveformRenderer};
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
static SYMPHONIA_DECODE_COUNT: AtomicUsize = AtomicUsize::new(0);

impl WaveformRenderer {
    pub(super) fn load_decoded_via_symphonia(
        &self,
        bytes: &[u8],
        cache_token: u64,
        max_frames: usize,
    ) -> Result<DecodedWaveform, WaveformDecodeError> {
        #[cfg(test)]
        SYMPHONIA_DECODE_COUNT.fetch_add(1, Ordering::Relaxed);

        let owned: Arc<[u8]> = Arc::from(bytes.to_vec());
        let decoder =
            SymphoniaDecoder::from_bytes(owned).map_err(|error| WaveformDecodeError::Invalid {
                message: error.to_string(),
            })?;

        let sample_rate = decoder.sample_rate().max(1);
        let channels = decoder.channels().max(1);
        let duration_seconds = decoder
            .total_duration()
            .map(|duration| duration.as_secs_f32());
        let frames_estimate = duration_seconds
            .map(|secs| (secs * sample_rate as f32).round().max(0.0) as usize)
            .unwrap_or(0);

        if frames_estimate > max_frames {
            return self.build_symphonia_peaks(
                decoder,
                cache_token,
                sample_rate,
                channels,
                frames_estimate,
            );
        }

        let samples: Vec<f32> = decoder.collect();
        let frames = samples.len() / channels as usize;
        let duration_seconds = frames as f32 / sample_rate as f32;
        Ok(DecodedWaveform {
            cache_token,
            samples: Arc::from(samples),
            analysis_samples: Arc::from(Vec::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: None,
            duration_seconds,
            sample_rate,
            channels,
        })
    }

    fn build_symphonia_peaks<I>(
        &self,
        mut samples: I,
        cache_token: u64,
        sample_rate: u32,
        channels: u16,
        frames_estimate: usize,
    ) -> Result<DecodedWaveform, WaveformDecodeError>
    where
        I: Iterator<Item = f32>,
    {
        let channels_usize = channels as usize;
        let bucket_size_frames = peaks::peak_bucket_size(frames_estimate).max(1);
        let bucket_count_est = frames_estimate.div_ceil(bucket_size_frames).max(1);
        let analysis_stride = peaks::analysis_stride(sample_rate, frames_estimate);
        let mut analysis_samples =
            Vec::with_capacity(frames_estimate.div_ceil(analysis_stride).max(1));

        let mut mono = vec![(1.0_f32, -1.0_f32); bucket_count_est];
        let mut left = if channels_usize >= 2 {
            Some(vec![(1.0_f32, -1.0_f32); bucket_count_est])
        } else {
            None
        };
        let mut right = if channels_usize >= 2 {
            Some(vec![(1.0_f32, -1.0_f32); bucket_count_est])
        } else {
            None
        };

        let mut total_frames = 0usize;
        let mut analysis_sum = 0.0f32;
        let mut analysis_count = 0usize;
        loop {
            let bucket = total_frames / bucket_size_frames;
            if bucket >= mono.len() {
                mono.push((1.0, -1.0));
                if let Some(left_peaks) = left.as_mut() {
                    left_peaks.push((1.0, -1.0));
                }
                if let Some(right_peaks) = right.as_mut() {
                    right_peaks.push((1.0, -1.0));
                }
            }
            let mut frame_min = 1.0_f32;
            let mut frame_max = -1.0_f32;
            let mut frame_count = 0usize;
            let mut frame_sum = 0.0f32;
            for ch in 0..channels_usize {
                let Some(sample) = samples.next() else {
                    let duration_seconds = total_frames as f32 / sample_rate as f32;
                    let bucket_count = total_frames.div_ceil(bucket_size_frames).max(1);
                    mono.truncate(bucket_count);
                    if let Some(left_peaks) = left.as_mut() {
                        left_peaks.truncate(bucket_count);
                    }
                    if let Some(right_peaks) = right.as_mut() {
                        right_peaks.truncate(bucket_count);
                    }
                    if analysis_count > 0 {
                        analysis_samples.push(analysis_sum / analysis_count as f32);
                    }
                    let analysis_sample_rate = ((sample_rate as f32) / analysis_stride as f32)
                        .round()
                        .max(1.0) as u32;
                    return Ok(DecodedWaveform {
                        cache_token,
                        samples: Arc::from(Vec::new()),
                        analysis_samples: Arc::from(analysis_samples),
                        analysis_sample_rate,
                        analysis_stride,
                        peaks: Some(Arc::new(WaveformPeaks {
                            total_frames,
                            channels,
                            bucket_size_frames,
                            mono,
                            left,
                            right,
                        })),
                        duration_seconds,
                        sample_rate,
                        channels,
                    });
                };
                let sample = clamp_sample(sample);
                frame_min = frame_min.min(sample);
                frame_max = frame_max.max(sample);
                frame_count = frame_count.saturating_add(1);
                frame_sum += sample;
                if ch == 0 {
                    if let Some(left_peaks) = left.as_mut() {
                        let (min, max) = &mut left_peaks[bucket];
                        *min = (*min).min(sample);
                        *max = (*max).max(sample);
                    }
                } else if ch == 1
                    && let Some(right_peaks) = right.as_mut()
                {
                    let (min, max) = &mut right_peaks[bucket];
                    *min = (*min).min(sample);
                    *max = (*max).max(sample);
                }
            }
            let (min, max) = &mut mono[bucket];
            if frame_count == 0 {
                *min = (*min).min(0.0);
                *max = (*max).max(0.0);
            } else {
                *min = (*min).min(frame_min);
                *max = (*max).max(frame_max);
            }
            if frame_count > 0 {
                analysis_sum += frame_sum / frame_count as f32;
                analysis_count += 1;
                if analysis_count >= analysis_stride {
                    analysis_samples.push(analysis_sum / analysis_count as f32);
                    analysis_sum = 0.0;
                    analysis_count = 0;
                }
            }
            total_frames = total_frames.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::SampleFormat;

    fn wav_bytes_int(bits_per_sample: u16, channels: u16, samples: &[i32]) -> Vec<u8> {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 48_000,
            bits_per_sample,
            sample_format: SampleFormat::Int,
        };
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec).expect("create wav writer");
            for &sample in samples {
                writer.write_sample(sample).expect("write sample");
            }
            writer.finalize().expect("finalize wav");
        }
        cursor.into_inner()
    }

    fn corrupt_byte_rate(bytes: &mut [u8]) {
        let byte_rate_offset = 12 + 8 + 2 + 2 + 4;
        if bytes.len() >= byte_rate_offset + 4 {
            bytes[byte_rate_offset..byte_rate_offset + 4].copy_from_slice(&0u32.to_le_bytes());
        }
    }

    fn assert_slice_close(actual: &[f32], expected: &[f32], label: &str) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "{label} length mismatch: actual={} expected={}",
            actual.len(),
            expected.len()
        );
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() < 1e-6,
                "{label}[{index}] mismatch: actual={actual} expected={expected}"
            );
        }
    }

    fn assert_peak_pairs_close(actual: &[(f32, f32)], expected: &[(f32, f32)], label: &str) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "{label} peak bucket count mismatch: actual={} expected={}",
            actual.len(),
            expected.len()
        );
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual.0 - expected.0).abs() < 1e-6,
                "{label}[{index}].min mismatch: actual={} expected={}",
                actual.0,
                expected.0
            );
            assert!(
                (actual.1 - expected.1).abs() < 1e-6,
                "{label}[{index}].max mismatch: actual={} expected={}",
                actual.1,
                expected.1
            );
        }
    }

    fn assert_long_decode_parity(expected: &DecodedWaveform, actual: &DecodedWaveform) {
        assert!(expected.samples.is_empty(), "expected long-file decode");
        assert!(
            actual.samples.is_empty(),
            "expected long-file fallback decode"
        );
        assert_eq!(actual.channels, expected.channels);
        assert_eq!(actual.sample_rate, expected.sample_rate);
        assert_eq!(actual.frame_count(), expected.frame_count());
        assert_eq!(actual.analysis_stride, expected.analysis_stride);
        assert_eq!(actual.analysis_sample_rate, expected.analysis_sample_rate);
        assert!(
            (actual.duration_seconds - expected.duration_seconds).abs() < 1e-6,
            "duration mismatch: actual={} expected={}",
            actual.duration_seconds,
            expected.duration_seconds
        );
        assert_slice_close(
            actual.analysis_samples.as_ref(),
            expected.analysis_samples.as_ref(),
            "analysis_samples",
        );

        let actual_peaks = actual.peaks.as_ref().expect("actual peaks");
        let expected_peaks = expected.peaks.as_ref().expect("expected peaks");
        assert_eq!(actual_peaks.total_frames, expected_peaks.total_frames);
        assert_eq!(actual_peaks.channels, expected_peaks.channels);
        assert_eq!(
            actual_peaks.bucket_size_frames,
            expected_peaks.bucket_size_frames
        );
        assert_peak_pairs_close(
            actual_peaks.mono.as_slice(),
            expected_peaks.mono.as_slice(),
            "mono",
        );
        match (&actual_peaks.left, &expected_peaks.left) {
            (Some(actual), Some(expected)) => {
                assert_peak_pairs_close(actual.as_slice(), expected.as_slice(), "left");
            }
            (None, None) => {}
            _ => panic!("left peak presence mismatch"),
        }
        match (&actual_peaks.right, &expected_peaks.right) {
            (Some(actual), Some(expected)) => {
                assert_peak_pairs_close(actual.as_slice(), expected.as_slice(), "right");
            }
            (None, None) => {}
            _ => panic!("right peak presence mismatch"),
        }
    }

    #[test]
    fn symphonia_fallback_decodes_ill_formed_riff_size() {
        let renderer = WaveformRenderer::new(12, 12);
        let mut bytes = wav_bytes_int(16, 1, &[0, 1000, -1000, 0]);

        // Corrupt the redundant `nAvgBytesPerSec` field (byte rate) in the fmt chunk so that
        // `hound` rejects the file as ill-formed, while tolerant decoders still accept it.
        let byte_rate_offset = 12 + 8 + 2 + 2 + 4;
        if bytes.len() >= byte_rate_offset + 4 {
            bytes[byte_rate_offset..byte_rate_offset + 4].copy_from_slice(&0u32.to_le_bytes());
        }

        assert!(
            hound::WavReader::new(std::io::Cursor::new(bytes.as_slice())).is_err(),
            "expected hound to reject the file"
        );

        let decoded = renderer
            .decode_from_bytes(&bytes)
            .expect("symphonia fallback should decode");
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.sample_rate, 48_000);
        assert!(!decoded.samples.is_empty());
        assert!(decoded.duration_seconds > 0.0);
    }

    #[test]
    /// Forced long-file fallback should match the shared mono peak/analysis decode semantics.
    fn symphonia_long_file_fallback_matches_wav_mono_peak_and_analysis_output() {
        let renderer = WaveformRenderer::new(12, 12);
        let clean = wav_bytes_int(16, 1, &[0, 1000, -1000, 2000, -500, 750]);
        let expected = renderer
            .load_decoded_with_max_frames(&clean, 1)
            .expect("expected wav long-file decode");

        let mut corrupted = clean.clone();
        corrupt_byte_rate(&mut corrupted);
        assert!(
            hound::WavReader::new(std::io::Cursor::new(corrupted.as_slice())).is_err(),
            "expected hound to reject the corrupted wav"
        );

        let actual = renderer
            .load_decoded_with_max_frames(&corrupted, 1)
            .expect("expected symphonia long-file fallback decode");

        assert_long_decode_parity(&expected, &actual);
    }

    #[test]
    /// Forced long-file fallback should preserve stereo bucket sizing and channel-specific peaks.
    fn symphonia_long_file_fallback_matches_wav_stereo_peak_and_analysis_output() {
        let renderer = WaveformRenderer::new(12, 12);
        let clean = wav_bytes_int(
            16,
            2,
            &[0, 1000, -2000, 1500, 2000, -1000, -1000, 500, 750, -750],
        );
        let expected = renderer
            .load_decoded_with_max_frames(&clean, 1)
            .expect("expected wav long-file decode");

        let mut corrupted = clean.clone();
        corrupt_byte_rate(&mut corrupted);
        assert!(
            hound::WavReader::new(std::io::Cursor::new(corrupted.as_slice())).is_err(),
            "expected hound to reject the corrupted wav"
        );

        let actual = renderer
            .load_decoded_with_max_frames(&corrupted, 1)
            .expect("expected symphonia long-file fallback decode");

        assert_long_decode_parity(&expected, &actual);
    }
}
