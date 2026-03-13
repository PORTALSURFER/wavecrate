use super::*;

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
        BrowserSearchFocused(bool),
        BrowserRatingFilter(Vec<i8>),
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
            expected: Expected::SelectionRange(Some((125, 125))),
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
        controller.ui.browser.search_focus_requested = true;
        controller.ui.focus.context = FocusContext::Waveform;
        controller.ui.waveform.selection = Some(crate::selection::SelectionRange::new(0.2, 0.8));
        controller.ui.waveform.edit_selection =
            Some(crate::selection::SelectionRange::new(0.3, 0.7));
        controller.apply_native_ui_action(case.action);
        match case.expected {
            Expected::BrowserSearch(expected) => {
                assert_eq!(
                    controller.ui.browser.search_query, expected,
                    "{}",
                    case.label
                );
            }
            Expected::BrowserSearchFocused(expected) => {
                assert_eq!(
                    controller.ui.browser.search_focus_requested, expected,
                    "{}",
                    case.label
                );
            }
            Expected::BrowserRatingFilter(expected) => {
                assert_eq!(
                    controller
                        .ui
                        .browser
                        .rating_filter
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
                    controller.ui.browser.random_navigation_mode, expected,
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
                    controller.pending_waveform_seek_milli_for_test(),
                    expected,
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
fn apply_native_begin_waveform_selection_at_preserves_exact_anchor_with_bpm_snap() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.set_bpm_snap_enabled(true);
    controller.set_bpm_value(120.0);

    controller.apply_native_ui_action(NativeUiAction::BeginWaveformSelectionAt {
        anchor_micros: 5_000,
    });

    assert_eq!(
        controller.ui.waveform.selection,
        Some(crate::selection::SelectionRange::new(0.005, 0.005))
    );
    assert!(controller.is_selection_dragging());
}

#[test]
fn apply_native_inverted_browser_rating_filter_toggles_off_when_reclicked() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_native_ui_action(NativeUiAction::ToggleBrowserRatingFilter {
        level: 4,
        invert: true,
    });
    assert_eq!(
        controller
            .ui
            .browser
            .rating_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![-3, -2, -1, 0, 1, 2, 3]
    );

    controller.apply_native_ui_action(NativeUiAction::ToggleBrowserRatingFilter {
        level: 4,
        invert: true,
    });
    assert!(controller.ui.browser.rating_filter.is_empty());
}

#[test]
fn apply_native_locked_keep_filter_sets_only_locked_level() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_native_ui_action(NativeUiAction::ToggleBrowserRatingFilter {
        level: 4,
        invert: false,
    });

    assert_eq!(
        controller
            .ui
            .browser
            .rating_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![4]
    );
}
