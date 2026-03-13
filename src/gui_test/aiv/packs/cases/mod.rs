//! Concrete desktop GUI case catalog used by the named AIV packs.

use crate::gui_test::{GuiAivAssertion, GuiAivStep};

mod browser;
mod options_prompt;
mod startup;
mod update;
mod waveform;

pub(super) use browser::{
    browser_interior_click_keeps_viewport_after_down_scroll_case,
    browser_interior_click_keeps_viewport_after_up_scroll_case,
    browser_refocus_after_down_scroll_keeps_single_focus_case,
    browser_repeated_scroll_refocus_preserves_guard_band_case, browser_search_select_commit_case,
    browser_search_type_smoke_case, browser_tabs_and_rating_filters_case,
    browser_wheel_scroll_uses_rendered_viewport_case,
};
pub(super) use options_prompt::{options_open_close_case, prompt_cancel_case, prompt_confirm_case};
pub(super) use startup::startup_ready_case;
pub(super) use update::update_panel_actions_case;
pub(super) use waveform::{
    waveform_outside_click_clears_both_marks_case, waveform_transport_cursor_selection_zoom_case,
};

const BROWSER_SCROLL_VIEWPORT: [u32; 2] = [1280, 720];

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
