use crate::gui_test::{GuiAivAssertion, GuiAivCase};

use super::super::super::{DEFAULT_VIEWPORT, GUI_TEST_WINDOW_TITLE};
use super::{assert_step, screenshot, wait_for_node};

pub(crate) fn startup_ready_case() -> GuiAivCase {
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
