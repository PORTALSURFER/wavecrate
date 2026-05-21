use std::{path::Path, process};

mod wav_normalize;
pub(super) use wav_normalize::normalize_wav_file_in_place;

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
