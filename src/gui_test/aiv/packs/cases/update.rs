use crate::gui_test::{GuiAivAssertion, GuiAivCase};

use super::super::super::{DEFAULT_VIEWPORT, GUI_TEST_WINDOW_TITLE};
use super::{assert_step, click_node, screenshot, wait_for_node};

pub(crate) fn update_panel_actions_case() -> GuiAivCase {
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
