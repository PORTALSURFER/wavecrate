use super::*;
use crate::app::controller::{
    startup_audio_refresh_count_for_tests, with_stubbed_startup_audio_refresh_for_tests,
};
use crate::app_core::controller::NativeFramePreparationPlan;

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

#[test]
fn browser_retained_pull_plan_skips_startup_lanes() {
    with_stubbed_startup_audio_refresh_for_tests(|| {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        controller
            .apply_configuration(crate::sample_sources::config::AppConfig::default())
            .expect("apply startup config");

        controller.prepare_native_frame_with_plan(NativeFramePreparationPlan::BrowserRetainedPull);
        controller.prepare_native_frame_with_plan(NativeFramePreparationPlan::BrowserRetainedPull);

        assert!(controller.has_pending_startup_audio_refresh());
        assert_eq!(startup_audio_refresh_count_for_tests(), 0);
    });
}

#[test]
fn full_pull_plan_runs_startup_lanes() {
    with_stubbed_startup_audio_refresh_for_tests(|| {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        controller
            .apply_configuration(crate::sample_sources::config::AppConfig::default())
            .expect("apply startup config");

        controller.prepare_native_frame_with_plan(NativeFramePreparationPlan::Full);
        controller.prepare_native_frame_with_plan(NativeFramePreparationPlan::Full);

        assert!(!controller.has_pending_startup_audio_refresh());
        assert_eq!(startup_audio_refresh_count_for_tests(), 1);
    });
}

/// Native seek actions should queue deferred playback commit work.
#[test]
fn apply_native_seek_queues_deferred_seek_commit() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_native_ui_action(NativeUiAction::SeekWaveform {
        position_milli: 420,
    });

    assert_eq!(
        controller.pending_waveform_seek_nanos_for_test(),
        Some(420_000_000)
    );
}

/// Precise native seek actions should preserve nanounit targets.
#[test]
fn apply_native_precise_seek_queues_exact_deferred_seek_commit() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_native_ui_action(NativeUiAction::SeekWaveformPrecise {
        position_nanos: 420_123_456,
    });

    assert_eq!(
        controller.pending_waveform_seek_nanos_for_test(),
        Some(420_123_456)
    );
}

/// Dispatch groups should route representative native actions to the right handlers.
#[test]
fn apply_native_ui_action_routes_grouped_dispatch_cases() {
    enum Expected {
        BrowserSearch(&'static str),
        BrowserSearchFocused(bool),
        BrowserRatingFilter(Vec<i8>),
        BrowserPlaybackAgeFilter(Vec<crate::app::state::PlaybackAgeFilterChip>),
        RandomNavigationMode(bool),
        MapTab(SampleBrowserTab),
        LoopEnabled(bool),
        OptionsPanelOpen(bool),
        InputMonitoring(bool),
        PendingSeek(Option<u16>),
        SelectionRange(Option<(u16, u16)>),
        EditSelectionRange(Option<(u16, u16)>),
        BothSelectionRangesCleared,
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
            label: "browser blur group",
            action: NativeUiAction::BlurBrowserSearch,
            expected: Expected::BrowserSearchFocused(false),
        },
        Case {
            label: "browser rating filter group",
            action: NativeUiAction::ToggleBrowserRatingFilter {
                level: 3,
                invert: false,
            },
            expected: Expected::BrowserRatingFilter(vec![3]),
        },
        Case {
            label: "browser rating invert group",
            action: NativeUiAction::ToggleBrowserRatingFilter {
                level: 4,
                invert: true,
            },
            expected: Expected::BrowserRatingFilter(vec![-3, -2, -1, 0, 1, 2, 3]),
        },
        Case {
            label: "browser playback-age invert group",
            action: NativeUiAction::ToggleBrowserPlaybackAgeFilter {
                bucket: crate::app_core::actions::NativePlaybackAgeFilterChip::OlderThanWeek,
                invert: true,
            },
            expected: Expected::BrowserPlaybackAgeFilter(vec![
                crate::app::state::PlaybackAgeFilterChip::NeverPlayed,
                crate::app::state::PlaybackAgeFilterChip::OlderThanMonth,
            ]),
        },
        Case {
            label: "browser random toggle group",
            action: NativeUiAction::ToggleRandomNavigationMode,
            expected: Expected::RandomNavigationMode(true),
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
            label: "options panel group",
            action: NativeUiAction::OpenOptionsMenu,
            expected: Expected::OptionsPanelOpen(true),
        },
        Case {
            label: "options toggle group",
            action: NativeUiAction::SetInputMonitoringEnabled { enabled: false },
            expected: Expected::InputMonitoring(false),
        },
        Case {
            label: "waveform group",
            action: NativeUiAction::SeekWaveform {
                position_milli: 333,
            },
            expected: Expected::PendingSeek(Some(333)),
        },
        Case {
            label: "waveform begin selection group",
            action: NativeUiAction::BeginWaveformSelectionAt {
                anchor_micros: 125_000,
            },
            expected: Expected::SelectionRange(Some((200, 800))),
        },
        Case {
            label: "waveform edit group",
            action: NativeUiAction::SetWaveformEditSelectionRange {
                start_micros: 125_000,
                end_micros: 625_000,
                preserve_view_edge: false,
            },
            expected: Expected::EditSelectionRange(Some((125, 625))),
        },
        Case {
            label: "waveform clear both group",
            action: NativeUiAction::ClearWaveformSelections,
            expected: Expected::BothSelectionRangesCleared,
        },
        Case {
            label: "prompt/update group",
            action: NativeUiAction::CheckForUpdates,
            expected: Expected::UpdateStatus(UpdateStatus::Checking),
        },
    ];

    for case in cases {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        controller.ui.browser.search.search_focus_requested = true;
        controller.ui.focus.context = FocusContext::Waveform;
        controller.ui.waveform.selection = Some(crate::selection::SelectionRange::new(0.2, 0.8));
        controller.ui.waveform.edit_selection =
            Some(crate::selection::SelectionRange::new(0.3, 0.7));
        controller.apply_native_ui_action(case.action);
        match case.expected {
            Expected::BrowserSearch(expected) => {
                assert_eq!(
                    controller.ui.browser.search.search_query, expected,
                    "{}",
                    case.label
                );
            }
            Expected::BrowserSearchFocused(expected) => {
                assert_eq!(
                    controller.ui.browser.search.search_focus_requested, expected,
                    "{}",
                    case.label
                );
            }
            Expected::BrowserRatingFilter(expected) => {
                assert_eq!(
                    controller
                        .ui
                        .browser
                        .search
                        .rating_filter
                        .iter()
                        .copied()
                        .collect::<Vec<_>>(),
                    expected,
                    "{}",
                    case.label
                );
            }
            Expected::BrowserPlaybackAgeFilter(expected) => {
                assert_eq!(
                    controller
                        .ui
                        .browser
                        .search
                        .playback_age_filter
                        .iter()
                        .copied()
                        .collect::<Vec<_>>(),
                    expected,
                    "{}",
                    case.label
                );
            }
            Expected::RandomNavigationMode(expected) => {
                assert_eq!(
                    controller.ui.browser.search.random_navigation_mode, expected,
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
            Expected::OptionsPanelOpen(expected) => {
                assert_eq!(controller.ui.options_panel.open, expected, "{}", case.label);
            }
            Expected::InputMonitoring(expected) => {
                assert_eq!(
                    controller.ui.controls.input_monitoring_enabled, expected,
                    "{}",
                    case.label
                );
            }
            Expected::PendingSeek(expected) => {
                assert_eq!(
                    controller.pending_waveform_seek_nanos_for_test(),
                    expected.map(|value| u32::from(value) * 1_000_000),
                    "{}",
                    case.label
                );
            }
            Expected::SelectionRange(expected) => {
                let actual = controller.ui.waveform.selection.map(|range| {
                    (
                        (range.start() * 1000.0).round() as u16,
                        (range.end() * 1000.0).round() as u16,
                    )
                });
                assert_eq!(actual, expected, "{}", case.label);
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
            Expected::BothSelectionRangesCleared => {
                assert!(controller.ui.waveform.selection.is_none(), "{}", case.label);
                assert!(
                    controller.ui.waveform.edit_selection.is_none(),
                    "{}",
                    case.label
                );
            }
            Expected::UpdateStatus(expected) => {
                assert_eq!(controller.ui.update.status, expected, "{}", case.label);
            }
        }
    }
}

#[test]
fn apply_native_begin_waveform_selection_at_arms_drag_without_visible_selection() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.set_bpm_snap_enabled(true);
    controller.set_bpm_value(120.0);

    controller.apply_native_ui_action(NativeUiAction::BeginWaveformSelectionAt {
        anchor_micros: 5_000,
    });

    assert!(controller.ui.waveform.selection.is_none());
    assert!(controller.is_selection_dragging());
}

#[test]
fn apply_native_loop_lock_cycles_locked_loop_override() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_native_ui_action(NativeUiAction::ToggleLoopLock);
    assert!(controller.ui.waveform.loop_lock_enabled);
    assert!(controller.ui.waveform.loop_enabled);

    controller.apply_native_ui_action(NativeUiAction::ToggleLoopLock);
    assert!(controller.ui.waveform.loop_lock_enabled);
    assert!(!controller.ui.waveform.loop_enabled);
}
