use super::*;
use telemetry::record_source_lifecycle_event;

impl AppController {
    /// Open the source root in the OS file explorer.
    pub fn open_source_folder(&mut self, index: usize) {
        let started_at = Instant::now();
        let Some(source) = self.library.sources.get(index) else {
            self.set_status("Source not found", StatusTone::Error);
            record_source_lifecycle_event(
                "sources.open_folder",
                None,
                "error",
                started_at,
                Some("source_not_found"),
            );
            return;
        };
        let source_id = source.id.as_str().to_string();
        let source_root = source.root.clone();
        if !source_root.exists() {
            self.set_status(
                format!("Source folder missing: {}", source_root.display()),
                StatusTone::Warning,
            );
            record_source_lifecycle_event(
                "sources.open_folder",
                Some(&source_id),
                "error",
                started_at,
                Some("source_root_missing"),
            );
            return;
        }
        if let Err(err) = open::that(&source_root) {
            self.set_status(
                format!("Could not open folder {}: {err}", source_root.display()),
                StatusTone::Error,
            );
            let error = err.to_string();
            record_source_lifecycle_event(
                "sources.open_folder",
                Some(&source_id),
                "error",
                started_at,
                Some(&error),
            );
            return;
        }
        record_source_lifecycle_event(
            "sources.open_folder",
            Some(&source_id),
            "success",
            started_at,
            None,
        );
    }
}
