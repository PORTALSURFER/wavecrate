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
    controller.ui.audio.applied = Some(crate::app_core::app_api::state::ActiveAudioOutput {
        host_id: String::from("asio"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: Some(256),
        channel_count: 2,
    });
    let first = build_projection_cache_key(&controller);
    controller.ui.audio.output_runtime_error = Some(String::from("USB disconnected"));
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
fn projection_cache_key_changes_when_audio_picker_and_option_lists_change() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.options_panel.open = true;
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

#[test]
/// Projection cache key should change when sidebar metadata revisions change.
fn projection_cache_key_changes_when_browser_sidebar_metadata_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.browser.tag_sidebar_open = true;
    let first = build_projection_cache_key(&controller);

    controller.mark_browser_row_metadata_projection_revision_dirty();
    let _ = controller.refresh_projection_revision_bus();

    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Projection cache key should change when the focused sidebar target swaps at the same count.
fn projection_cache_key_changes_when_browser_sidebar_focus_target_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.browser.tag_sidebar_open = true;
    controller.ui.browser.selection.last_focused_path = Some(PathBuf::from("first.wav"));
    let first = build_projection_cache_key(&controller);

    controller.ui.browser.selection.last_focused_path = Some(PathBuf::from("second.wav"));

    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Projection cache key should change when selected-visible fallback swaps sidebar targets.
fn projection_cache_key_changes_when_browser_sidebar_selected_visible_target_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.browser.tag_sidebar_open = true;
    controller.ui.browser.viewport.visible =
        crate::app_core::app_api::state::VisibleRows::All { total: 2 };
    controller.ui.browser.selection.selected_visible = Some(0);
    controller.set_wav_entries_for_tests(vec![
        crate::sample_sources::WavEntry {
            relative_path: PathBuf::from("first.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: None,
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            sound_type: None,
            locked: false,
            missing: false,
            last_played_at: None,
            user_tag: None,
        },
        crate::sample_sources::WavEntry {
            relative_path: PathBuf::from("second.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: None,
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            sound_type: None,
            locked: false,
            missing: false,
            last_played_at: None,
            user_tag: None,
        },
    ]);
    let first = build_projection_cache_key(&controller);

    controller.ui.browser.selection.selected_visible = Some(1);

    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}
