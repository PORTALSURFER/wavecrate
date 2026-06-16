use std::{
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const NORMALIZED_PEAK_TOLERANCE: f32 = 0.001;
const ANALYZE_PROGRESS_END: f32 = 0.45;
const WRITE_PROGRESS_START: f32 = 0.45;
const WRITE_PROGRESS_END: f32 = 0.95;
const REPLACE_PROGRESS: f32 = 0.98;
const NORMALIZATION_PROGRESS_SAMPLE_STEP: u64 = 16_384;
const REPLACE_RETRY_COUNT: usize = 12;
const REPLACE_RETRY_DELAY: Duration = Duration::from_millis(75);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum WavNormalizationOutcome {
    Normalized,
    Skipped,
}

#[cfg(test)]
pub(in crate::native_app) fn normalize_wav_file_in_place(
    path: &Path,
) -> Result<WavNormalizationOutcome, String> {
    normalize_wav_file_in_place_with_progress(path, |_, _| {})
}

pub(in crate::native_app) fn normalize_wav_file_in_place_with_progress(
    path: &Path,
    mut progress: impl FnMut(f32, &'static str),
) -> Result<WavNormalizationOutcome, String> {
    let started_at = Instant::now();
    ensure_normalizable_wav(path)?;
    progress(0.0, "Opening");
    let analyze_started_at = Instant::now();
    let analysis = analyze_wav_peak(path, |fraction| {
        progress(fraction * ANALYZE_PROGRESS_END, "Analyzing");
    })?;
    log_normalization_phase(path, "analyze", analyze_started_at);
    if analysis.sample_count == 0 {
        return Err(String::from("No audio data to normalize"));
    }
    if !analysis.peak.is_finite() || analysis.peak <= f32::EPSILON {
        progress(1.0, "Skipped");
        log_normalization_phase(path, "total_skipped_silent", started_at);
        return Ok(WavNormalizationOutcome::Skipped);
    }
    if (1.0 - analysis.peak).abs() <= NORMALIZED_PEAK_TOLERANCE {
        progress(1.0, "Already normalized");
        log_normalization_phase(path, "total_skipped_normalized", started_at);
        return Ok(WavNormalizationOutcome::Skipped);
    }

    let temp_path = temporary_normalized_path(path);
    let backup_path = backup_original_path(path);
    let write_started_at = Instant::now();
    let result = write_normalized_wav(
        path,
        &temp_path,
        analysis.spec,
        1.0 / analysis.peak,
        |fraction| {
            progress(
                WRITE_PROGRESS_START + fraction * (WRITE_PROGRESS_END - WRITE_PROGRESS_START),
                "Writing",
            );
        },
    )
    .inspect(|()| log_normalization_phase(path, "write", write_started_at))
    .and_then(|()| {
        progress(REPLACE_PROGRESS, "Replacing");
        let replace_started_at = Instant::now();
        replace_with_backup(path, &temp_path, &backup_path)
            .inspect(|()| log_normalization_phase(path, "replace", replace_started_at))
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result.map(|()| {
        progress(1.0, "Done");
        log_normalization_phase(path, "total_normalized", started_at);
        WavNormalizationOutcome::Normalized
    })
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

fn analyze_wav_peak(path: &Path, mut progress: impl FnMut(f32)) -> Result<WavPeakAnalysis, String> {
    let reader_source = wavecrate::wav_sanitize::open_sanitized_wav(path)?;
    let buf_reader = std::io::BufReader::with_capacity(1024 * 1024, reader_source);
    let mut reader =
        hound::WavReader::new(buf_reader).map_err(|err| format!("Invalid WAV: {err}"))?;
    let spec = reader.spec();
    let total_samples = reader.duration() as u64;
    let mut sample_count = 0_u64;
    let mut peak = 0.0_f32;
    read_wav_samples_as_f32(&mut reader, spec, |sample| {
        sample_count = sample_count.saturating_add(1);
        peak = peak.max(sample.abs());
        report_sample_progress(sample_count, total_samples, &mut progress);
        Ok(())
    })?;
    progress(1.0);
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
                sample(value.map_err(|err| format!("Invalid WAV sample data: {err}"))?)?;
            }
            Ok(())
        }
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            let scale = (1i64 << spec.bits_per_sample.saturating_sub(1)).max(1) as f32;
            for value in reader.samples::<i16>() {
                sample(
                    value.map_err(|err| format!("Invalid WAV sample data: {err}"))? as f32 / scale,
                )?;
            }
            Ok(())
        }
        hound::SampleFormat::Int => {
            let scale = (1i64 << spec.bits_per_sample.saturating_sub(1)).max(1) as f32;
            for value in reader.samples::<i32>() {
                sample(
                    value.map_err(|err| format!("Invalid WAV sample data: {err}"))? as f32 / scale,
                )?;
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
    progress: impl FnMut(f32),
) -> Result<(), String> {
    let reader_source = wavecrate::wav_sanitize::open_sanitized_wav(source_path)?;
    let buf_reader = std::io::BufReader::with_capacity(1024 * 1024, reader_source);
    let mut reader =
        hound::WavReader::new(buf_reader).map_err(|err| format!("Invalid WAV: {err}"))?;
    let total_samples = reader.duration() as u64;
    let file = std::fs::File::create(target_path)
        .map_err(|err| format!("Failed to create normalized temp file: {err}"))?;
    let buf_writer = std::io::BufWriter::with_capacity(1024 * 1024, file);
    let output_spec = normalized_wav_spec(spec);
    let mut writer = hound::WavWriter::new(buf_writer, output_spec)
        .map_err(|err| format!("Failed to write wav: {err}"))?;
    write_normalized_samples(
        &mut reader,
        spec,
        output_spec,
        gain,
        &mut writer,
        total_samples,
        progress,
    )?;
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
    total_samples: u64,
    mut progress: impl FnMut(f32),
) -> Result<(), String> {
    let mut sample_count = 0_u64;
    match output_spec.sample_format {
        hound::SampleFormat::Float => read_wav_samples_as_f32(reader, source_spec, |sample| {
            sample_count = sample_count.saturating_add(1);
            report_sample_progress(sample_count, total_samples, &mut progress);
            writer
                .write_sample(normalized_float_sample(sample, gain))
                .map_err(|err| format!("Failed to write sample: {err}"))
        }),
        hound::SampleFormat::Int if output_spec.bits_per_sample <= 16 => {
            read_wav_samples_as_f32(reader, source_spec, |sample| {
                sample_count = sample_count.saturating_add(1);
                report_sample_progress(sample_count, total_samples, &mut progress);
                writer
                    .write_sample(
                        normalized_int_sample(sample, gain, output_spec.bits_per_sample) as i16,
                    )
                    .map_err(|err| format!("Failed to write sample: {err}"))
            })
        }
        hound::SampleFormat::Int => read_wav_samples_as_f32(reader, source_spec, |sample| {
            sample_count = sample_count.saturating_add(1);
            report_sample_progress(sample_count, total_samples, &mut progress);
            writer
                .write_sample(normalized_int_sample(
                    sample,
                    gain,
                    output_spec.bits_per_sample,
                ))
                .map_err(|err| format!("Failed to write sample: {err}"))
        }),
    }?;
    progress(1.0);
    Ok(())
}

fn report_sample_progress(sample_count: u64, total_samples: u64, progress: &mut impl FnMut(f32)) {
    if total_samples == 0 || !sample_count.is_multiple_of(NORMALIZATION_PROGRESS_SAMPLE_STEP) {
        return;
    }
    progress((sample_count as f32 / total_samples as f32).clamp(0.0, 1.0));
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

fn log_normalization_phase(path: &Path, phase: &'static str, started_at: Instant) {
    tracing::info!(
        target: "wavecrate::debug::normalization",
        event = "browser.normalize.worker.phase",
        phase,
        elapsed_ms = started_at.elapsed().as_secs_f64() * 1000.0,
        path = %path.display()
    );
}
