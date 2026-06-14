use crate::gui_test::{GuiAivAssertion, GuiAivCase};

use super::super::super::super::{DEFAULT_VIEWPORT, GUI_TEST_WINDOW_TITLE};
use super::super::{assert_step, click_node, press_key, screenshot, type_into_node, wait_for_node};

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
