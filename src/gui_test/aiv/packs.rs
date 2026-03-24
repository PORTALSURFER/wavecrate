//! Desktop AIV pack selection and coverage tests.

mod cases;

use self::cases::{
    browser_interior_click_keeps_viewport_after_down_scroll_case,
    browser_interior_click_keeps_viewport_after_up_scroll_case, browser_map_point_focus_case,
    browser_refocus_after_down_scroll_keeps_single_focus_case,
    browser_repeated_scroll_refocus_preserves_guard_band_case, browser_search_select_commit_case,
    browser_search_type_smoke_case, browser_tabs_and_rating_filters_case,
    browser_wheel_scroll_uses_rendered_viewport_case, options_open_close_case, prompt_cancel_case,
    prompt_confirm_case, startup_ready_case, transport_volume_slider_drag_case,
    update_panel_actions_case, waveform_click_seek_case,
    waveform_outside_click_clears_both_marks_case,
    waveform_transport_button_case, waveform_transport_cursor_selection_zoom_case,
};
use super::{
    DEFAULT_GUI_AIV_PACK, GUI_AIV_SCHEMA_VERSION, GUI_TEST_WINDOW_TITLE, GuiAivSuiteManifest,
    REGRESSION_GUI_AIV_PACK,
};

/// Resolve one named desktop AIV suite pack.
pub fn gui_aiv_suite_manifest(pack_name: &str) -> Result<GuiAivSuiteManifest, String> {
    match pack_name {
        DEFAULT_GUI_AIV_PACK => Ok(desktop_smoke_pack()),
        REGRESSION_GUI_AIV_PACK => Ok(desktop_regression_pack()),
        other => Err(format!("unknown desktop AIV suite pack '{other}'")),
    }
}

fn desktop_smoke_pack() -> GuiAivSuiteManifest {
    GuiAivSuiteManifest {
        schema_version: GUI_AIV_SCHEMA_VERSION,
        pack_name: String::from(DEFAULT_GUI_AIV_PACK),
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        cases: vec![
            startup_ready_case(),
            options_open_close_case(),
            browser_search_type_smoke_case(),
        ],
    }
}

fn desktop_regression_pack() -> GuiAivSuiteManifest {
    GuiAivSuiteManifest {
        schema_version: GUI_AIV_SCHEMA_VERSION,
        pack_name: String::from(REGRESSION_GUI_AIV_PACK),
        window_title: String::from(GUI_TEST_WINDOW_TITLE),
        cases: vec![
            startup_ready_case(),
            browser_search_select_commit_case(),
            browser_tabs_and_rating_filters_case(),
            browser_interior_click_keeps_viewport_after_down_scroll_case(),
            browser_interior_click_keeps_viewport_after_up_scroll_case(),
            browser_refocus_after_down_scroll_keeps_single_focus_case(),
            browser_repeated_scroll_refocus_preserves_guard_band_case(),
            browser_wheel_scroll_uses_rendered_viewport_case(),
            browser_map_point_focus_case(),
            options_open_close_case(),
            prompt_confirm_case(),
            prompt_cancel_case(),
            waveform_transport_button_case(),
            transport_volume_slider_drag_case(),
            waveform_click_seek_case(),
            waveform_transport_cursor_selection_zoom_case(),
            waveform_outside_click_clears_both_marks_case(),
            update_panel_actions_case(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn desktop_pack_names_resolve_with_expected_case_lists() {
        let smoke = gui_aiv_suite_manifest(DEFAULT_GUI_AIV_PACK).expect("smoke pack");
        let regression = gui_aiv_suite_manifest(REGRESSION_GUI_AIV_PACK).expect("regression pack");
        assert_eq!(
            smoke
                .cases
                .iter()
                .map(|case| case.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "startup_ready",
                "options_open_close",
                "browser_search_type_smoke"
            ]
        );
        assert_eq!(
            regression
                .cases
                .iter()
                .map(|case| case.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "startup_ready",
                "browser_search_select_commit",
                "browser_tabs_and_rating_filters",
                "browser_interior_click_keeps_viewport_after_down_scroll",
                "browser_interior_click_keeps_viewport_after_up_scroll",
                "browser_refocus_after_down_scroll_keeps_single_focus",
                "browser_repeated_scroll_refocus_preserves_guard_band",
                "browser_wheel_scroll_uses_rendered_viewport",
                "browser_map_point_focus",
                "options_open_close",
                "prompt_confirm",
                "prompt_cancel",
                "waveform_transport_button",
                "transport_volume_slider_drag",
                "waveform_click_seek",
                "waveform_transport_cursor_selection_zoom",
                "waveform_outside_click_clears_both_marks",
                "update_panel_actions",
            ]
        );
    }

    #[test]
    fn regression_pack_covers_required_nodes_and_assertion_verbs() {
        let regression = gui_aiv_suite_manifest(REGRESSION_GUI_AIV_PACK).expect("regression pack");
        let json = serde_json::to_value(&regression).expect("serialize manifest");
        let json_text = serde_json::to_string(&json).expect("serialize manifest text");
        for node_id in [
            "browser.search_field",
            "browser.row.0",
            "browser.row.18",
            "browser.row.19",
            "browser.row.20",
            "browser.row.12",
            "browser.row.15",
            "browser.row.5",
            "browser.row.4",
            "browser.row.3",
            "browser.tab.samples",
            "browser.tab.map",
            "browser.map_canvas",
            "browser.map.point.gui-map-source::kick_one.wav",
            "browser.rating_filter.3",
            "shell.top_bar.volume_slider",
            "shell.top_bar.options_button",
            "overlay.prompt.confirm",
            "overlay.prompt.cancel",
            "overlay.prompt.input",
            "waveform.region",
            "waveform.selection",
            "waveform.edit_selection",
            "waveform.toolbar.play",
            "waveform.toolbar.loop",
            "shell.top_bar.update.open",
            "shell.top_bar.update.dismiss",
        ] {
            assert!(
                json_text.contains(node_id),
                "manifest should include node id {node_id}"
            );
        }
        let kinds = collect_kinds(&json);
        for kind in [
            "assert_node_present",
            "assert_node_absent",
            "assert_node_selected",
            "assert_node_value_contains",
            "assert_node_metadata_contains",
            "assert_action_recorded",
        ] {
            assert!(kinds.contains(kind), "manifest should include kind {kind}");
        }
    }

    fn collect_kinds(value: &serde_json::Value) -> BTreeSet<&str> {
        let mut kinds = BTreeSet::new();
        collect_kinds_recursive(value, &mut kinds);
        kinds
    }

    fn collect_kinds_recursive<'a>(value: &'a serde_json::Value, out: &mut BTreeSet<&'a str>) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(kind) = map.get("kind").and_then(serde_json::Value::as_str) {
                    out.insert(kind);
                }
                for child in map.values() {
                    collect_kinds_recursive(child, out);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    collect_kinds_recursive(child, out);
                }
            }
            _ => {}
        }
    }
}
