use std::path::Path;
use std::{fmt, fs::File, io};

use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error, formats::FormatOptions,
    io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

/// Raw decoded audio in interleaved `f32` samples.
pub(crate) struct DecodedAudio {
    pub(crate) samples: Vec<f32>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
}

/// Structured reason why analysis decoding could not produce audio.
#[derive(Debug)]
pub enum AnalysisDecodeError {
    /// The source format or codec has no available decoder.
    Unsupported(String),
    /// The source is malformed, truncated, or has no decodable audio frames.
    Corrupt(String),
    /// The source could not be read from the filesystem.
    Io {
        /// Kind reported by the filesystem operation.
        kind: io::ErrorKind,
        /// Original diagnostic retained for callers that need logs.
        detail: String,
    },
    /// Required stream metadata was absent or invalid.
    Invalid(String),
}

impl fmt::Display for AnalysisDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(detail) | Self::Corrupt(detail) | Self::Invalid(detail) => {
                formatter.write_str(detail)
            }
            Self::Io { detail, .. } => formatter.write_str(detail),
        }
    }
}

impl std::error::Error for AnalysisDecodeError {}

/// Decode audio while retaining the owning decoder failure class.
pub fn decode_audio_typed(
    path: &Path,
    max_seconds: Option<f32>,
) -> Result<DecodedAudio, AnalysisDecodeError> {
    match decode_with_symphonia(path, max_seconds) {
        Ok((samples, sample_rate, channels)) => Ok(DecodedAudio {
            samples,
            sample_rate: sample_rate.max(1),
            channels: channels.max(1),
        }),
        Err(error) => Err(error),
    }
}

fn decode_with_symphonia(
    path: &Path,
    max_seconds: Option<f32>,
) -> Result<(Vec<f32>, u32, u16), AnalysisDecodeError> {
    let file = File::open(path).map_err(|error| AnalysisDecodeError::Io {
        kind: error.kind(),
        detail: format!("Open {}: {error}", path.display()),
    })?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|error| match error {
            Error::Unsupported(_) => AnalysisDecodeError::Unsupported(format!(
                "Symphonia probe failed for {}: {error}",
                path.display()
            )),
            _ => AnalysisDecodeError::Corrupt(format!(
                "Symphonia probe failed for {}: {error}",
                path.display()
            )),
        })?;
    let mut format = probed.format;
    let track = format.default_track().ok_or_else(|| {
        AnalysisDecodeError::Invalid(format!("No default track for {}", path.display()))
    })?;
    let codec_params = &track.codec_params;
    let sample_rate = codec_params.sample_rate.ok_or_else(|| {
        AnalysisDecodeError::Invalid(format!("Missing sample rate for {}", path.display()))
    })?;
    let channels = codec_params
        .channels
        .ok_or_else(|| {
            AnalysisDecodeError::Invalid(format!("Missing channel count for {}", path.display()))
        })?
        .count() as u16;
    let max_samples = max_seconds.filter(|limit| *limit > 0.0).map(|limit| {
        let frames = (limit * sample_rate as f32).ceil().max(1.0);
        (frames as usize).saturating_mul(channels as usize).max(1)
    });

    let mut decoder = symphonia::default::get_codecs()
        .make(codec_params, &DecoderOptions::default())
        .map_err(|error| match error {
            Error::Unsupported(_) => AnalysisDecodeError::Unsupported(format!(
                "Symphonia decoder failed for {}: {error}",
                path.display()
            )),
            _ => AnalysisDecodeError::Corrupt(format!(
                "Symphonia decoder failed for {}: {error}",
                path.display()
            )),
        })?;

    let mut samples = Vec::new();
    loop {
        if max_samples.is_some_and(|limit| samples.len() >= limit) {
            break;
        }
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(_)) => break,
            Err(err) => {
                return Err(AnalysisDecodeError::Corrupt(format!(
                    "Symphonia packet read failed for {}: {err}",
                    path.display()
                )));
            }
        };
        let audio_buf = match decoder.decode(&packet) {
            Ok(audio_buf) => audio_buf,
            Err(Error::DecodeError(_)) => continue,
            Err(err) => {
                return Err(AnalysisDecodeError::Corrupt(format!(
                    "Symphonia decode failed for {}: {err}",
                    path.display()
                )));
            }
        };
        let spec = *audio_buf.spec();
        let mut sample_buf = SampleBuffer::<f32>::new(audio_buf.capacity() as u64, spec);
        sample_buf.copy_interleaved_ref(audio_buf);
        samples.extend_from_slice(sample_buf.samples());
        if let Some(limit) = max_samples
            && samples.len() >= limit
        {
            samples.truncate(limit);
            break;
        }
    }

    if samples.is_empty() {
        return Err(AnalysisDecodeError::Corrupt(format!(
            "Symphonia decoded 0 samples for {}",
            path.display()
        )));
    }

    Ok((samples, sample_rate, channels))
}
