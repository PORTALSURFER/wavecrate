use std::{
    fs::File,
    io::{BufWriter, ErrorKind, Read, Seek, Write},
    path::{Path, PathBuf},
};
use wavecrate::audio::{
    DEFAULT_SHORT_EDGE_FADE, short_edge_fade_frame_count, short_edge_fade_gain,
};

pub(in super::super) fn write_extraction_atomically(
    source_path: &Path,
    target_folder: &Path,
    write: impl FnOnce(&mut File) -> Result<(), String>,
) -> Result<PathBuf, String> {
    // Same-directory staging keeps publication on one filesystem. Dropping this owned file
    // removes every pre-publication failure without touching any user-visible destination.
    let mut staged = tempfile::Builder::new()
        .prefix(".wavecrate-extraction-")
        .suffix(".wav.tmp")
        .tempfile_in(target_folder)
        .map_err(|err| format!("failed to create extraction staging file: {err}"))?;
    write(staged.as_file_mut())?;
    staged
        .as_file()
        .sync_all()
        .map_err(|err| format!("failed to sync finalized extraction: {err}"))?;
    publish_staged_extraction(staged, source_path, target_folder)
}

pub(in super::super) fn publish_staged_extraction(
    mut staged: tempfile::NamedTempFile,
    source_path: &Path,
    target_folder: &Path,
) -> Result<PathBuf, String> {
    for index in 0..10_000 {
        let candidate = extraction_path(source_path, target_folder, index)?;
        match staged.persist_noclobber(&candidate) {
            Ok(_) => return Ok(candidate),
            Err(error) if error.error.kind() == ErrorKind::AlreadyExists => {
                staged = error.file;
            }
            Err(error) => {
                return Err(format!(
                    "failed to publish extraction {}: {}",
                    candidate.display(),
                    error.error
                ));
            }
        }
    }
    Err(String::from(
        "Could not find an available extraction file name",
    ))
}

fn extraction_path(
    source_path: &Path,
    target_folder: &Path,
    index: usize,
) -> Result<PathBuf, String> {
    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| String::from("Source sample has no file name"))?;
    let suffix = if index == 0 {
        String::from("_extraction")
    } else {
        format!("_extraction_{index}")
    };
    Ok(target_folder.join(format!("{stem}{suffix}.wav")))
}

pub(in super::super) fn wav_writer(
    output: &mut File,
    spec: hound::WavSpec,
) -> Result<hound::WavWriter<BufWriter<&mut File>>, String> {
    hound::WavWriter::new(BufWriter::new(output), spec)
        .map_err(|err| format!("failed to create extraction: {err}"))
}

pub(in super::super) fn finalize_wav_writer<W: Write + Seek>(
    writer: hound::WavWriter<W>,
) -> Result<(), String> {
    writer
        .finalize()
        .map_err(|err| format!("failed to finalize extraction: {err}"))
}

pub(in super::super) fn write_wav_frame_range<R: Read + Seek>(
    mut reader: hound::WavReader<R>,
    spec: hound::WavSpec,
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    output: &mut File,
    gain: f32,
) -> Result<(), String> {
    let sample_count = end_frame
        .saturating_sub(start_frame)
        .saturating_mul(channels);
    let mut writer = wav_writer(output, spec)?;
    let start_frame =
        u32::try_from(start_frame).map_err(|_| String::from("WAV selection starts too late"))?;
    reader
        .seek(start_frame)
        .map_err(|err| format!("failed to seek WAV selection: {err}"))?;
    match spec.sample_format {
        hound::SampleFormat::Float => write_samples::<_, _, f32>(
            &mut reader,
            &mut writer,
            sample_count,
            channels,
            spec.sample_rate,
            spec.bits_per_sample,
            gain,
        )?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => write_samples::<_, _, i16>(
            &mut reader,
            &mut writer,
            sample_count,
            channels,
            spec.sample_rate,
            spec.bits_per_sample,
            gain,
        )?,
        hound::SampleFormat::Int => write_samples::<_, _, i32>(
            &mut reader,
            &mut writer,
            sample_count,
            channels,
            spec.sample_rate,
            spec.bits_per_sample,
            gain,
        )?,
    }
    finalize_wav_writer(writer)
}

fn write_samples<R, W, S>(
    reader: &mut hound::WavReader<R>,
    writer: &mut hound::WavWriter<W>,
    sample_count: usize,
    channels: usize,
    sample_rate: u32,
    bits_per_sample: u16,
    gain: f32,
) -> Result<(), String>
where
    R: Read,
    W: Write + Seek,
    S: hound::Sample + FadedSample,
{
    let frame_count = sample_count / channels.max(1);
    let fade_frames =
        short_edge_fade_frame_count(sample_rate, frame_count, DEFAULT_SHORT_EDGE_FADE);
    for (sample_index, sample) in reader.samples::<S>().take(sample_count).enumerate() {
        let frame = sample_index / channels.max(1);
        let gain = gain * short_edge_fade_gain(frame, frame_count, fade_frames);
        writer
            .write_sample(
                sample
                    .map_err(|err| format!("failed to read sample: {err}"))?
                    .with_gain(gain, bits_per_sample),
            )
            .map_err(|err| format!("failed to write extraction: {err}"))?;
    }
    Ok(())
}

trait FadedSample {
    fn with_gain(self, gain: f32, bits_per_sample: u16) -> Self;
}

impl FadedSample for f32 {
    fn with_gain(self, gain: f32, _bits_per_sample: u16) -> Self {
        (self * gain).clamp(-1.0, 1.0)
    }
}

impl FadedSample for i16 {
    fn with_gain(self, gain: f32, bits_per_sample: u16) -> Self {
        let (min, max) = signed_integer_sample_bounds(bits_per_sample.min(16));
        (f64::from(self) * f64::from(gain))
            .round()
            .clamp(min as f64, max as f64) as i16
    }
}

impl FadedSample for i32 {
    fn with_gain(self, gain: f32, bits_per_sample: u16) -> Self {
        let (min, max) = signed_integer_sample_bounds(bits_per_sample.min(32));
        (f64::from(self) * f64::from(gain))
            .round()
            .clamp(min as f64, max as f64) as i32
    }
}

fn signed_integer_sample_bounds(bits_per_sample: u16) -> (i64, i64) {
    let bits = u32::from(bits_per_sample.clamp(1, 32));
    let max = (1_i64 << (bits - 1)) - 1;
    (-1_i64 << (bits - 1), max)
}
