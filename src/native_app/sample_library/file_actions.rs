use std::path::Path;

mod wav_normalize;
#[cfg(test)]
pub(in crate::native_app) use wav_normalize::normalize_wav_file_in_place;
pub(in crate::native_app) use wav_normalize::{
    WavNormalizationOutcome, normalize_wav_file_in_place_with_progress,
};

pub(in crate::native_app) fn sample_path_label(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

pub(in crate::native_app) fn format_copy_path(path: &Path) -> String {
    let mut rendered = path.to_string_lossy().replace('\\', "/");
    if rendered.contains(' ') {
        rendered = format!("\"{rendered}\"");
    }
    rendered
}
