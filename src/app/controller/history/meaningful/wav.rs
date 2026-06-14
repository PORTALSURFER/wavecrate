use super::*;

/// Reversible selected and loaded wav identity state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WavLoadHistorySnapshot {
    selected_wav: Option<PathBuf>,
    loaded_wav: Option<PathBuf>,
}

pub(super) fn capture_wav_load_snapshot(controller: &AppController) -> WavLoadHistorySnapshot {
    WavLoadHistorySnapshot {
        selected_wav: controller.sample_view.wav.selected_wav.clone(),
        loaded_wav: controller.sample_view.wav.loaded_wav.clone(),
    }
}

pub(super) fn restore_wav_load_snapshot(
    controller: &mut AppController,
    snapshot: &WavLoadHistorySnapshot,
) {
    clear_loaded_wav_context(controller);
    if let Some(source) = controller.current_source() {
        restore_selected_wav(controller, snapshot);
        restore_loaded_wav(controller, &source, snapshot);
    } else {
        controller.rebuild_browser_lists();
    }
}

fn clear_loaded_wav_context(controller: &mut AppController) {
    controller.sample_view.wav.loaded_audio = None;
    controller.sample_view.wav.selected_wav = None;
    controller.sample_view.wav.loaded_wav = None;
    controller.set_ui_loaded_wav(None);
    controller.clear_focused_similarity_highlight();
}

fn restore_selected_wav(controller: &mut AppController, snapshot: &WavLoadHistorySnapshot) {
    if let Some(path) = snapshot.selected_wav.clone() {
        controller.selection_state.suppress_autoplay_once = true;
        controller.select_wav_by_path_with_rebuild(&path, true);
    } else {
        controller.rebuild_browser_lists();
    }
}

fn restore_loaded_wav(
    controller: &mut AppController,
    source: &SampleSource,
    snapshot: &WavLoadHistorySnapshot,
) {
    if let Some(path) = snapshot.loaded_wav.clone() {
        let _ = controller.queue_audio_load_for(&source, &path, AudioLoadIntent::Selection, None);
    }
}
