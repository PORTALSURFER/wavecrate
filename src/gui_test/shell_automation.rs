use crate::{
    app_core::actions::{
        NativeAppModel, NativeAutomationNodeSnapshot, NativeAutomationRole, NativeBrowserRowModel,
        NativeFocusContextModel, NativeFolderRowModel, NativeSourceRowModel,
    },
    gui_runtime::capture_gui_automation_snapshot,
};

fn child<'a>(
    parent: &'a NativeAutomationNodeSnapshot,
    id: &str,
) -> &'a NativeAutomationNodeSnapshot {
    parent
        .children
        .iter()
        .find(|node| node.id.0 == id)
        .unwrap_or_else(|| panic!("missing automation child {id}"))
}

#[test]
fn automation_snapshot_exposes_semantic_shell_nodes_from_sempal_fixture() {
    let mut model = NativeAppModel::default();
    model.title = String::from("Sempal");
    model.status.center = String::from("rows: 3 | selected: 1 | anchor: 1 | search: kick");
    model.sources.rows.push(NativeSourceRowModel::new(
        "Primary source",
        "C:/samples",
        true,
        false,
    ));
    model
        .sources
        .upper_folder_pane
        .folder_rows
        .push(NativeFolderRowModel::new(
            "drums",
            String::new(),
            0,
            false,
            true,
            true,
            true,
            true,
        ));
    model.browser.rows.push(NativeBrowserRowModel::new(
        0,
        "kick_001.wav",
        1,
        false,
        true,
    ));
    model.browser.visible_count = 1;
    model.browser.selected_visible_row = Some(0);
    model.focus_context = NativeFocusContextModel::SampleBrowser;

    let snapshot = capture_gui_automation_snapshot([1440.0, 810.0], &model);
    assert_eq!(snapshot.root.id.0, "shell.root");
    assert_eq!(snapshot.root.label.as_deref(), Some("Sempal shell"));

    let top_bar = child(&snapshot.root, "shell.top_bar");
    let sources = child(&snapshot.root, "sources.panel");
    let waveform = child(&snapshot.root, "waveform.panel");
    let browser = child(&snapshot.root, "browser.panel");
    let status = child(&snapshot.root, "shell.status_bar");

    assert_eq!(top_bar.role, NativeAutomationRole::Panel);
    assert_eq!(sources.role, NativeAutomationRole::Panel);
    assert_eq!(waveform.role, NativeAutomationRole::Panel);
    assert_eq!(browser.role, NativeAutomationRole::Panel);
    assert_eq!(status.role, NativeAutomationRole::Readout);
    assert_eq!(status.value.as_deref(), Some(model.status.center.as_str()));

    let table = child(browser, "browser.table");
    let row = child(table, "browser.row.0");
    assert_eq!(table.role, NativeAutomationRole::Table);
    assert_eq!(row.label.as_deref(), Some("kick_001.wav"));
    assert!(row.selected);
}
