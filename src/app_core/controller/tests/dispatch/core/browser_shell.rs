use super::*;

#[test]
fn apply_ui_action_routes_browser_search_and_shell_focus_cases() {
    let mut controller = controller_for_grouped_dispatch();

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::SetBrowserSearch {
            query: String::from("kicks"),
        },
    ));
    assert_eq!(controller.ui.browser.search.search_query, "kicks");

    controller.apply_ui_action(NativeUiAction::Shell(
        crate::app_core::actions::NativeShellAction::BlurBrowserSearch,
    ));
    assert!(!controller.ui.browser.search.search_focus_requested);
}

#[test]
fn apply_ui_action_routes_browser_filter_cases() {
    let mut controller = controller_for_grouped_dispatch();

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::ToggleBrowserRatingFilter {
            level: 3,
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
        vec![3]
    );

    let mut controller = controller_for_grouped_dispatch();
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

    let mut controller = controller_for_grouped_dispatch();
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
}

#[test]
fn apply_ui_action_routes_browser_random_and_map_cases() {
    let mut controller = controller_for_grouped_dispatch();

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::ToggleRandomNavigationMode,
    ));
    assert!(controller.ui.browser.search.random_navigation_mode);

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::SetBrowserTab { map: true },
    ));
    assert_eq!(controller.ui.browser.active_tab, SampleBrowserTab::Map);
}
