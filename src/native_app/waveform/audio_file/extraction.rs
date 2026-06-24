use std::{
    fs::File,
    io::{BufReader, Cursor, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use wavecrate::audio::{
    DEFAULT_SHORT_EDGE_FADE, short_edge_fade_frame_count, short_edge_fade_gain,
};

mod raw_wav;

const F32_SAMPLE_BYTES: u64 = std::mem::size_of::<f32>() as u64;

pub(in crate::native_app::waveform) struct InterleavedF32FileExtractionSource<'a> {
    pub(in crate::native_app::waveform) cache_path: &'a Path,
    pub(in crate::native_app::waveform) sample_count: u64,
    pub(in crate::native_app::waveform) sample_rate: u32,
    pub(in crate::native_app::waveform) channels: usize,
    pub(in crate::native_app::waveform) loaded_frames: usize,
}

pub(in crate::native_app) fn extract_wav_range_to_folder(
    source_path: &Path,
    target_folder: &Path,
    bytes: &[u8],
    loaded_frames: usize,
    selection: wavecrate::selection::SelectionRange,
) -> Result<PathBuf, String> {
    let cursor = Cursor::new(bytes);
    extract_wav_reader_range_to_folder(source_path, target_folder, cursor, loaded_frames, selection)
}

pub(in crate::native_app) fn extract_wav_file_range_to_folder(
    source_path: &Path,
    target_folder: &Path,
    loaded_frames: usize,
    selection: wavecrate::selection::SelectionRange,
) -> Result<PathBuf, String> {
    let file = File::open(source_path)
        .map_err(|err| format!("failed to open source WAV {}: {err}", source_path.display()))?;
    extract_wav_reader_range_to_folder(source_path, target_folder, file, loaded_frames, selection)
}

pub(in crate::native_app) fn extract_interleaved_f32_range_to_folder(
    source_path: &Path,
    target_folder: &Path,
    samples: &[f32],
    sample_rate: u32,
    channels: usize,
    loaded_frames: usize,
    selection: wavecrate::selection::SelectionRange,
) -> Result<PathBuf, String> {
    let spec = playback_wav_spec(sample_rate, channels)?;
    let total_frames = usable_interleaved_frame_count(samples.len(), channels, loaded_frames)?;
    let frame_range = selection.frame_bounds(total_frames);
    let sample_bounds = interleaved_sample_bounds(
        frame_range.start_frame,
        frame_range.end_frame,
        channels,
        samples.len(),
    )?;
    let output_path = next_extraction_path(source_path, target_folder)?;
    let mut writer = hound::WavWriter::create(&output_path, spec)
        .map_err(|err| format!("failed to create extraction: {err}"))?;
    write_f32_samples_with_edge_fade(
        &mut writer,
        &samples[sample_bounds.start..sample_bounds.end],
        channels,
        sample_rate,
        frame_range
            .end_frame
            .saturating_sub(frame_range.start_frame),
    )?;
    writer
        .finalize()
        .map_err(|err| format!("failed to finalize extraction: {err}"))?;
    Ok(output_path)
}

fn write_f32_samples_with_edge_fade(
    writer: &mut hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    samples: &[f32],
    channels: usize,
    sample_rate: u32,
    frame_count: usize,
) -> Result<(), String> {
    let fade_frames =
        short_edge_fade_frame_count(sample_rate, frame_count, DEFAULT_SHORT_EDGE_FADE);
    for (sample_index, sample) in samples.iter().enumerate() {
        let frame = sample_index / channels.max(1);
        let gain = short_edge_fade_gain(frame, frame_count, fade_frames);
        writer
            .write_sample((sample * gain).clamp(-1.0, 1.0))
            .map_err(|err| format!("failed to write extraction: {err}"))?;
    }
    Ok(())
}

pub(in crate::native_app) fn extract_interleaved_f32_file_range_to_folder(
    source_path: &Path,
    target_folder: &Path,
    cache: InterleavedF32FileExtractionSource<'_>,
    selection: wavecrate::selection::SelectionRange,
) -> Result<PathBuf, String> {
    let spec = playback_wav_spec(cache.sample_rate, cache.channels)?;
    let total_frames = usable_interleaved_frame_count_u64(
        cache.sample_count,
        cache.channels,
        cache.loaded_frames,
    )?;
    let frame_range = selection.frame_bounds(total_frames);
    let start_sample = frame_range
        .start_frame
        .checked_mul(cache.channels)
        .ok_or_else(|| String::from("Playback cache selection is too large"))?;
    let samples_to_write = frame_range
        .end_frame
        .saturating_sub(frame_range.start_frame)
        .checked_mul(cache.channels)
        .ok_or_else(|| String::from("Playback cache selection is too large"))?;
    let output_path = next_extraction_path(source_path, target_folder)?;
    let mut reader = open_f32_reader_at(cache.cache_path, start_sample as u64)?;
    let mut writer = hound::WavWriter::create(&output_path, spec)
        .map_err(|err| format!("failed to create extraction: {err}"))?;
    let mut bytes = [0_u8; F32_SAMPLE_BYTES as usize];
    let frame_count = frame_range
        .end_frame
        .saturating_sub(frame_range.start_frame);
    let fade_frames =
        short_edge_fade_frame_count(cache.sample_rate, frame_count, DEFAULT_SHORT_EDGE_FADE);
    for sample_index in 0..samples_to_write {
        reader
            .read_exact(&mut bytes)
            .map_err(|err| format!("failed to read playback cache: {err}"))?;
        let frame = sample_index / cache.channels.max(1);
        let gain = short_edge_fade_gain(frame, frame_count, fade_frames);
        writer
            .write_sample((f32::from_le_bytes(bytes) * gain).clamp(-1.0, 1.0))
            .map_err(|err| format!("failed to write extraction: {err}"))?;
    }
    writer
        .finalize()
        .map_err(|err| format!("failed to finalize extraction: {err}"))?;
    Ok(output_path)
}

pub(super) fn extract_wav_reader_range_to_folder<R: Read + Seek>(
    source_path: &Path,
    target_folder: &Path,
    mut reader: R,
    loaded_frames: usize,
    selection: wavecrate::selection::SelectionRange,
) -> Result<PathBuf, String> {
    let output_path = next_extraction_path(source_path, target_folder)?;
    if raw_wav::copy_selection_to_file(
        &mut reader,
        loaded_frames,
        selection,
        &output_path,
        DEFAULT_SHORT_EDGE_FADE,
    )? {
        return Ok(output_path);
    }
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|err| format!("failed to rewind WAV after fast extraction check: {err}"))?;
    let reader =
        hound::WavReader::new(reader).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let total_frames = (reader.duration() as usize).min(loaded_frames);
    if total_frames == 0 {
        return Err(String::from("WAV contains no complete frames"));
    }
    let frame_range = selection.frame_bounds(total_frames);
    write_wav_frame_range(
        reader,
        spec,
        channels,
        frame_range.start_frame,
        frame_range.end_frame,
        &output_path,
    )?;
    Ok(output_path)
}

fn playback_wav_spec(sample_rate: u32, channels: usize) -> Result<hound::WavSpec, String> {
    if sample_rate == 0 {
        return Err(String::from("Playback cache has no sample rate"));
    }
    let channels = u16::try_from(channels)
        .ok()
        .filter(|channels| *channels > 0)
        .ok_or_else(|| String::from("Playback cache has an invalid channel count"))?;
    Ok(hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    })
}

fn usable_interleaved_frame_count(
    sample_count: usize,
    channels: usize,
    loaded_frames: usize,
) -> Result<usize, String> {
    if channels == 0 {
        return Err(String::from("Playback cache has an invalid channel count"));
    }
    let total_frames = loaded_frames.min(sample_count / channels);
    if total_frames == 0 {
        return Err(String::from("Playback cache contains no complete frames"));
    }
    Ok(total_frames)
}

fn usable_interleaved_frame_count_u64(
    sample_count: u64,
    channels: usize,
    loaded_frames: usize,
) -> Result<usize, String> {
    let channels_u64 = u64::try_from(channels)
        .ok()
        .filter(|channels| *channels > 0)
        .ok_or_else(|| String::from("Playback cache has an invalid channel count"))?;
    let cache_frames = usize::try_from(sample_count / channels_u64).unwrap_or(usize::MAX);
    let total_frames = loaded_frames.min(cache_frames);
    if total_frames == 0 {
        return Err(String::from("Playback cache contains no complete frames"));
    }
    Ok(total_frames)
}

struct SampleBounds {
    start: usize,
    end: usize,
}

fn interleaved_sample_bounds(
    start_frame: usize,
    end_frame: usize,
    channels: usize,
    sample_count: usize,
) -> Result<SampleBounds, String> {
    let start = start_frame
        .checked_mul(channels)
        .ok_or_else(|| String::from("Playback cache selection is too large"))?;
    let end = end_frame
        .checked_mul(channels)
        .ok_or_else(|| String::from("Playback cache selection is too large"))?
        .min(sample_count);
    Ok(SampleBounds { start, end })
}

fn open_f32_reader_at(path: &Path, sample: u64) -> Result<BufReader<File>, String> {
    let mut file = File::open(path)
        .map_err(|err| format!("failed to open playback cache {}: {err}", path.display()))?;
    file.seek(SeekFrom::Start(sample.saturating_mul(F32_SAMPLE_BYTES)))
        .map_err(|err| format!("failed to seek playback cache {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn next_extraction_path(source_path: &Path, target_folder: &Path) -> Result<PathBuf, String> {
    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| String::from("Source sample has no file name"))?;
    for index in 0..10_000 {
        let suffix = if index == 0 {
            String::from("_extraction")
        } else {
            format!("_extraction_{index}")
        };
        let candidate = target_folder.join(format!("{stem}{suffix}.wav"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(String::from(
        "Could not find an available extraction file name",
    ))
}

pub(super) fn write_wav_frame_range<R: Read + Seek>(
    mut reader: hound::WavReader<R>,
    spec: hound::WavSpec,
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    output_path: &Path,
) -> Result<(), String> {
    let sample_count = end_frame
        .saturating_sub(start_frame)
        .saturating_mul(channels);
    let mut writer = hound::WavWriter::create(output_path, spec)
        .map_err(|err| format!("failed to create extraction: {err}"))?;
    let start_frame =
        u32::try_from(start_frame).map_err(|_| String::from("WAV selection starts too late"))?;
    reader
        .seek(start_frame)
        .map_err(|err| format!("failed to seek WAV selection: {err}"))?;
    match spec.sample_format {
        hound::SampleFormat::Float => write_samples::<_, f32>(
            &mut reader,
            &mut writer,
            sample_count,
            channels,
            spec.sample_rate,
        )?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => write_samples::<_, i16>(
            &mut reader,
            &mut writer,
            sample_count,
            channels,
            spec.sample_rate,
        )?,
        hound::SampleFormat::Int => write_samples::<_, i32>(
            &mut reader,
            &mut writer,
            sample_count,
            channels,
            spec.sample_rate,
        )?,
    }
    writer
        .finalize()
        .map_err(|err| format!("failed to finalize extraction: {err}"))?;
    Ok(())
}

fn write_samples<R, S>(
    reader: &mut hound::WavReader<R>,
    writer: &mut hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    sample_count: usize,
    channels: usize,
    sample_rate: u32,
) -> Result<(), String>
where
    R: std::io::Read,
    S: hound::Sample + FadedSample,
{
    let frame_count = sample_count / channels.max(1);
    let fade_frames =
        short_edge_fade_frame_count(sample_rate, frame_count, DEFAULT_SHORT_EDGE_FADE);
    for (sample_index, sample) in reader.samples::<S>().take(sample_count).enumerate() {
        let frame = sample_index / channels.max(1);
        let gain = short_edge_fade_gain(frame, frame_count, fade_frames);
        writer
            .write_sample(
                sample
                    .map_err(|err| format!("failed to read sample: {err}"))?
                    .with_gain(gain),
            )
            .map_err(|err| format!("failed to write extraction: {err}"))?;
    }
    Ok(())
}

trait FadedSample {
    fn with_gain(self, gain: f32) -> Self;
}

impl FadedSample for f32 {
    fn with_gain(self, gain: f32) -> Self {
        (self * gain).clamp(-1.0, 1.0)
    }
}

impl FadedSample for i16 {
    fn with_gain(self, gain: f32) -> Self {
        (f32::from(self) * gain)
            .round()
            .clamp(i16::MIN as f32, i16::MAX as f32) as i16
    }
}

impl FadedSample for i32 {
    fn with_gain(self, gain: f32) -> Self {
        (self as f32 * gain)
            .round()
            .clamp(i32::MIN as f32, i32::MAX as f32) as i32
    }
}
