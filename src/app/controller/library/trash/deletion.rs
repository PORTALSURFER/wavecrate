use super::super::*;
use std::fs;
use tracing::warn;

impl AppController {
    /// Permanently delete the contents of the configured trash folder after confirmation.
    pub fn take_out_trash(&mut self) {
        if !self.confirm_warning(
            "Take out trash?",
            "Everything inside the trash folder will be permanently deleted. Continue?",
        ) {
            return;
        }
        let Ok(trash_root) = self.ensure_trash_folder_ready() else {
            return;
        };
        self.set_status("Deleting trash...", StatusTone::Busy);
        let mut files_removed = 0usize;
        let mut errors = Vec::new();
        let mut stack = vec![trash_root.clone()];
        let mut dirs = Vec::new();
        while let Some(dir) = stack.pop() {
            match fs::read_dir(&dir) {
                Ok(entries) => {
                    dirs.push(dir.clone());
                    for entry in entries {
                        match entry {
                            Ok(entry) => {
                                let path = entry.path();
                                if path.is_dir() {
                                    stack.push(path);
                                } else if path.is_file() {
                                    match fs::remove_file(&path) {
                                        Ok(_) => files_removed += 1,
                                        Err(err) => errors.push(format!(
                                            "Failed to delete {}: {err}",
                                            path.display()
                                        )),
                                    }
                                }
                            }
                            Err(err) => errors.push(format!("Failed to read entry: {err}")),
                        }
                    }
                }
                Err(err) => errors.push(format!(
                    "Failed to read trash folder {}: {err}",
                    dir.display()
                )),
            }
        }
        for dir in dirs.into_iter().rev() {
            if dir == trash_root {
                continue;
            }
            if let Err(err) = fs::remove_dir(&dir)
                && dir.exists()
            {
                errors.push(format!("Failed to remove folder {}: {err}", dir.display()));
            }
        }
        if errors.is_empty() {
            self.set_status(
                format!("Deleted {files_removed} file(s) from trash"),
                StatusTone::Info,
            );
        } else {
            let summary = format!(
                "Deleted {files_removed} file(s) from trash with {} error(s)",
                errors.len()
            );
            self.set_status(summary, StatusTone::Warning);
            for err in errors {
                warn!(error = %err, trash_root = %trash_root.display(), "Trash delete error");
            }
        }
    }
}
