use crate::gui_test::{GuiAivAssertion, GuiAivCase};

use super::super::super::{DEFAULT_VIEWPORT, GUI_TEST_WINDOW_TITLE};
use super::{assert_step, click_node, drag_in_node, screenshot, scroll_in_node, wait_for_node};

pub(crate) fn waveform_transport_cursor_selection_zoom_case() -> GuiAivCase {
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

pub(crate) fn waveform_click_play_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("waveform_click_play"),
        fixture_tag: String::from("waveform"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("waveform.region"),
            click_node("waveform.region", Some(80), Some(50)),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("play_waveform_at_precise"),
            }),
            screenshot("waveform-click-play"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertActionRecorded {
            action_id: String::from("play_waveform_at_precise"),
        }],
    }
}

pub(crate) fn waveform_transport_button_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("waveform_transport_button"),
        fixture_tag: String::from("transport"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("waveform.toolbar.play"),
            click_node("waveform.toolbar.play", None, None),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("toggle_transport"),
            }),
            screenshot("waveform-transport-button"),
        ],
        expected_assertions: vec![GuiAivAssertion::AssertActionRecorded {
            action_id: String::from("toggle_transport"),
        }],
    }
}

pub(crate) fn transport_volume_slider_drag_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("transport_volume_slider_drag"),
        fixture_tag: String::from("transport"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("shell.top_bar.volume_slider"),
            drag_in_node("shell.top_bar.volume_slider", 42, 50, 80, 50),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("set_volume"),
            }),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("commit_volume_setting"),
            }),
            assert_step(GuiAivAssertion::AssertNodeValueContains {
                node_id: String::from("shell.top_bar.volume_slider"),
                needle: String::from("0.800"),
            }),
            screenshot("transport-volume-slider-drag"),
        ],
        expected_assertions: vec![
            GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("set_volume"),
            },
            GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("commit_volume_setting"),
            },
        ],
    }
}

pub(crate) fn waveform_outside_click_clears_both_marks_case() -> GuiAivCase {
    GuiAivCase {
        name: String::from("waveform_outside_click_clears_both_marks"),
        fixture_tag: String::from("waveform_mixed"),
        viewport: DEFAULT_VIEWPORT,
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        steps: vec![
            wait_for_node("waveform.region"),
            wait_for_node("waveform.selection"),
            wait_for_node("waveform.edit_selection"),
            click_node("waveform.region", Some(10), Some(50)),
            assert_step(GuiAivAssertion::AssertActionRecorded {
                action_id: String::from("clear_waveform_selections"),
            }),
            assert_step(GuiAivAssertion::AssertNodeAbsent {
                node_id: String::from("waveform.selection"),
            }),
            assert_step(GuiAivAssertion::AssertNodeAbsent {
                node_id: String::from("waveform.edit_selection"),
            }),
            screenshot("waveform-outside-click-clears-both-marks"),
        ],
        expected_assertions: vec![
            GuiAivAssertion::AssertNodeAbsent {
                node_id: String::from("waveform.selection"),
            },
            GuiAivAssertion::AssertNodeAbsent {
                node_id: String::from("waveform.edit_selection"),
            },
        ],
    }
}
