use super::*;

#[test]
fn apply_ui_inverted_browser_rating_filter_toggles_off_when_reclicked() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::ToggleBrowserRatingFilter {
            level: 4,
            invert: true,
        },
    ));
    assert_eq!(
        controller
            .ui
            .browser
            .search
            .rating_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![-3, -2, -1, 0, 1, 2, 3]
    );

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::ToggleBrowserRatingFilter {
            level: 4,
            invert: true,
        },
    ));
    assert!(controller.ui.browser.search.rating_filter.is_empty());
}

#[test]
fn apply_ui_locked_keep_filter_sets_only_locked_level() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::ToggleBrowserRatingFilter {
            level: 4,
            invert: false,
        },
    ));

    assert_eq!(
        controller
            .ui
            .browser
            .search
            .rating_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![4]
    );
}

#[test]
fn apply_ui_inverted_browser_playback_age_filter_toggles_off_when_reclicked() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::ToggleBrowserPlaybackAgeFilter {
            bucket: crate::app_core::actions::NativePlaybackAgeFilterChip::OlderThanWeek,
            invert: true,
        },
    ));
    assert_eq!(
        controller
            .ui
            .browser
            .search
            .playback_age_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![
            PlaybackAgeFilterChip::NeverPlayed,
            PlaybackAgeFilterChip::OlderThanMonth,
        ]
    );

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::ToggleBrowserPlaybackAgeFilter {
            bucket: crate::app_core::actions::NativePlaybackAgeFilterChip::OlderThanWeek,
            invert: true,
        },
    ));
    assert!(controller.ui.browser.search.playback_age_filter.is_empty());
}

#[test]
fn apply_ui_toggle_browser_marked_filter_updates_search_state() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::ToggleBrowserMarkedFilter,
    ));

    assert!(controller.ui.browser.search.marked_only);
}
