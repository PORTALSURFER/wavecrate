use super::*;
use crate::app::controller::library::selection_export::SelectionClipExportRequest;
use std::path::{Path, PathBuf};
use std::time::Instant;

impl AppController {
    /// Copy either the current waveform selection (as a new wav file) or the currently selected
    /// samples to the system clipboard as file drops.
    pub fn copy_selection_to_clipboard(&mut self) {
        let result = self.clipboard_paths_for_copy();
        match result {
            Ok(paths) if paths.is_empty() => {
                self.set_status("Select a sample to copy", StatusTone::Warning);
            }
            Ok(paths) => {
                if let Err(err) = crate::external_clipboard::copy_file_paths(&paths) {
                    self.set_status(err, StatusTone::Error);
                } else {
                    let label = clipboard_copy_label(&paths);
                    self.set_status(label, StatusTone::Info);
                    self.record_copy_flash();
                }
            }
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    /// Copy the status log text to the system clipboard.
    pub fn copy_status_log_to_clipboard(&mut self) {
        let text = self.ui.status.log_text();
        if text.is_empty() {
            self.set_status("Status log is empty", StatusTone::Info);
            return;
        }
        match crate::external_clipboard::copy_text(&text) {
            Ok(()) => self.set_status("Copied status log to clipboard", StatusTone::Info),
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    fn clipboard_paths_for_copy(&mut self) -> Result<Vec<PathBuf>, String> {
        let waveform_copy = self.waveform_selection_clipboard_path()?;
        if let Some(path) = waveform_copy {
            return Ok(vec![path]);
        }
        self.selected_sample_paths()
    }

    fn waveform_selection_clipboard_path(&mut self) -> Result<Option<PathBuf>, String> {
        if self.ui.focus.context != crate::app::state::FocusContext::Waveform {
            return Ok(None);
        }
        let Some(bounds) = self.selection_state.range.range() else {
            return Ok(None);
        };
        let (source_id, relative_path) = {
            let audio = self
                .sample_view
                .wav
                .loaded_audio
                .as_ref()
                .ok_or_else(|| "Load a sample before copying a selection".to_string())?;
            (audio.source_id.clone(), audio.relative_path.clone())
        };
        let clip_root = crate::app_dirs::app_root_dir()
            .map_err(|err| err.to_string())?
            .join("clipboard_clips");
        std::fs::create_dir_all(&clip_root)
            .map_err(|err| format!("Failed to create clipboard folder: {err}"))?;
        let name_hint = relative_path
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("selection.wav"));
        let entry = self.export_selection_clip_to_root(
            SelectionClipExportRequest {
                source_id: &source_id,
                relative_path: &relative_path,
                bounds,
                target_tag: None,
                add_to_browser: false,
                register_in_source: false,
            },
            &clip_root,
            &name_hint,
        )?;
        Ok(Some(clip_root.join(entry.relative_path)))
    }

    fn selected_sample_paths(&self) -> Result<Vec<PathBuf>, String> {
        let mut paths: Vec<PathBuf> = if !self.ui.browser.selection.selected_paths.is_empty() {
            let Some(source) = self.current_source() else {
                return Err("Select a source first".into());
            };
            self.ui
                .browser
                .selection
                .selected_paths
                .iter()
                .map(|p| source.root.join(p))
                .collect()
        } else if let Some(selected) = self.sample_view.wav.selected_wav.as_ref() {
            let Some(source) = self.current_source() else {
                return Err("Select a source first".into());
            };
            vec![source.root.join(selected)]
        } else if let Some(loaded) = self.sample_view.wav.loaded_audio.as_ref() {
            vec![loaded.root.join(&loaded.relative_path)]
        } else {
            Vec::new()
        };
        paths.retain(|p| p.exists());
        Ok(paths)
    }

    fn record_copy_flash(&mut self) {
        let now = Instant::now();
        self.ui.waveform.copy_flash_at = Some(now);
        let paths = self.copy_flash_paths();
        if paths.is_empty() {
            self.ui.browser.copy_flash_paths.clear();
            self.ui.browser.copy_flash_at = None;
            return;
        }
        self.ui.browser.copy_flash_paths = paths;
        self.ui.browser.copy_flash_at = Some(now);
    }

    fn copy_flash_paths(&self) -> Vec<PathBuf> {
        if !self.ui.browser.selection.selected_paths.is_empty() {
            return self.ui.browser.selection.selected_paths.clone();
        }
        if let Some(selected) = self.sample_view.wav.selected_wav.as_ref() {
            return vec![selected.clone()];
        }
        if let Some(loaded) = self.sample_view.wav.loaded_audio.as_ref() {
            return vec![loaded.relative_path.clone()];
        }
        Vec::new()
    }
}

fn display_path(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn clipboard_copy_label(paths: &[PathBuf]) -> String {
    if paths.len() == 1 {
        format!("Copied {} to clipboard", display_path(&paths[0]))
    } else {
        format!("Copied {} files to clipboard", paths.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::LoadedAudio;
    use crate::app::controller::test_support::write_test_wav;
    use crate::app::state::FocusContext;
    use crate::app_dirs::ConfigBaseGuard;
    use std::path::Path;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn copy_shortcut_exports_waveform_selection_clip_for_clipboard_paths() {
        let temp = tempdir().unwrap();
        let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
        let source_root = temp.path().join("source");
        std::fs::create_dir_all(&source_root).unwrap();

        let renderer = crate::waveform::WaveformRenderer::new(12, 12);
        let mut controller = AppController::new(renderer, None);
        let source = SampleSource::new(source_root.clone());
        controller.library.sources.push(source.clone());
        controller.selection_state.ctx.selected_source = Some(source.id.clone());

        let orig = source_root.join("clip.wav");
        write_test_wav(&orig, &[0.1, 0.2, 0.3, 0.4]);
        controller
            .load_waveform_for_selection(&source, Path::new("clip.wav"))
            .unwrap();

        controller.ui.focus.context = FocusContext::Waveform;
        controller
            .selection_state
            .range
            .set_range(Some(SelectionRange::new(0.25, 0.75)));

        let paths = controller.clipboard_paths_for_copy().unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].is_file());
        assert!(
            paths[0]
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("clip_sel"))
        );
    }

    #[test]
    fn copy_flash_paths_prefers_browser_selection() {
        let renderer = crate::waveform::WaveformRenderer::new(8, 8);
        let mut controller = AppController::new(renderer, None);
        controller.ui.browser.selection.selected_paths =
            vec![PathBuf::from("alpha.wav"), PathBuf::from("beta.wav")];
        controller.sample_view.wav.selected_wav = Some(PathBuf::from("fallback.wav"));

        let paths = controller.copy_flash_paths();

        assert_eq!(
            paths,
            vec![PathBuf::from("alpha.wav"), PathBuf::from("beta.wav")]
        );
    }

    #[test]
    fn selected_sample_paths_prefers_selected_browser_rows() {
        let temp = tempdir().unwrap();
        let source_root = temp.path().join("source");
        std::fs::create_dir_all(&source_root).unwrap();

        let renderer = crate::waveform::WaveformRenderer::new(8, 8);
        let mut controller = AppController::new(renderer, None);
        let source = SampleSource::new(source_root.clone());
        controller.library.sources.push(source.clone());
        controller.selection_state.ctx.selected_source = Some(source.id.clone());

        let alpha = source_root.join("alpha.wav");
        let beta = source_root.join("beta.wav");
        write_test_wav(&alpha, &[0.1, 0.2]);
        write_test_wav(&beta, &[0.3, 0.4]);
        controller.ui.browser.selection.selected_paths =
            vec![PathBuf::from("alpha.wav"), PathBuf::from("beta.wav")];

        let paths = controller.selected_sample_paths().unwrap();

        assert_eq!(paths, vec![alpha, beta]);
    }

    #[test]
    fn selected_sample_paths_falls_back_to_loaded_audio_file() {
        let temp = tempdir().unwrap();
        let source_root = temp.path().join("source");
        std::fs::create_dir_all(&source_root).unwrap();

        let renderer = crate::waveform::WaveformRenderer::new(8, 8);
        let mut controller = AppController::new(renderer, None);
        let source = SampleSource::new(source_root.clone());
        controller.library.sources.push(source.clone());

        let loaded_path = source_root.join("loaded.wav");
        write_test_wav(&loaded_path, &[0.1, 0.2, 0.3]);
        controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
            source_id: source.id,
            root: source_root,
            relative_path: PathBuf::from("loaded.wav"),
            bytes: Arc::from(Vec::<u8>::new()),
            duration_seconds: 0.1,
            sample_rate: 44_100,
        });

        let paths = controller.selected_sample_paths().unwrap();

        assert_eq!(paths, vec![loaded_path]);
    }
}
