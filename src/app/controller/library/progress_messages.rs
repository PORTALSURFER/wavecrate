use super::*;
use crate::app::state::ProgressTaskKind;
use crate::sample_sources::scanner::ScanMode;

const SCAN_PROGRESS_LABEL: &str = "Scanning source";
const WAV_LOAD_LABEL: &str = "Loading samples";

impl AppController {
    pub(crate) fn scan_status_label(mode: ScanMode) -> &'static str {
        match mode {
            ScanMode::Targeted => "Targeted sync",
            ScanMode::Quick => "Quick sync",
            ScanMode::Hard => "Hard sync",
        }
    }

    pub(crate) fn begin_scan_progress(&mut self, mode: ScanMode, source: &SampleSource) {
        let status_label = Self::scan_status_label(mode);
        self.set_status_message(StatusMessage::custom(
            format!("{status_label} on {}", source.root.display()),
            StatusTone::Busy,
        ));
        self.ensure_scan_progress_for_source(mode, source);
    }

    pub(crate) fn ensure_scan_progress_for_source(
        &mut self,
        mode: ScanMode,
        source: &SampleSource,
    ) {
        let should_seed_detail = !self.ui.progress.has_task(ProgressTaskKind::Scan);
        if should_seed_detail {
            self.show_status_progress(ProgressTaskKind::Scan, SCAN_PROGRESS_LABEL, 0, true);
            self.update_progress_detail_for_task(
                ProgressTaskKind::Scan,
                format!(
                    "{} • {}",
                    Self::scan_status_label(mode),
                    source.root.display()
                ),
            );
            return;
        }
        self.update_status_progress_title(ProgressTaskKind::Scan, SCAN_PROGRESS_LABEL);
    }

    pub(crate) fn ensure_wav_load_progress(&mut self, source: &SampleSource) {
        if !self.ui.progress.has_task(ProgressTaskKind::WavLoad) {
            self.show_status_progress(ProgressTaskKind::WavLoad, WAV_LOAD_LABEL, 0, false);
            self.update_progress_detail_for_task(
                ProgressTaskKind::WavLoad,
                format!("Loading wavs for {}", source.root.display()),
            );
        }
        self.set_background_status(
            format!("Loading wavs for {}", source.root.display()),
            StatusTone::Info,
        );
    }
}
