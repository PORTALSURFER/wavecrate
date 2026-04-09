use super::super::DragDropController;
#[cfg(target_os = "windows")]
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use tracing::info;

impl DragDropController<'_> {
    #[cfg(target_os = "windows")]
    pub(crate) fn start_external_drag(&self, paths: &[PathBuf]) -> Result<(), String> {
        let hwnd = self
            .drag_hwnd
            .ok_or_else(|| "Window handle unavailable for external drag".to_string())?;
        info!(
            hwnd = ?hwnd,
            path_count = paths.len(),
            first_path = %paths
                .first()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            "drag controller: launching Windows external drag"
        );
        crate::external_drag::start_file_drag(hwnd, paths)
    }
}
