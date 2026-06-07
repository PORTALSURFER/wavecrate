use std::{io::Read, path::Path, sync::Arc};

pub(in crate::native_app::waveform) fn read_audio_file_with_progress(
    path: &Path,
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<Arc<[u8]>, String> {
    let mut file =
        std::fs::File::open(path).map_err(|err| format!("failed to read audio file: {err}"))?;
    let total = file.metadata().ok().map(|metadata| metadata.len() as usize);
    let mut bytes = Vec::with_capacity(total.unwrap_or_default().min(64 * 1024 * 1024));
    let mut buffer = [0_u8; 256 * 1024];
    let mut read = 0usize;
    loop {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        let count = file
            .read(&mut buffer)
            .map_err(|err| format!("failed to read audio file: {err}"))?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
        read = read.saturating_add(count);
        if let Some(total) = total.filter(|total| *total > 0) {
            super::progress::report_phase_progress(start, end, read, total, progress);
        }
    }
    progress(end);
    Ok(bytes.into())
}
