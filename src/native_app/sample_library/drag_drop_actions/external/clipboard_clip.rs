use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radiant::runtime::BusinessWorkContext;
use wavecrate::{app_dirs, external_clipboard};

use crate::native_app::waveform::{WaveformExtractionRequest, execute_waveform_extraction};

const CLIPBOARD_CLIP_CACHE_VERSION: &[u8] = b"wavecrate-clipboard-clip-v1";

pub(super) fn copy_waveform_selection_clip_to_clipboard(
    worker_context: BusinessWorkContext,
    request: WaveformExtractionRequest,
) -> Result<PathBuf, String> {
    let target_folder = clipboard_clip_staging_dir()?;
    worker_context.checkpoint()?;
    let path = match staged_clip_path_for_request(&request, &target_folder)? {
        Some(staged_path) if reusable_staged_clip_available(&staged_path)? => staged_path,
        Some(staged_path) => render_selection_clip_to_staged_path(
            &worker_context,
            request,
            &target_folder,
            &staged_path,
        )?,
        None => render_selection_clip_to_unique_path(&worker_context, request, &target_folder)?,
    };
    copy_single_file_to_clipboard_and_confirm(&path)?;
    worker_context.checkpoint()?;
    Ok(path)
}

fn render_selection_clip_to_unique_path(
    worker_context: &BusinessWorkContext,
    request: WaveformExtractionRequest,
    target_folder: &Path,
) -> Result<PathBuf, String> {
    let completion =
        execute_waveform_extraction(request.with_target_folder(target_folder.to_path_buf()));
    worker_context.checkpoint()?;
    completion.result
}

fn render_selection_clip_to_staged_path(
    worker_context: &BusinessWorkContext,
    request: WaveformExtractionRequest,
    target_folder: &Path,
    staged_path: &Path,
) -> Result<PathBuf, String> {
    let rendered_path =
        render_selection_clip_to_unique_path(worker_context, request, target_folder)?;
    if rendered_path == staged_path {
        return Ok(rendered_path);
    }
    match fs::rename(&rendered_path, staged_path) {
        Ok(()) => Ok(staged_path.to_path_buf()),
        Err(_) if staged_path.is_file() => {
            let _ = fs::remove_file(&rendered_path);
            Ok(staged_path.to_path_buf())
        }
        Err(error) => {
            let _ = fs::remove_file(&rendered_path);
            Err(format!(
                "Failed to stage clipboard clip at {}: {error}",
                staged_path.display()
            ))
        }
    }
}

fn staged_clip_path_for_request(
    request: &WaveformExtractionRequest,
    target_folder: &Path,
) -> Result<Option<PathBuf>, String> {
    let Some(source_identity) = SourceFileIdentity::from_path(request.source_path()) else {
        return Ok(None);
    };
    let stem = request
        .source_path()
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| String::from("Source sample has no file name"))?;
    let key = staged_clip_key(request, source_identity);
    Ok(Some(target_folder.join(format!("{stem}_clip_{key}.wav"))))
}

fn staged_clip_key(
    request: &WaveformExtractionRequest,
    source_identity: SourceFileIdentity,
) -> String {
    let selection = request.selection();
    let frames = selection.frame_bounds(request.loaded_frames());
    let mut hasher = blake3::Hasher::new();
    hasher.update(CLIPBOARD_CLIP_CACHE_VERSION);
    hasher.update(request.source_path().to_string_lossy().as_bytes());
    hasher.update(&source_identity.len.to_le_bytes());
    hasher.update(&source_identity.modified_nanos.to_le_bytes());
    hasher.update(&request.sample_rate().to_le_bytes());
    hasher.update(&(request.channels() as u64).to_le_bytes());
    hasher.update(&(request.loaded_frames() as u64).to_le_bytes());
    hasher.update(&(frames.start_frame as u64).to_le_bytes());
    hasher.update(&(frames.end_frame as u64).to_le_bytes());
    hasher.update(&selection.start_f64().to_le_bytes());
    hasher.update(&selection.end_f64().to_le_bytes());
    short_clip_hash(hasher.finalize())
}

fn short_clip_hash(hash: blake3::Hash) -> String {
    let mut key = String::with_capacity(16);
    for byte in &hash.as_bytes()[..8] {
        let _ = write!(&mut key, "{byte:02x}");
    }
    key
}

fn reusable_staged_clip_available(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    let metadata = fs::metadata(path)
        .map_err(|err| format!("Failed to inspect clipboard clip {}: {err}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "Clipboard clip staging path is not a file: {}",
            path.display()
        ));
    }
    if metadata.len() == 0 {
        let _ = fs::remove_file(path);
        return Ok(false);
    }
    match hound::WavReader::open(path) {
        Ok(reader) if reader.duration() > 0 => Ok(true),
        Ok(_) | Err(_) => {
            let _ = fs::remove_file(path);
            Ok(false)
        }
    }
}

fn copy_single_file_to_clipboard_and_confirm(path: &Path) -> Result<(), String> {
    let path = path.to_path_buf();
    external_clipboard::copy_file_paths(std::slice::from_ref(&path))?;
    let copied_paths = external_clipboard::read_file_paths()
        .map_err(|error| format!("Clipboard verification failed: {error}"))?;
    if copied_paths
        .iter()
        .any(|copied_path| same_filesystem_path(copied_path, &path))
    {
        return Ok(());
    }
    Err(format!(
        "Clipboard verification failed: {} was not on the clipboard",
        path.display()
    ))
}

fn same_filesystem_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    let Ok(left) = fs::canonicalize(left) else {
        return false;
    };
    let Ok(right) = fs::canonicalize(right) else {
        return false;
    };
    left == right
}

#[derive(Clone, Copy)]
struct SourceFileIdentity {
    len: u64,
    modified_nanos: u128,
}

impl SourceFileIdentity {
    fn from_path(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        if !metadata.is_file() {
            return None;
        }
        Some(Self {
            len: metadata.len(),
            modified_nanos: metadata
                .modified()
                .ok()
                .and_then(system_time_nanos)
                .unwrap_or(0),
        })
    }
}

fn system_time_nanos(time: SystemTime) -> Option<u128> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_nanos())
}

fn clipboard_clip_staging_dir() -> Result<PathBuf, String> {
    let folder = app_dirs::handoff_staging_dir()
        .map_err(|err| err.to_string())?
        .join("clipboard_clips");
    fs::create_dir_all(&folder)
        .map_err(|err| format!("Failed to create clipboard clip folder: {err}"))?;
    Ok(folder)
}
