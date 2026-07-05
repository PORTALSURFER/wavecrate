use std::path::Path;

use crate::native_app::{
    app::{NativeAppState, sample_path_label},
    waveform::InstantWaveformPreview,
};

use super::deferred_drop::defer_large_drop;

impl NativeAppState {
    pub(in crate::native_app) fn start_starmap_waveform_preview(&mut self, path: &str) {
        if self.waveform.current.has_loaded_sample()
            && self.waveform.current.path() == Path::new(path)
            && !self.waveform.instant_preview_active()
        {
            return;
        }
        self.waveform.capture_starmap_drag_restore();
        if let Some(preview) = self
            .waveform
            .cache
            .instant_waveform_preview(Path::new(path))
        {
            self.replace_current_with_instant_waveform_preview(preview);
            return;
        }
        if self.waveform.instant_preview_path() != Some(Path::new(path)) {
            let previous = self
                .waveform
                .replace_current_with_instant_waveform_preview_loading(
                    Path::new(path).to_path_buf(),
                );
            defer_large_drop(previous);
        }
        self.waveform.load.label = Some(sample_path_label(path));
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
    }

    pub(in crate::native_app) fn restore_starmap_waveform_preview_after_drag(&mut self) {
        if let Some(previous) = self.waveform.restore_starmap_drag_snapshot() {
            defer_large_drop(previous);
        }
    }

    fn replace_current_with_instant_waveform_preview(&mut self, preview: InstantWaveformPreview) {
        let previous = self
            .waveform
            .replace_current_with_instant_waveform_preview(preview);
        defer_large_drop(previous);
    }
}
