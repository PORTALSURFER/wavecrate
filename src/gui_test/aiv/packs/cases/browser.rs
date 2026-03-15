use crate::gui_test::{GuiAivAssertion, GuiAivCase};

use super::super::super::{DEFAULT_VIEWPORT, GUI_TEST_WINDOW_TITLE};
use super::{
    BROWSER_SCROLL_VIEWPORT, assert_metadata_contains, assert_step, click_node, press_key,
    screenshot, scroll_in_node, type_into_node, wait_for_node,
};

pub(crate) fn browser_search_type_smoke_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_search_type_smoke"),
        fixture_tag: String::from("browser"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.search_field"),
            type_into_node("browser.search_field", "snare", true),
            assert_step(GuiAivAssertion::AssertNodeValueContains {
                node_id: String::from("browser.search_field"),
                needle: String::from("snare"),
            }),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("set_browser_search"),
            }),
            screenshot("browser-search-type-smoke"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertNodePresent {
            node_id: String::from("browser.row.0"),
        }],
    }
}

pub(crate) fn browser_search_select_commit_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_search_select_commit"),
        fixture_tag: String::from("browser"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.search_field"),
            type_into_node("browser.search_field", "snare", true),
            assert_step(GuiAivAssertion::AssertNodeValueContains {
                node_id: String::from("browser.search_field"),
                needle: String::from("snare"),
            }),
            click_node("browser.row.0", None, None),
            assert_step(GuiAivAssertion::AssertNodeSelected {
                node_id: String::from("browser.row.0"),
                selected: true,
            }),
            press_key("enter", false, false, false),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("commit_focused_browser_row"),
            }),
            screenshot("browser-search-select-commit"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertNodeSelected {
            node_id: String::from("browser.row.0"),
            selected: true,
        }],
    }
}

pub(crate) fn browser_tabs_and_rating_filters_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_tabs_and_rating_filters"),
        fixture_tag: String::from("browser"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.tab.samples"),
            click_node("browser.tab.map", None, None),
            assert_step(GuiAivAssertion::AssertNodeSelected {
                node_id: String::from("browser.tab.map"),
                selected: true,
            }),
            click_node("browser.tab.samples", None, None),
            assert_step(GuiAivAssertion::AssertNodeSelected {
                node_id: String::from("browser.tab.samples"),
                selected: true,
            }),
            click_node("browser.rating_filter.3", None, None),
            assert_step(GuiAivAssertion::AssertNodeSelected {
                node_id: String::from("browser.rating_filter.3"),
                selected: true,
            }),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("toggle_browser_rating_filter"),
            }),
            screenshot("browser-tabs-rating-filters"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertActionRecorded {
            action_id: String::from("set_browser_tab"),
        }],
    }
}

pub(crate) fn browser_map_point_focus_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_map_point_focus"),
        fixture_tag: String::from("map"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.tab.map"),
            wait_for_node("browser.map_canvas"),
            wait_for_node("browser.map.point.gui-map-source::kick_one.wav"),
            click_node("browser.map.point.gui-map-source::kick_one.wav", None, None),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("focus_map_sample"),
            }),
            assert_step(GuiAivAssertion::AssertNodeSelected {
                node_id: String::from("browser.map.point.gui-map-source::kick_one.wav"),
                selected: true,
            }),
            screenshot("browser-map-point-focus"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertNodeSelected {
            node_id: String::from("browser.map.point.gui-map-source::kick_one.wav"),
            selected: true,
        }],
    }
}

pub(crate) fn browser_interior_click_keeps_viewport_after_down_scroll_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_interior_click_keeps_viewport_after_down_scroll"),
        fixture_tag: String::from("browser"),
        viewport: BROWSER_SCROLL_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.row.18"),
            click_node("browser.row.18", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "1",
            )),
            assert_step(GuiAivAssertion::AssertNodePresent {
                node_id: String::from("browser.row.1"),
            }),
            assert_step(GuiAivAssertion::AssertNodeAbsent {
                node_id: String::from("browser.row.0"),
            }),
            screenshot("browser-interior-click-keeps-viewport-after-down-scroll"),
        ],
        expected_assertions: vec![assert_metadata_contains(
            "browser.table",
            "first_visible_row",
            "1",
        )],
    }
}

pub(crate) fn browser_interior_click_keeps_viewport_after_up_scroll_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_interior_click_keeps_viewport_after_up_scroll"),
        fixture_tag: String::from("browser"),
        viewport: BROWSER_SCROLL_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.row.20"),
            click_node("browser.row.18", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "1",
            )),
            click_node("browser.row.19", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "2",
            )),
            click_node("browser.row.20", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "3",
            )),
            screenshot("browser-interior-click-keeps-viewport-after-up-scroll"),
        ],
        expected_assertions: vec![assert_metadata_contains(
            "browser.table",
            "first_visible_row",
            "3",
        )],
    }
}

pub(crate) fn browser_refocus_after_down_scroll_keeps_single_focus_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_refocus_after_down_scroll_keeps_single_focus"),
        fixture_tag: String::from("browser"),
        viewport: BROWSER_SCROLL_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.row.18"),
            click_node("browser.row.18", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "1",
            )),
            click_node("browser.row.12", None, None),
            assert_step(GuiAivAssertion::AssertNodeSelected {
                node_id: String::from("browser.row.12"),
                selected: true,
            }),
            assert_step(GuiAivAssertion::AssertNodeSelected {
                node_id: String::from("browser.row.18"),
                selected: false,
            }),
            assert_step(assert_metadata_contains(
                "browser.row.12",
                "focused",
                "true",
            )),
            assert_step(assert_metadata_contains(
                "browser.row.18",
                "focused",
                "false",
            )),
            screenshot("browser-refocus-after-down-scroll-keeps-single-focus"),
        ],
        expected_assertions: vec![
            GuiAivAssertion::AssertNodeSelected {
                node_id: String::from("browser.row.12"),
                selected: true,
            },
            GuiAivAssertion::AssertNodeSelected {
                node_id: String::from("browser.row.18"),
                selected: false,
            },
            assert_metadata_contains("browser.row.12", "focused", "true"),
            assert_metadata_contains("browser.row.18", "focused", "false"),
        ],
    }
}

pub(crate) fn browser_wheel_scroll_uses_rendered_viewport_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_wheel_scroll_uses_rendered_viewport"),
        fixture_tag: String::from("browser"),
        viewport: BROWSER_SCROLL_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.table"),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "0",
            )),
            scroll_in_node("browser.table", -120, Some(50), Some(50)),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "1",
            )),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("set_browser_view_start"),
            }),
            scroll_in_node("browser.table", 120, Some(50), Some(50)),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "0",
            )),
            screenshot("browser-wheel-scroll-uses-rendered-viewport"),
        ],
        expected_assertions: vec![assert_metadata_contains(
            "browser.table",
            "first_visible_row",
            "0",
        )],
    }
}

pub(crate) fn browser_repeated_scroll_refocus_preserves_guard_band_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_repeated_scroll_refocus_preserves_guard_band"),
        fixture_tag: String::from("browser"),
        viewport: BROWSER_SCROLL_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.row.20"),
            click_node("browser.row.18", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "1",
            )),
            click_node("browser.row.19", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "2",
            )),
            click_node("browser.row.20", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "3",
            )),
            click_node("browser.row.15", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "3",
            )),
            click_node("browser.row.5", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "2",
            )),
            click_node("browser.row.4", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "1",
            )),
            click_node("browser.row.3", None, None),
            assert_step(assert_metadata_contains(
                "browser.table",
                "first_visible_row",
                "0",
            )),
            screenshot("browser-repeated-scroll-refocus-preserves-guard-band"),
        ],
        expected_assertions: vec![
            assert_metadata_contains("browser.table", "first_visible_row", "0"),
            GuiAivAssertion::AssertNodeSelected {
                node_id: String::from("browser.row.3"),
                selected: true,
            },
        ],
    }
}
