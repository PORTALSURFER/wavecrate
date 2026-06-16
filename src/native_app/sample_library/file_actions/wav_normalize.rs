use std::{
    io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const REPLACE_RETRY_COUNT: usize = 12;
const REPLACE_RETRY_DELAY: Duration = Duration::from_millis(75);

pub(in crate::native_app) fn normalize_wav_file_in_place(path: &Path) -> Result<(), String> {
    ensure_normalizable_wav(path)?;
    let analysis = analyze_wav_peak(path)?;
    if analysis.sample_count == 0 {
        return Err(String::from("No audio data to normalize"));
    }
    if !analysis.peak.is_finite() || analysis.peak <= f32::EPSILON {
        return Ok(());
    }

    let temp_path = temporary_normalized_path(path);
    let backup_path = backup_original_path(path);
    let result = write_normalized_wav(path, &temp_path, analysis.spec, 1.0 / analysis.peak)
        .and_then(|()| replace_with_backup(path, &temp_path, &backup_path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

fn ensure_normalizable_wav(path: &Path) -> Result<(), String> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
    {
        return Ok(());
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{extension}"))
        .unwrap_or_else(|| String::from("this file type"));
    Err(format!(
        "Normalize overwrite only supports WAV files; {extension} is not supported"
    ))
}

struct WavPeakAnalysis {
    spec: hound::WavSpec,
    sample_count: u64,
    peak: f32,
}

fn analyze_wav_peak(path: &Path) -> Result<WavPeakAnalysis, String> {
    let reader_source = wavecrate::wav_sanitize::open_sanitized_wav(path)?;
    let buf_reader = std::io::BufReader::with_capacity(1024 * 1024, reader_source);
    let mut reader =
        hound::WavReader::new(buf_reader).map_err(|err| format!("Invalid wav: {err}"))?;
    let spec = reader.spec();
    let mut sample_count = 0_u64;
    let mut peak = 0.0_f32;
    read_wav_samples_as_f32(&mut reader, spec, |sample| {
        sample_count = sample_count.saturating_add(1);
        peak = peak.max(sample.abs());
        Ok(())
    })?;
    Ok(WavPeakAnalysis {
        spec,
        sample_count,
        peak,
    })
}

fn read_wav_samples_as_f32<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    spec: hound::WavSpec,
    mut sample: impl FnMut(f32) -> Result<(), String>,
) -> Result<(), String> {
    match spec.sample_format {
        hound::SampleFormat::Float => {
            for value in reader.samples::<f32>() {
                sample(value.map_err(|err| format!("Sample error: {err}"))?)?;
            }
            Ok(())
        }
        hound::SampleFormat::Int => {
            let scale = (1i64 << spec.bits_per_sample.saturating_sub(1)).max(1) as f32;
            for value in reader.samples::<i32>() {
                sample(value.map_err(|err| format!("Sample error: {err}"))? as f32 / scale)?;
            }
            Ok(())
        }
    }
}

fn write_normalized_wav(
    source_path: &Path,
    target_path: &Path,
    spec: hound::WavSpec,
    gain: f32,
) -> Result<(), String> {
    let reader_source = wavecrate::wav_sanitize::open_sanitized_wav(source_path)?;
    let buf_reader = std::io::BufReader::with_capacity(1024 * 1024, reader_source);
    let mut reader =
        hound::WavReader::new(buf_reader).map_err(|err| format!("Invalid wav: {err}"))?;
    let file = std::fs::File::create(target_path)
        .map_err(|err| format!("Failed to create normalized temp file: {err}"))?;
    let buf_writer = std::io::BufWriter::with_capacity(1024 * 1024, file);
    let output_spec = normalized_wav_spec(spec);
    let mut writer = hound::WavWriter::new(buf_writer, output_spec)
        .map_err(|err| format!("Failed to write wav: {err}"))?;
    write_normalized_samples(&mut reader, spec, output_spec, gain, &mut writer)?;
    writer
        .finalize()
        .map_err(|err| format!("Failed to finalize wav: {err}"))
}

fn normalized_wav_spec(source: hound::WavSpec) -> hound::WavSpec {
    match source.sample_format {
        hound::SampleFormat::Float => hound::WavSpec {
            channels: source.channels.max(1),
            sample_rate: source.sample_rate.max(1),
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
        hound::SampleFormat::Int => hound::WavSpec {
            channels: source.channels.max(1),
            sample_rate: source.sample_rate.max(1),
            bits_per_sample: source.bits_per_sample.clamp(8, 32),
            sample_format: hound::SampleFormat::Int,
        },
    }
}

fn write_normalized_samples<W: io::Write + io::Seek, R: io::Read>(
    reader: &mut hound::WavReader<R>,
    source_spec: hound::WavSpec,
    output_spec: hound::WavSpec,
    gain: f32,
    writer: &mut hound::WavWriter<W>,
) -> Result<(), String> {
    match output_spec.sample_format {
        hound::SampleFormat::Float => read_wav_samples_as_f32(reader, source_spec, |sample| {
            writer
                .write_sample(normalized_float_sample(sample, gain))
                .map_err(|err| format!("Failed to write sample: {err}"))
        }),
        hound::SampleFormat::Int if output_spec.bits_per_sample <= 16 => {
            read_wav_samples_as_f32(reader, source_spec, |sample| {
                writer
                    .write_sample(
                        normalized_int_sample(sample, gain, output_spec.bits_per_sample) as i16,
                    )
                    .map_err(|err| format!("Failed to write sample: {err}"))
            })
        }
        hound::SampleFormat::Int => read_wav_samples_as_f32(reader, source_spec, |sample| {
            writer
                .write_sample(normalized_int_sample(
                    sample,
                    gain,
                    output_spec.bits_per_sample,
                ))
                .map_err(|err| format!("Failed to write sample: {err}"))
        }),
    }
}

fn normalized_float_sample(sample: f32, gain: f32) -> f32 {
    if sample.is_finite() {
        (sample * gain).clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn normalized_int_sample(sample: f32, gain: f32, bits_per_sample: u16) -> i32 {
    let normalized = normalized_float_sample(sample, gain);
    let max = ((1_i64 << bits_per_sample.saturating_sub(1)) - 1).max(1) as f32;
    (normalized * max).round().clamp(-max, max) as i32
}

fn replace_with_backup(path: &Path, temp_path: &Path, backup_path: &Path) -> Result<(), String> {
    retry_rename(path, backup_path)
        .map_err(|err| format!("Failed to stage original file for replacement: {err}"))?;
    match retry_rename(temp_path, path) {
        Ok(()) => {
            let _ = std::fs::remove_file(backup_path);
            Ok(())
        }
        Err(err) => {
            let restore_result = retry_rename(backup_path, path);
            let _ = std::fs::remove_file(temp_path);
            match restore_result {
                Ok(()) => Err(format!("Failed to replace normalized file: {err}")),
                Err(restore_err) => Err(format!(
                    "Failed to replace normalized file: {err}; original remains at {} and could not be restored: {restore_err}",
                    backup_path.display()
                )),
            }
        }
    }
}

fn retry_rename(from: &Path, to: &Path) -> io::Result<()> {
    let mut last_error = None;
    for attempt in 0..=REPLACE_RETRY_COUNT {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(err) => {
                last_error = Some(err);
                if attempt < REPLACE_RETRY_COUNT {
                    std::thread::sleep(REPLACE_RETRY_DELAY);
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| io::Error::other("rename failed")))
}

fn temporary_normalized_path(path: &Path) -> PathBuf {
    sibling_work_path(path, "normalize", "tmp")
}

fn backup_original_path(path: &Path) -> PathBuf {
    sibling_work_path(path, "normalize-backup", "bak")
}

fn sibling_work_path(path: &Path, label: &str, extension: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sample");
    path.with_file_name(format!(
        ".{file_name}.wavecrate-{label}-{}-{stamp}.{extension}",
        std::process::id()
    ))
}
