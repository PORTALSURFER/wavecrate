//! Concrete desktop GUI case catalog used by the named AIV packs.

use crate::gui_test::{GuiAivAssertion, GuiAivCase, GuiAivStep};

use super::super::{DEFAULT_VIEWPORT, GUI_TEST_WINDOW_TITLE};

const BROWSER_SCROLL_VIEWPORT: [u32; 2] = [1280, 720];

pub(super) fn startup_ready_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("startup_ready"),
        fixture_tag: String::from("default"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("shell.top_bar.options_button"),
            wait_for_node("browser.panel"),
            assert_step(GuiAivAssertion::AssertNodePresent {
                node_id: String::from("shell.root"),
            }),
            screenshot("startup-ready"),
        ],
        expected_assertions: vec![
            GuiAivAssertion::AssertNodePresent {
                node_id: String::from("shell.root"),
            },
            GuiAivAssertion::AssertNodePresent {
                node_id: String::from("shell.top_bar.options_button"),
            },
        ],
    }
}

pub(super) fn browser_search_type_smoke_case() -> GuiAivCase {
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

pub(super) fn browser_search_select_commit_case() -> GuiAivCase {
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

pub(super) fn browser_tabs_and_rating_filters_case() -> GuiAivCase {
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

pub(super) fn browser_interior_click_keeps_viewport_after_down_scroll_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_interior_click_keeps_viewport_after_down_scroll"),
        fixture_tag: String::from("browser"),
        viewport: BROWSER_SCROLL_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.row.18"),
            click_node("browser.row.18", None, None),
            assert_step(assert_metadata_contains("browser.table", "first_visible_row", "1")),
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

pub(super) fn browser_interior_click_keeps_viewport_after_up_scroll_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_interior_click_keeps_viewport_after_up_scroll"),
        fixture_tag: String::from("browser"),
        viewport: BROWSER_SCROLL_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.row.20"),
            click_node("browser.row.18", None, None),
            assert_step(assert_metadata_contains("browser.table", "first_visible_row", "1")),
            click_node("browser.row.19", None, None),
            assert_step(assert_metadata_contains("browser.table", "first_visible_row", "2")),
            click_node("browser.row.20", None, None),
            assert_step(assert_metadata_contains("browser.table", "first_visible_row", "3")),
            screenshot("browser-interior-click-keeps-viewport-after-up-scroll"),
        ],
        expected_assertions: vec![assert_metadata_contains(
            "browser.table",
            "first_visible_row",
            "3",
        )],
    }
}

pub(super) fn options_open_close_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("options_open_close"),
        fixture_tag: String::from("default"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("shell.top_bar.options_button"),
            click_node("shell.top_bar.options_button", None, None),
            assert_step(GuiAivAssertion::AssertNodePresent {
                node_id: String::from("overlay.options_panel"),
            }),
            click_node("overlay.options_panel.close", None, None),
            assert_step(GuiAivAssertion::AssertNodeAbsent {
                node_id: String::from("overlay.options_panel"),
            }),
            screenshot("options-open-close"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertActionRecorded {
            action_id: String::from("close_options_panel"),
        }],
    }
}

pub(super) fn prompt_confirm_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("prompt_confirm"),
        fixture_tag: String::from("prompt"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("overlay.prompt.input"),
            type_into_node("overlay.prompt.input", "kick_aiv_confirm.wav", true),
            assert_step(GuiAivAssertion::AssertNodeValueContains {
                node_id: String::from("overlay.prompt.input"),
                needle: String::from("kick_aiv_confirm"),
            }),
            click_node("overlay.prompt.confirm", None, None),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("confirm_prompt"),
            }),
            assert_step(GuiAivAssertion::AssertNodeAbsent {
                node_id: String::from("overlay.prompt"),
            }),
            screenshot("prompt-confirm"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertNodeAbsent {
            node_id: String::from("overlay.prompt"),
        }],
    }
}

pub(super) fn prompt_cancel_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("prompt_cancel"),
        fixture_tag: String::from("prompt"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("overlay.prompt.cancel"),
            click_node("overlay.prompt.cancel", None, None),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("cancel_prompt"),
            }),
            assert_step(GuiAivAssertion::AssertNodeAbsent {
                node_id: String::from("overlay.prompt"),
            }),
            screenshot("prompt-cancel"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertNodeAbsent {
            node_id: String::from("overlay.prompt"),
        }],
    }
}

pub(super) fn waveform_transport_cursor_selection_zoom_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("waveform_transport_cursor_selection_zoom"),
        fixture_tag: String::from("waveform"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("waveform.region"),
            click_node("waveform.toolbar.loop", None, None),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("toggle_loop_playback"),
            }),
            click_node("waveform.region", Some(80), Some(50)),
            drag_in_node("waveform.region", 62, 50, 88, 50),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("set_waveform_selection_range"),
            }),
            scroll_in_node("waveform.region", 240, Some(80), Some(50)),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("zoom_waveform"),
            }),
            screenshot("waveform-transport-cursor-selection-zoom"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertNodeSelected {
            node_id: String::from("waveform.toolbar.loop"),
            selected: true,
        }],
    }
}

pub(super) fn update_panel_actions_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("update_panel_actions"),
        fixture_tag: String::from("update"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("shell.top_bar.update_panel"),
            assert_step(GuiAivAssertion::AssertNodePresent {
                node_id: String::from("shell.top_bar.update.open"),
            }),
            click_node("shell.top_bar.update.open", None, None),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("open_update_link"),
            }),
            click_node("shell.top_bar.update.dismiss", None, None),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("dismiss_update"),
            }),
            screenshot("update-panel-actions"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertNodeAbsent {
            node_id: String::from("shell.top_bar.update.dismiss"),
        }],
    }
}

fn wait_for_node(node_id: &str) -> GuiAivStep {
    GuiAivStep::WaitForNode {
        node_id: String::from(node_id),
        timeout_ms: 15_000,
    }
}

fn click_node(node_id: &str, x_percent: Option<u8>, y_percent: Option<u8>) -> GuiAivStep {
    GuiAivStep::ClickNode {
        node_id: String::from(node_id),
        x_percent,
        y_percent,
    }
}

fn type_into_node(node_id: &str, text: &str, clear_existing: bool) -> GuiAivStep {
    GuiAivStep::TypeIntoNode {
        node_id: String::from(node_id),
        text: String::from(text),
        clear_existing,
    }
}

fn press_key(key: &str, ctrl: bool, alt: bool, shift: bool) -> GuiAivStep {
    GuiAivStep::PressKey {
        key: String::from(key),
        ctrl,
        alt,
        shift,
    }
}

fn drag_in_node(
    node_id: &str,
    start_x_percent: u8,
    start_y_percent: u8,
    end_x_percent: u8,
    end_y_percent: u8,
) -> GuiAivStep {
    GuiAivStep::DragInNode {
        node_id: String::from(node_id),
        start_x_percent,
        start_y_percent,
        end_x_percent,
        end_y_percent,
    }
}

fn scroll_in_node(
    node_id: &str,
    delta: i32,
    x_percent: Option<u8>,
    y_percent: Option<u8>,
) -> GuiAivStep {
    GuiAivStep::ScrollInNode {
        node_id: String::from(node_id),
        delta,
        x_percent,
        y_percent,
    }
}

fn screenshot(label: &str) -> GuiAivStep {
    GuiAivStep::CaptureScreenshot {
        label: String::from(label),
    }
}

fn assert_step(assertion: GuiAivAssertion) -> GuiAivStep {
    GuiAivStep::Assert { assertion }
}

fn assert_metadata_contains(node_id: &str, key: &str, needle: &str) -> GuiAivAssertion {
    GuiAivAssertion::AssertNodeMetadataContains {
        node_id: String::from(node_id),
        key: String::from(key),
        needle: String::from(needle),
    }
}
