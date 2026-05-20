use std::{path::Path, process};

pub(super) fn sample_path_label(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

pub(super) fn format_copy_path(path: &Path) -> String {
    let mut rendered = path.to_string_lossy().replace('\\', "/");
    if rendered.contains(' ') {
        rendered = format!("\"{rendered}\"");
    }
    rendered
}

pub(super) fn reveal_in_file_explorer(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }
    #[cfg(target_os = "windows")]
    {
        let status = process::Command::new("explorer.exe")
            .arg(format!("/select,{}", windows_explorer_target(path)))
            .status()
            .map_err(|err| format!("Failed to launch explorer: {err}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "Explorer exited unsuccessfully for {}",
                path.display()
            ))
        }
    }
    #[cfg(target_os = "macos")]
    {
        let status = process::Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
            .map_err(|err| format!("Failed to launch Finder: {err}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "Finder exited unsuccessfully for {}",
                path.display()
            ))
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let parent = path
            .parent()
            .ok_or_else(|| "Unable to resolve parent directory".to_string())?;
        open::that(parent)
            .map_err(|err| format!("Could not open folder {}: {err}", parent.display()))
    }
}

pub(super) fn open_folder_in_file_explorer(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Folder not found: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("Not a folder: {}", path.display()));
    }
    #[cfg(target_os = "windows")]
    {
        let status = process::Command::new("explorer.exe")
            .arg(windows_explorer_target(path))
            .status()
            .map_err(|err| format!("Failed to launch explorer: {err}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "Explorer exited unsuccessfully for {}",
                path.display()
            ))
        }
    }
    #[cfg(target_os = "macos")]
    {
        let status = process::Command::new("open")
            .arg(path)
            .status()
            .map_err(|err| format!("Failed to launch Finder: {err}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "Finder exited unsuccessfully for {}",
                path.display()
            ))
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        open::that(path).map_err(|err| format!("Could not open folder {}: {err}", path.display()))
    }
}

#[cfg(target_os = "windows")]
fn windows_explorer_target(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

pub(super) fn normalize_wav_file_in_place(path: &Path) -> Result<(), String> {
    ensure_normalizable_wav(path)?;
    let reader_source = wavecrate::wav_sanitize::open_sanitized_wav(path)?;
    let buf_reader = std::io::BufReader::with_capacity(1024 * 1024, reader_source);
    let mut reader =
        hound::WavReader::new(buf_reader).map_err(|err| format!("Invalid wav: {err}"))?;
    let spec = reader.spec();
    let mut samples = read_wav_samples_as_f32(&mut reader, spec)?;
    if samples.is_empty() {
        return Err(String::from("No audio data to normalize"));
    }
    normalize_peak_in_place(&mut samples);
    let target_spec = hound::WavSpec {
        channels: spec.channels.max(1),
        sample_rate: spec.sample_rate.max(1),
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    write_f32_wav(path, &samples, target_spec)
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

fn read_wav_samples_as_f32<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    spec: hound::WavSpec,
) -> Result<Vec<f32>, String> {
    match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| sample.map_err(|err| format!("Sample error: {err}")))
            .collect(),
        hound::SampleFormat::Int => {
            let scale = (1i64 << spec.bits_per_sample.saturating_sub(1)).max(1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| value as f32 / scale)
                        .map_err(|err| format!("Sample error: {err}"))
                })
                .collect()
        }
    }
}

fn normalize_peak_in_place(samples: &mut [f32]) {
    let peak = samples
        .iter()
        .copied()
        .map(f32::abs)
        .fold(0.0_f32, f32::max);
    if !peak.is_finite() || peak <= f32::EPSILON {
        return;
    }
    let gain = 1.0 / peak;
    for sample in samples {
        *sample = (*sample * gain).clamp(-1.0, 1.0);
    }
}

fn write_f32_wav(path: &Path, samples: &[f32], spec: hound::WavSpec) -> Result<(), String> {
    let file =
        std::fs::File::create(path).map_err(|err| format!("Failed to create file: {err}"))?;
    let buf_writer = std::io::BufWriter::with_capacity(1024 * 1024, file);
    let mut writer = hound::WavWriter::new(buf_writer, spec)
        .map_err(|err| format!("Failed to write wav: {err}"))?;
    for sample in samples {
        writer
            .write_sample(*sample)
            .map_err(|err| format!("Failed to write sample: {err}"))?;
    }
    writer
        .finalize()
        .map_err(|err| format!("Failed to finalize wav: {err}"))
}
