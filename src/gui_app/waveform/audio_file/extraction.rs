use std::{io::Cursor, path::Path, path::PathBuf};

pub(in crate::gui_app) fn extract_wav_range_to_sibling(
    source_path: &Path,
    bytes: &[u8],
    loaded_frames: usize,
    selection: wavecrate::selection::SelectionRange,
) -> Result<PathBuf, String> {
    let parent = source_path
        .parent()
        .ok_or_else(|| String::from("Source sample has no parent folder"))?;
    extract_wav_range_to_folder(source_path, parent, bytes, loaded_frames, selection)
}

pub(in crate::gui_app) fn extract_wav_range_to_folder(
    source_path: &Path,
    target_folder: &Path,
    bytes: &[u8],
    loaded_frames: usize,
    selection: wavecrate::selection::SelectionRange,
) -> Result<PathBuf, String> {
    let cursor = Cursor::new(bytes);
    let reader =
        hound::WavReader::new(cursor).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let total_frames = (reader.duration() as usize).min(loaded_frames);
    if total_frames == 0 {
        return Err(String::from("WAV contains no complete frames"));
    }
    let frame_range = selection.frame_bounds(total_frames);
    let output_path = next_extraction_path(source_path, target_folder)?;
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

fn write_wav_frame_range<R: std::io::Read>(
    mut reader: hound::WavReader<R>,
    spec: hound::WavSpec,
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    output_path: &Path,
) -> Result<(), String> {
    let start_sample = start_frame.saturating_mul(channels);
    let sample_count = end_frame
        .saturating_sub(start_frame)
        .saturating_mul(channels);
    let mut writer = hound::WavWriter::create(output_path, spec)
        .map_err(|err| format!("failed to create extraction: {err}"))?;
    match spec.sample_format {
        hound::SampleFormat::Float => {
            write_samples::<_, f32>(&mut reader, &mut writer, start_sample, sample_count)?
        }
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            write_samples::<_, i16>(&mut reader, &mut writer, start_sample, sample_count)?
        }
        hound::SampleFormat::Int => {
            write_samples::<_, i32>(&mut reader, &mut writer, start_sample, sample_count)?
        }
    }
    writer
        .finalize()
        .map_err(|err| format!("failed to finalize extraction: {err}"))?;
    Ok(())
}

fn write_samples<R, S>(
    reader: &mut hound::WavReader<R>,
    writer: &mut hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    start_sample: usize,
    sample_count: usize,
) -> Result<(), String>
where
    R: std::io::Read,
    S: hound::Sample,
{
    for sample in reader.samples::<S>().skip(start_sample).take(sample_count) {
        writer
            .write_sample(sample.map_err(|err| format!("failed to read sample: {err}"))?)
            .map_err(|err| format!("failed to write extraction: {err}"))?;
    }
    Ok(())
}
