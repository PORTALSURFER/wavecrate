use crate::gui_test::{GuiAivAssertion, GuiAivCase};

use super::super::super::{DEFAULT_VIEWPORT, GUI_TEST_WINDOW_TITLE};
use super::{assert_step, click_node, screenshot, type_into_node, wait_for_node};

pub(crate) fn options_open_close_case() -> GuiAivCase {
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

pub(crate) fn prompt_confirm_case() -> GuiAivCase {
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

pub(crate) fn prompt_cancel_case() -> GuiAivCase {
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
