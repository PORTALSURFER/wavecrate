use super::*;

/// Left-sidebar rating chips route through the source hit-test path.
#[test]
fn left_sidebar_rating_chip_routes_browser_filter_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = populated_single_sidebar_model();
    let mut state = NativeShellState::new();
    let chip = state
        .sidebar_rating_filter_chip_rect(&layout, &model, 3)
        .expect("left-sidebar rating chip should exist");

    assert_eq!(
        state.source_action_at_point(&layout, &model, chip.center()),
        Some(
            crate::app_core::native_shell::runtime_contract::UiAction::ToggleBrowserRatingFilter {
                level: 3,
                invert: false,
            }
        )
    );
}

/// Left-sidebar metadata filter rows open dropdowns that route sidebar filter actions.
#[test]
fn left_sidebar_metadata_filter_dropdowns_route_browser_filter_actions() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = populated_single_sidebar_model();
    let mut state = NativeShellState::new();

    let expected = [
        (
            0,
            0,
            crate::app_core::app_api::state::BrowserSidebarFilterOption::Format(
                crate::app_core::app_api::state::BrowserFormatFacet::Wav,
            ),
        ),
        (
            1,
            0,
            crate::app_core::app_api::state::BrowserSidebarFilterOption::BitDepth(
                crate::app_core::app_api::state::BrowserBitDepthFacet::Unavailable,
            ),
        ),
        (
            2,
            3,
            crate::app_core::app_api::state::BrowserSidebarFilterOption::Channels(
                crate::app_core::app_api::state::BrowserChannelFacet::Unavailable,
            ),
        ),
        (
            3,
            0,
            crate::app_core::app_api::state::BrowserSidebarFilterOption::Bpm(
                crate::app_core::app_api::state::BrowserBpmFacet::Unknown,
            ),
        ),
        (
            4,
            0,
            crate::app_core::app_api::state::BrowserSidebarFilterOption::Key(
                crate::app_core::app_api::state::BrowserKeyFacet::Unknown,
            ),
        ),
    ];

    for (row_index, option_index, option) in expected {
        let row = state
            .sidebar_filter_row_rect(&layout, &model, row_index)
            .expect("left-sidebar filter row should exist");
        assert_eq!(
            state.source_action_at_point(&layout, &model, row.center()),
            Some(crate::app_core::native_shell::runtime_contract::UiAction::FocusBrowserPanel)
        );
        assert!(state.sidebar_filter_dropdown_visible());
        let option_rect = state
            .sidebar_filter_dropdown_option_rect(&layout, &model, option_index)
            .expect("left-sidebar filter dropdown option should exist");
        assert_eq!(
            state.sidebar_filter_dropdown_action_at_point(
                &layout,
                &model,
                option_rect.center()
            ),
            Some(
                crate::app_core::native_shell::runtime_contract::UiAction::ToggleBrowserSidebarFilter {
                    option,
                    additive: true,
                }
            )
        );
    }
}

/// Left-sidebar filter dropdowns expose clear actions for active facets.
#[test]
fn left_sidebar_filter_dropdown_clear_routes_browser_filter_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = populated_single_sidebar_model();
    model.sidebar_filters.toggle(
        crate::app_core::app_api::state::BrowserSidebarFilterOption::Bpm(
            crate::app_core::app_api::state::BrowserBpmFacet::Mid,
        ),
        true,
    );
    let mut state = NativeShellState::new();
    let row = state
        .sidebar_filter_row_rect(&layout, &model, 3)
        .expect("left-sidebar BPM filter row should exist");

    assert_eq!(
        state.source_action_at_point(&layout, &model, row.center()),
        Some(crate::app_core::native_shell::runtime_contract::UiAction::FocusBrowserPanel)
    );
    let clear_rect = state
        .sidebar_filter_dropdown_option_rect(&layout, &model, 4)
        .expect("left-sidebar filter dropdown clear option should exist");

    assert_eq!(
        state.sidebar_filter_dropdown_action_at_point(&layout, &model, clear_rect.center()),
        Some(
            crate::app_core::native_shell::runtime_contract::UiAction::ClearBrowserSidebarFilter {
                facet: crate::app_core::app_api::state::BrowserSidebarFilterFacet::Bpm,
            }
        )
    );
}
