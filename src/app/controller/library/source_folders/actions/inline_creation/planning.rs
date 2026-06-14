use super::super::ops;
use super::*;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(super) struct FolderCreateCommand {
    pub(super) source: SampleSource,
    pub(super) relative: PathBuf,
    pub(super) destination: PathBuf,
}

pub(super) fn plan_folder_create(
    controller: &AppController,
    parent: &Path,
    name: &str,
) -> Result<FolderCreateCommand, String> {
    let folder_name = ops::normalize_folder_name(name)?;
    let source = controller
        .current_source()
        .ok_or_else(|| "Select a source first".to_string())?;
    let relative = if parent.as_os_str().is_empty() {
        PathBuf::from(&folder_name)
    } else {
        parent.join(&folder_name)
    };
    let destination = source.root.join(&relative);
    if destination.exists() {
        return Err(format!("Folder already exists: {}", relative.display()));
    }
    if controller.runtime.jobs.file_ops_in_progress() {
        return Err("File operation already in progress".to_string());
    }
    Ok(FolderCreateCommand {
        source,
        relative,
        destination,
    })
}
