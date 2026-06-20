use std::path::PathBuf;

pub(super) fn copy_file_paths(_paths: &[PathBuf]) -> Result<(), String> {
    Err("Clipboard file copy is only implemented on Windows and macOS in this build".into())
}

pub(super) fn copy_text(_text: &str) -> Result<(), String> {
    Err("Clipboard text copy is only implemented on Windows and macOS in this build".into())
}

pub(super) fn read_file_paths() -> Result<Vec<PathBuf>, String> {
    Err("Clipboard file paste is only implemented on Windows and macOS in this build".into())
}

pub(super) fn read_text() -> Result<String, String> {
    Err("Clipboard text read is only implemented on Windows and macOS in this build".into())
}
