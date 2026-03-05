use crate::app_core::actions::NativeUiAction;
use crate::app_core::app_api::state::{SampleBrowserTab, UpdateStatus};
use crate::waveform::WaveformChannelView;

use super::{AppController, AppControllerNativeRuntimeExt, WaveformRenderer};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn prepare_native_frame_animation_only_updates_fps_when_not_playing() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    assert!(controller.average_fps().is_none());
    controller.prepare_native_frame(true);
    assert!(controller.average_fps().is_none());

    sleep(Duration::from_millis(2));
    controller.prepare_native_frame(true);

    assert!(controller.average_fps().is_some());
}

/// Native seek actions should queue deferred playback commit work.
#[test]
fn apply_native_seek_queues_deferred_seek_commit() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_native_ui_action(NativeUiAction::SeekWaveform {
        position_milli: 420,
    });

    assert_eq!(controller.pending_waveform_seek_milli_for_test(), Some(420));
}

/// Dispatch groups should route representative native actions to the right handlers.
#[test]
fn apply_native_ui_action_routes_grouped_dispatch_cases() {
    enum Expected {
        BrowserSearch(&'static str),
        MapTab(SampleBrowserTab),
        LoopEnabled(bool),
        PendingSeek(Option<u16>),
        EditSelectionRange(Option<(u16, u16)>),
        UpdateStatus(UpdateStatus),
    }

    struct Case {
        label: &'static str,
        action: NativeUiAction,
        expected: Expected,
    }

    let cases = [
        Case {
            label: "browser group",
            action: NativeUiAction::SetBrowserSearch {
                query: String::from("kicks"),
            },
            expected: Expected::BrowserSearch("kicks"),
        },
        Case {
            label: "map group",
            action: NativeUiAction::SetBrowserTab { map: true },
            expected: Expected::MapTab(SampleBrowserTab::Map),
        },
        Case {
            label: "transport group",
            action: NativeUiAction::ToggleLoopPlayback,
            expected: Expected::LoopEnabled(true),
        },
        Case {
            label: "waveform group",
            action: NativeUiAction::SeekWaveform {
                position_milli: 333,
            },
            expected: Expected::PendingSeek(Some(333)),
        },
        Case {
            label: "waveform edit group",
            action: NativeUiAction::SetWaveformEditSelectionRange {
                start_milli: 125,
                end_milli: 625,
            },
            expected: Expected::EditSelectionRange(Some((125, 625))),
        },
        Case {
            label: "prompt/update group",
            action: NativeUiAction::CheckForUpdates,
            expected: Expected::UpdateStatus(UpdateStatus::Checking),
        },
    ];

    for case in cases {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        controller.apply_native_ui_action(case.action);
        match case.expected {
            Expected::BrowserSearch(expected) => {
                assert_eq!(
                    controller.ui.browser.search_query, expected,
                    "{}",
                    case.label
                );
            }
            Expected::MapTab(expected) => {
                assert_eq!(controller.ui.browser.active_tab, expected, "{}", case.label);
            }
            Expected::LoopEnabled(expected) => {
                assert_eq!(
                    controller.ui.waveform.loop_enabled, expected,
                    "{}",
                    case.label
                );
            }
            Expected::PendingSeek(expected) => {
                assert_eq!(
                    controller.pending_waveform_seek_milli_for_test(),
                    expected,
                    "{}",
                    case.label
                );
            }
            Expected::EditSelectionRange(expected) => {
                let actual = controller.ui.waveform.edit_selection.map(|range| {
                    (
                        (range.start() * 1000.0).round() as u16,
                        (range.end() * 1000.0).round() as u16,
                    )
                });
                assert_eq!(actual, expected, "{}", case.label);
            }
            Expected::UpdateStatus(expected) => {
                assert_eq!(controller.ui.update.status, expected, "{}", case.label);
            }
        }
    }
}

#[test]
/// Native folder-row focus action should select the clicked folder for filtering.
fn focus_folder_row_action_replaces_folder_selection() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = match tempdir() {
        Ok(dir) => dir,
        Err(err) => panic!("failed to create tempdir: {err}"),
    };
    let source_root = dir.path().join("source");
    let folder_path = PathBuf::from("drums");
    if let Err(err) = std::fs::create_dir_all(source_root.join(&folder_path)) {
        panic!("failed to create folder fixture: {err}");
    }

    if let Err(err) = controller.add_source_from_path(source_root) {
        panic!("failed to add source from path: {err}");
    }
    controller.select_source_by_index(0);
    controller.refresh_folder_browser_for_tests();

    let row_index = match controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == folder_path)
    {
        Some(index) => index,
        None => panic!("failed to locate folder row index"),
    };

    controller.apply_native_ui_action(NativeUiAction::FocusFolderRow { index: row_index });

    let selected = controller
        .folder_selection_for_filter()
        .cloned()
        .unwrap_or_default();
    assert_eq!(selected, [folder_path].into_iter().collect::<BTreeSet<_>>());
    assert_eq!(controller.ui.sources.folders.focused, Some(row_index));
}

#[test]
/// Native source-row reload action should route to the targeted source index.
fn reload_source_row_action_selects_target_source() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = match tempdir() {
        Ok(dir) => dir,
        Err(err) => panic!("failed to create tempdir: {err}"),
    };
    let source_a = dir.path().join("source-a");
    let source_b = dir.path().join("source-b");
    if let Err(err) = std::fs::create_dir_all(&source_a) {
        panic!("failed to create source-a fixture: {err}");
    }
    if let Err(err) = std::fs::create_dir_all(&source_b) {
        panic!("failed to create source-b fixture: {err}");
    }
    if let Err(err) = controller.add_source_from_path(source_a) {
        panic!("failed to add source-a fixture: {err}");
    }
    if let Err(err) = controller.add_source_from_path(source_b) {
        panic!("failed to add source-b fixture: {err}");
    }

    controller.select_source_by_index(0);
    controller.apply_native_ui_action(NativeUiAction::ReloadSourceRow { index: 1 });

    assert_eq!(controller.ui.sources.selected, Some(1));
}

#[test]
/// Waveform toolbar option actions should update controller waveform state.
fn apply_native_waveform_option_actions_update_waveform_state() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_native_ui_action(NativeUiAction::SetWaveformChannelView { stereo: true });
    assert_eq!(
        controller.ui.waveform.channel_view,
        WaveformChannelView::SplitStereo
    );

    controller
        .apply_native_ui_action(NativeUiAction::SetNormalizedAuditionEnabled { enabled: true });
    assert!(controller.ui.waveform.normalized_audition_enabled);

    controller.ui.waveform.bpm_value = Some(120.0);
    controller.apply_native_ui_action(NativeUiAction::AdjustWaveformBpm { delta: 1 });
    assert_eq!(controller.ui.waveform.bpm_value, Some(121.0));
    controller.apply_native_ui_action(NativeUiAction::SetWaveformBpmValue { value_tenths: 1275 });
    assert_eq!(controller.ui.waveform.bpm_value, Some(127.5));

    controller.apply_native_ui_action(NativeUiAction::SetBpmSnapEnabled { enabled: true });
    assert!(controller.ui.waveform.bpm_snap_enabled);

    controller.apply_native_ui_action(NativeUiAction::SetTransientSnapEnabled { enabled: true });
    assert!(controller.ui.waveform.transient_snap_enabled);

    controller
        .apply_native_ui_action(NativeUiAction::SetTransientMarkersEnabled { enabled: false });
    assert!(!controller.ui.waveform.transient_markers_enabled);
    assert!(!controller.ui.waveform.transient_snap_enabled);

    controller.ui.waveform.selected_slices = vec![0, 1];
    controller.apply_native_ui_action(NativeUiAction::SetSliceModeEnabled { enabled: true });
    assert!(controller.ui.waveform.slice_mode_enabled);

    controller.apply_native_ui_action(NativeUiAction::SetSliceModeEnabled { enabled: false });
    assert!(!controller.ui.waveform.slice_mode_enabled);
    assert!(controller.ui.waveform.selected_slices.is_empty());
}
