use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::time::Duration;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;

use super::analysis_prep::{downmix_to_mono_into, prepare_mono_for_analysis_from_slice};
use super::resample::resample_linear_into;
use super::{ANALYSIS_SAMPLE_RATE, AnalysisAudio, MAX_ANALYSIS_SECONDS, WINDOW_SECONDS};

pub(crate) fn decode_for_analysis(path: &Path) -> Result<AnalysisAudio, String> {
    decode_for_analysis_with_rate(path, ANALYSIS_SAMPLE_RATE)
}

/// Audio metadata probed without running the full analysis decode path.
pub struct AudioProbe {
    /// Total decoded duration when the source reports one.
    pub duration_seconds: Option<f32>,
    /// Source sample rate when known.
    pub sample_rate: Option<u32>,
    #[cfg(test)]
    /// Source channel count when known.
    pub channels: Option<u16>,
}

/// Probe source metadata without running the full analysis decode path.
pub fn probe_metadata(path: &Path) -> Result<AudioProbe, String> {
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"))
    {
        let reader = hound::WavReader::open(path)
            .map_err(|err| format!("WAV probe failed for {}: {err}", path.display()))?;
        let spec = reader.spec();
        let sample_rate = spec.sample_rate.max(1);
        let channels = spec.channels.max(1);
        let duration_seconds =
            (reader.duration() as f32 / channels as f32 / sample_rate as f32).max(0.0);
        return Ok(AudioProbe {
            duration_seconds: Some(duration_seconds),
            sample_rate: Some(sample_rate),
            #[cfg(test)]
            channels: Some(channels),
        });
    }

    let file =
        File::open(path).map_err(|err| format!("Failed to open {}: {err}", path.display()))?;
    let hint = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut probe_hint = Hint::new();
    if let Some(hint) = hint.as_deref() {
        probe_hint.with_extension(hint);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &probe_hint,
            mss,
            &FormatOptions::default(),
            &Default::default(),
        )
        .map_err(|err| format!("Audio metadata probe failed for {}: {err}", path.display()))?;
    let track = probed.format.default_track().ok_or_else(|| {
        format!(
            "Audio metadata probe failed for {}: no default track",
            path.display()
        )
    })?;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44_100).max(1);
    let duration_seconds = track
        .codec_params
        .n_frames
        .map(|frames| Duration::from_secs_f64(frames as f64 / sample_rate as f64).as_secs_f32());
    Ok(AudioProbe {
        duration_seconds,
        sample_rate: Some(sample_rate),
        #[cfg(test)]
        channels: Some(
            track
                .codec_params
                .channels
                .map(|channels| channels.count() as u16)
                .unwrap_or(2)
                .max(1),
        ),
    })
}

/// Decode and prepare audio at the requested analysis sample rate.
pub fn decode_for_analysis_with_rate(
    path: &Path,
    sample_rate: u32,
) -> Result<AnalysisAudio, String> {
    decode_for_analysis_with_rate_limit(path, sample_rate, None)
}

/// Decode and prepare audio at the requested analysis sample rate with an optional duration cap.
pub fn decode_for_analysis_with_rate_limit(
    path: &Path,
    sample_rate: u32,
    max_seconds: Option<f32>,
) -> Result<AnalysisAudio, String> {
    let default_max = MAX_ANALYSIS_SECONDS + WINDOW_SECONDS;
    let max_decode_seconds = max_seconds
        .filter(|limit| limit.is_finite() && *limit > 0.0)
        .map(|limit| default_max.min(limit + WINDOW_SECONDS))
        .unwrap_or(default_max);
    let decoded = crate::analysis::audio_decode::decode_audio(path, Some(max_decode_seconds))?;
    DECODE_SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        downmix_to_mono_into(&mut scratch.mono, &decoded.samples, decoded.channels);
        let mono_len = scratch.mono.len();
        let mut resampled = Vec::new();
        resample_linear_into(
            &mut resampled,
            &scratch.mono[..mono_len],
            decoded.sample_rate,
            sample_rate,
        );
        Ok(prepare_mono_for_analysis_from_slice(
            &resampled,
            sample_rate,
        ))
    })
}

struct DecodeScratch {
    mono: Vec<f32>,
}

thread_local! {
    static DECODE_SCRATCH: RefCell<DecodeScratch> = const { RefCell::new(DecodeScratch {
        mono: Vec::new(),
    }) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::audio::ANALYSIS_SAMPLE_RATE;
    use hound::{SampleFormat, WavSpec, WavWriter};
    use tempfile::TempDir;

    #[test]
    fn wav_probe_reads_duration_without_full_decode() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("probe.wav");
        let spec = WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&path, spec).unwrap();
        for _ in 0..48_000 {
            writer.write_sample::<i16>(0).unwrap();
        }
        writer.finalize().unwrap();
        let probe = probe_metadata(&path).unwrap();
        let duration = probe.duration_seconds.unwrap();
        assert!((duration - 1.0).abs() < 1e-3);
        assert_eq!(probe.sample_rate, Some(48_000));
        assert_eq!(probe.channels, Some(1));
    }

    #[test]
    fn decode_for_analysis_decodes_wav_to_target_rate() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("fixture.wav");
        let spec = WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut writer = WavWriter::create(&path, spec).unwrap();
        for _ in 0..(44_100 / 10) {
            writer.write_sample::<f32>(0.25).unwrap();
            writer.write_sample::<f32>(0.25).unwrap();
        }
        writer.finalize().unwrap();

        let decoded = decode_for_analysis(&path).unwrap();
        assert_eq!(decoded.sample_rate_used, ANALYSIS_SAMPLE_RATE);
        assert!((decoded.duration_seconds - 0.1).abs() < 0.02);
        let peak = decoded
            .mono
            .iter()
            .copied()
            .map(|v| v.abs())
            .fold(0.0, f32::max);
        assert!((peak - 1.0).abs() < 1e-6);
    }

    #[test]
    fn decode_for_analysis_trims_leading_and_trailing_silence() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("trim.wav");
        let sample_rate = ANALYSIS_SAMPLE_RATE;
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut writer = WavWriter::create(&path, spec).unwrap();
        let silence_frames = (0.1 * sample_rate as f32).round() as usize;
        let tone_frames = (0.1 * sample_rate as f32).round() as usize;
        let tail_silence_frames = (0.2 * sample_rate as f32).round() as usize;
        for _ in 0..silence_frames {
            writer.write_sample::<f32>(0.0).unwrap();
        }
        for _ in 0..tone_frames {
            writer.write_sample::<f32>(0.25).unwrap();
        }
        for _ in 0..tail_silence_frames {
            writer.write_sample::<f32>(0.0).unwrap();
        }
        writer.finalize().unwrap();

        let decoded = decode_for_analysis(&path).unwrap();
        assert!(decoded.duration_seconds < 0.25);
        assert!(decoded.duration_seconds > 0.08);
    }

    #[test]
    fn quiet_samples_are_not_trimmed_to_empty() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("quiet.wav");
        let sample_rate = ANALYSIS_SAMPLE_RATE;
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut writer = WavWriter::create(&path, spec).unwrap();
        let frames = (0.1 * sample_rate as f32).round() as usize;
        for _ in 0..frames {
            writer.write_sample::<f32>(0.001).unwrap();
        }
        writer.finalize().unwrap();

        let decoded = decode_for_analysis(&path).unwrap();
        assert!(!decoded.mono.is_empty());
        let peak = decoded
            .mono
            .iter()
            .copied()
            .map(|v| v.abs())
            .fold(0.0, f32::max);
        assert!(peak > 0.5);
    }
}
