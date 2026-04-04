use super::*;

#[test]
fn projection_cache_key_changes_when_map_cache_revision_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.map.cached_points_revision += 1;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
fn projection_cache_key_changes_when_update_status_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.update.status = UpdateStatus::Checking;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
fn projection_cache_key_changes_when_options_panel_state_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.options_panel.open = true;
    controller.ui.trash_folder = Some(PathBuf::from("trash_bin"));
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
fn projection_cache_key_changes_when_audio_engine_chip_state_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.audio.output_runtime_error = Some(String::from("USB disconnected"));
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
fn projection_cache_key_changes_when_audio_picker_and_option_lists_change() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.options_panel.active_audio_picker =
        Some(crate::app::state::AudioPickerTarget::OutputHost);
    controller
        .ui
        .audio
        .hosts
        .push(crate::app::state::AudioHostView {
            id: String::from("asio"),
            label: String::from("ASIO"),
            is_default: true,
        });
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Projection cache key should change when browser filter enum encoding changes.
fn projection_cache_key_changes_when_browser_filter_encoding_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.browser.search.filter = TriageFlagFilter::Keep;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Projection cache key should change when browser sort enum encoding changes.
fn projection_cache_key_changes_when_browser_sort_encoding_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.browser.search.sort = SampleBrowserSort::PlaybackAgeAsc;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Projection cache key should change when browser tab enum encoding changes.
fn projection_cache_key_changes_when_browser_tab_encoding_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.browser.active_tab = SampleBrowserTab::Map;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}
