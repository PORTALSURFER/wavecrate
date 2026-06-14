use crate::gui_test::{GuiAivAssertion, GuiAivCase};

use super::super::super::super::{DEFAULT_VIEWPORT, GUI_TEST_WINDOW_TITLE};
use super::super::{assert_step, click_node, screenshot, wait_for_node};

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
