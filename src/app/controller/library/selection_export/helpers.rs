use super::*;
use crate::app::controller::playback::audio_samples::{crop_samples, decode_samples_from_bytes};

/// Decode the loaded audio and crop it to the requested normalized selection bounds.
pub(super) fn crop_selection_samples(
    audio: &LoadedAudio,
    bounds: SelectionRange,
) -> Result<(Vec<f32>, hound::WavSpec), String> {
    let decoded = decode_samples_from_bytes(&audio.bytes)?;
    let cropped = crop_samples(&decoded.samples, decoded.channels, bounds)?;
    let spec = hound::WavSpec {
        channels: decoded.channels.max(1),
        sample_rate: decoded.sample_rate.max(1),
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    Ok((cropped, spec))
}

/// Build the lightweight content-hash placeholder used before background analysis runs.
pub(super) fn fast_content_hash(file_size: u64, modified_ns: i64) -> String {
    format!("fast-{}-{}", file_size, modified_ns)
}

impl AppController {
    /// Remove legacy or current selection suffixes before numbering a new export.
    pub(super) fn strip_selection_suffix(stem: &str) -> &str {
        if let Some(prefix) = Self::strip_indexed_selection_suffix(stem, "_selection_") {
            return prefix;
        }
        if let Some(prefix) = Self::strip_indexed_selection_suffix(stem, "_sel_") {
            return prefix;
        }
        if let Some(prefix) = stem.strip_suffix("_selection")
            && !prefix.is_empty()
        {
            return prefix;
        }
        if let Some(prefix) = stem.strip_suffix("_sel")
            && !prefix.is_empty()
        {
            return prefix;
        }
        stem
    }

    /// Strip one numbered selection suffix when the stem already ends with it.
    fn strip_indexed_selection_suffix<'a>(stem: &'a str, marker: &str) -> Option<&'a str> {
        let (prefix, suffix) = stem.rsplit_once(marker)?;
        if prefix.is_empty() || suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(prefix)
    }
}
