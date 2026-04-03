use super::*;
use crate::gui_test::find_automation_node;

fn assert_fixture_node_actions(fixture_tag: &str, node_id: &str, expected_actions: &[&str]) {
    let bundle = capture_default_bundle(&deterministic_test_config(fixture_tag))
        .unwrap_or_else(|err| panic!("fixture {fixture_tag} capture failed: {err}"));
    let node = find_automation_node(&bundle.automation_snapshot, node_id)
        .unwrap_or_else(|| panic!("fixture {fixture_tag} missing automation node {node_id}"));
    let actual_actions: Vec<_> = node.available_actions.iter().map(String::as_str).collect();
    assert_eq!(
        actual_actions, expected_actions,
        "fixture {fixture_tag} node {node_id} advertised unexpected actions"
    );
    for action_id in expected_actions {
        assert!(
            action_catalog_entry_by_id(action_id).is_some(),
            "fixture {fixture_tag} node {node_id} advertised uncataloged expected action {action_id}"
        );
    }
}

#[test]
fn capture_default_bundle_advertises_expected_action_ids_for_representative_nodes() {
    let cases: &[(&str, &str, &[&str])] = &[
        ("browser", "browser.tab.samples", &["set_browser_tab"]),
        (
            "browser",
            "browser.search_field",
            &["focus_browser_search", "set_browser_search"],
        ),
        (
            "browser",
            "browser.marked_filter",
            &["toggle_browser_marked_filter"],
        ),
        (
            "browser",
            "browser.row.0",
            &[
                "focus_browser_row",
                "toggle_browser_row_selection",
                "commit_focused_browser_row",
            ],
        ),
        (
            "browser",
            "sources.source_row.0",
            &[
                "select_source_row",
                "reload_source_row",
                "hard_sync_source_row",
                "open_source_folder_row",
                "remove_source_row",
            ],
        ),
        (
            "browser",
            "sources.upper.folder_visibility_toggle",
            &["toggle_show_all_folders"],
        ),
        (
            "sources",
            "sources.upper.folder_row.1",
            &[
                "focus_folder_row",
                "activate_folder_row",
                "start_new_folder_at_folder_row",
                "toggle_folder_row_expanded",
            ],
        ),
        (
            "transport",
            "shell.top_bar.volume_slider",
            &["set_volume", "commit_volume_setting"],
        ),
        (
            "browser",
            "shell.top_bar.options_button",
            &["open_options_menu"],
        ),
        (
            "options",
            "shell.top_bar.options_button",
            &["close_options_panel"],
        ),
        (
            "waveform",
            "waveform.region",
            &[
                "detect_waveform_silence_slices",
                "detect_waveform_exact_duplicate_slices",
                "clean_waveform_exact_duplicate_slices",
                "audition_waveform_duplicate_slice",
                "toggle_waveform_duplicate_slice_exemption",
                "move_waveform_slice_focus",
                "toggle_focused_waveform_slice_export_mark",
                "play_waveform_at_precise",
                "clear_waveform_selections",
                "seek_waveform",
                "set_waveform_cursor",
                "set_waveform_selection_range",
                "zoom_waveform",
                "set_waveform_view_center",
            ],
        ),
        (
            "browser",
            "browser.table",
            &["focus_browser_panel", "set_browser_view_start"],
        ),
        (
            "waveform",
            "waveform.selection",
            &["clear_waveform_selection"],
        ),
        (
            "waveform_mixed",
            "waveform.edit_selection",
            &["clear_waveform_edit_selection"],
        ),
        ("options", "overlay.options_panel", &["close_options_panel"]),
        ("prompt", "overlay.prompt.confirm", &["confirm_prompt"]),
        ("prompt", "overlay.prompt.cancel", &["cancel_prompt"]),
        ("prompt", "overlay.prompt.input", &["set_prompt_input"]),
        ("update", "shell.top_bar.update.open", &["open_update_link"]),
        (
            "update",
            "shell.top_bar.update.install",
            &["install_update"],
        ),
        (
            "update",
            "shell.top_bar.update.dismiss",
            &["dismiss_update"],
        ),
    ];

    for (fixture_tag, node_id, expected_actions) in cases {
        assert_fixture_node_actions(fixture_tag, node_id, expected_actions);
    }
}
