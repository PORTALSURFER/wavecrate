use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use crate::Source;
use crate::decoder::SymphoniaDecoder;

pub(crate) fn decoder_from_bytes(bytes: Arc<[u8]>) -> Result<SymphoniaDecoder, String> {
    SymphoniaDecoder::from_bytes(bytes)
}

pub(crate) fn decoder_from_path(path: &Path) -> Result<SymphoniaDecoder, String> {
    SymphoniaDecoder::from_path(path)
}

pub(crate) fn decoder_duration(bytes: &Arc<[u8]>) -> Option<f32> {
    decoder_from_bytes(bytes.clone())
        .ok()
        .and_then(|decoder| decoder.total_duration())
        .map(|duration| duration.as_secs_f32())
}

pub(crate) fn decoder_sample_rate(bytes: &Arc<[u8]>) -> Option<u32> {
    decoder_from_bytes(bytes.clone())
        .ok()
        .map(|decoder| decoder.sample_rate().max(1))
}

pub(crate) fn wav_header_duration(bytes: &Arc<[u8]>) -> Option<f32> {
    wav_spec_from_bytes(bytes).map(|(duration, _, _)| duration)
}

pub(crate) fn wav_spec_from_bytes(bytes: &Arc<[u8]>) -> Option<(f32, u32, u16)> {
    let reader = hound::WavReader::new(Cursor::new(bytes.clone())).ok()?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate as f32;
    if sample_rate <= 0.0 {
        return None;
    }
    let duration = reader.duration() as f32 / sample_rate;
    Some((duration, spec.sample_rate, spec.channels.max(1)))
}

pub(crate) fn map_seek_error(error: String) -> String {
    format!("Audio seek failed: {error}")
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, sync::Arc};

    use super::wav_spec_from_bytes;

    #[test]
    fn wav_spec_duration_is_frame_based_for_stereo_files() {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec).expect("writer");
            for _ in 0..48_000 {
                writer.write_sample(0_i16).expect("left sample");
                writer.write_sample(0_i16).expect("right sample");
            }
            writer.finalize().expect("finalize wav");
        }
        let bytes = Arc::<[u8]>::from(cursor.into_inner());

        let (duration, sample_rate, channels) = wav_spec_from_bytes(&bytes).expect("wav spec");

        assert_eq!(sample_rate, 48_000);
        assert_eq!(channels, 2);
        assert!((duration - 1.0).abs() < 0.000_001);
    }
}
