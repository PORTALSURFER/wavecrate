//! Meaningful UI snapshots composed from source, browser, wav, and waveform state.

use super::*;

mod browser;
mod source;
mod wav;
mod waveform;

use browser::BrowserHistorySnapshot;
use source::SourceFolderHistorySnapshot;
use wav::WavLoadHistorySnapshot;
use waveform::WaveformHistorySnapshot;

/// Reversible subset of controller state that represents meaningful UI context.
/// This intentionally excludes transient playback helpers such as compare-anchor state.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MeaningfulUiSnapshot {
    source: SourceFolderHistorySnapshot,
    browser: BrowserHistorySnapshot,
    wav: WavLoadHistorySnapshot,
    waveform: WaveformHistorySnapshot,
}

pub(super) fn capture_meaningful_ui_snapshot(controller: &AppController) -> MeaningfulUiSnapshot {
    MeaningfulUiSnapshot {
        source: source::capture_source_folder_snapshot(controller),
        browser: browser::capture_browser_snapshot(controller),
        wav: wav::capture_wav_load_snapshot(controller),
        waveform: waveform::capture_waveform_snapshot(controller),
    }
}

pub(super) fn restore_meaningful_ui_snapshot(
    controller: &mut AppController,
    snapshot: &MeaningfulUiSnapshot,
) {
    source::restore_source_folder_snapshot(controller, &snapshot.source);
    wav::restore_wav_load_snapshot(controller, &snapshot.wav);
    browser::restore_browser_snapshot(controller, &snapshot.browser);
    waveform::restore_waveform_snapshot(controller, &snapshot.waveform);
}
