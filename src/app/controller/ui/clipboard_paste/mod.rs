use super::*;
use crate::app::controller::jobs::{FileOpMessage, FileOpResult};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

mod source_job;

/// Background clipboard paste/import request with stable status labels.
struct ClipboardPasteJob {
    kind: ClipboardPasteJobKind,
    paths: Vec<PathBuf>,
    action_label: &'static str,
    action_progress: &'static str,
    action_past_tense: &'static str,
    target_label: String,
}

/// Target-specific payload for a clipboard paste/import job.
enum ClipboardPasteJobKind {
    /// Paste or import files into one source folder.
    Source(SourceClipboardPasteJob),
}

/// Destination details for a source paste/import job.
#[derive(Clone)]
struct SourceClipboardPasteJob {
    source_id: SourceId,
    source_root: PathBuf,
    target_folder: PathBuf,
}

impl AppController {
    /// Paste file paths from the system clipboard into the active source.
    pub fn paste_files_from_clipboard(&mut self) -> bool {
        let paths = match read_clipboard_paths() {
            Ok(Some(paths)) => paths,
            Ok(None) => return false,
            Err(err) => {
                self.set_status(err, StatusTone::Error);
                return true;
            }
        };
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status(
                "Another file operation is already running",
                StatusTone::Warning,
            );
            return true;
        }
        let Some(source) = self.current_source() else {
            self.set_status("Select a source first", StatusTone::Info);
            return true;
        };
        let job = ClipboardPasteJob {
            kind: ClipboardPasteJobKind::Source(SourceClipboardPasteJob {
                source_id: source.id,
                source_root: source.root,
                target_folder: PathBuf::new(),
            }),
            paths,
            action_label: "paste",
            action_progress: "Pasting",
            action_past_tense: "Pasted",
            target_label: "source".to_string(),
        };
        self.begin_clipboard_paste_job(job, "Pasting files");
        true
    }

    /// Import external audio files into the active source folder.
    pub(crate) fn import_external_files_to_source_folder(
        &mut self,
        target_folder: PathBuf,
        paths: Vec<PathBuf>,
    ) {
        if paths.is_empty() {
            return;
        }
        let Some(source) = self.current_source() else {
            self.set_status("Select a source first", StatusTone::Info);
            return;
        };
        if let Err(err) = validate_relative_folder_path(&target_folder) {
            self.set_status(err, StatusTone::Error);
            return;
        }
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status(
                "Another file operation is already running",
                StatusTone::Warning,
            );
            return;
        }
        let job = import_source_clipboard_paste_job(source.id, source.root, target_folder, paths);
        self.begin_clipboard_paste_job(job, "Importing files");
    }

    fn begin_clipboard_paste_job(&mut self, job: ClipboardPasteJob, title: &str) {
        if job.paths.is_empty() {
            self.set_status(
                format!("No files to {}", job.action_label),
                StatusTone::Warning,
            );
            return;
        }
        let total = job.paths.len();
        self.set_status(format!("{title}..."), StatusTone::Busy);
        self.show_status_progress(
            crate::app::state::ProgressTaskKind::FileOps,
            title,
            total,
            true,
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.runtime.jobs.start_file_ops(rx, cancel.clone());
        std::thread::spawn(move || {
            let result = source_job::run_clipboard_paste_job(job, cancel, Some(&tx));
            let _ = tx.send(FileOpMessage::Finished(FileOpResult::ClipboardPaste(
                result,
            )));
        });
    }
}

#[cfg(test)]
impl AppController {
    /// Import external audio files synchronously for tests without spawning background jobs.
    pub(crate) fn import_external_files_to_source_folder_for_tests(
        &mut self,
        target_folder: PathBuf,
        paths: Vec<PathBuf>,
    ) -> Result<crate::app::controller::jobs::ClipboardPasteResult, String> {
        if paths.is_empty() {
            return Err("No files to import".into());
        }
        let Some(source) = self.current_source() else {
            return Err("Select a source first".into());
        };
        validate_relative_folder_path(&target_folder)?;
        if self.runtime.jobs.file_ops_in_progress() {
            return Err("Another file operation is already running".into());
        }
        let job = import_source_clipboard_paste_job(source.id, source.root, target_folder, paths);
        Ok(source_job::run_clipboard_paste_job(
            job,
            Arc::new(AtomicBool::new(false)),
            None,
        ))
    }
}

fn import_source_clipboard_paste_job(
    source_id: SourceId,
    source_root: PathBuf,
    target_folder: PathBuf,
    paths: Vec<PathBuf>,
) -> ClipboardPasteJob {
    let target_label = if target_folder.as_os_str().is_empty() {
        "source root".to_string()
    } else {
        format!("folder {}", target_folder.display())
    };
    ClipboardPasteJob {
        kind: ClipboardPasteJobKind::Source(SourceClipboardPasteJob {
            source_id,
            source_root,
            target_folder,
        }),
        paths,
        action_label: "import",
        action_progress: "Importing",
        action_past_tense: "Imported",
        target_label,
    }
}

fn read_clipboard_paths() -> Result<Option<Vec<PathBuf>>, String> {
    let paths = crate::external_clipboard::read_file_paths()?;
    if paths.is_empty() {
        Ok(None)
    } else {
        Ok(Some(paths))
    }
}

fn unique_destination_name(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| "File has no name".to_string())?;
    let candidate = PathBuf::from(file_name);
    if !root.join(&candidate).exists() {
        return Ok(candidate);
    }
    let stem = path
        .file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "sample".to_string());
    let extension = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_string());
    for index in 1..=999 {
        let suffix = format!("{stem}_copy{index:03}");
        let file_name = if let Some(ext) = &extension {
            format!("{suffix}.{ext}")
        } else {
            suffix
        };
        let candidate = PathBuf::from(file_name);
        if !root.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err("Unable to find a unique destination name".into())
}

fn validate_relative_folder_path(path: &Path) -> Result<(), String> {
    if path.is_absolute() {
        return Err("Target folder must be a relative path".into());
    }
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("Target folder cannot contain '..'".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_relative_folder_path_blocks_parent_dirs() {
        assert!(validate_relative_folder_path(Path::new("..")).is_err());
        assert!(validate_relative_folder_path(Path::new("foo/../bar")).is_err());
    }

    #[test]
    fn validate_relative_folder_path_allows_relative() {
        assert!(validate_relative_folder_path(Path::new("")).is_ok());
        assert!(validate_relative_folder_path(Path::new("samples/drums")).is_ok());
    }
}
