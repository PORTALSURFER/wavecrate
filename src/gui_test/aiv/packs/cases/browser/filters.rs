use crate::gui_test::{GuiAivAssertion, GuiAivCase};

use super::super::super::super::{DEFAULT_VIEWPORT, GUI_TEST_WINDOW_TITLE};
use super::super::{assert_step, click_node, screenshot, wait_for_node};

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

pub(crate) fn browser_playback_age_filters_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("browser_playback_age_filters"),
        fixture_tag: String::from("browser"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("browser.playback_age_filter.never"),
            click_node("browser.playback_age_filter.month", None, None),
            assert_step(GuiAivAssertion::AssertNodeSelected {
                node_id: String::from("browser.playback_age_filter.month"),
                selected: true,
            }),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("toggle_browser_playback_age_filter"),
            }),
            screenshot("browser-playback-age-filters"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertNodeSelected {
            node_id: String::from("browser.playback_age_filter.month"),
            selected: true,
        }],
    }
}
