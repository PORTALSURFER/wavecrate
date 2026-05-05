use crate::{
    app_core::actions::{
        NativeAppModel, NativeAutomationNodeSnapshot, NativeAutomationRole, NativeBrowserRowModel,
        NativeFocusContextModel, NativeFolderRowModel, NativeProgressOverlayModel,
        NativeSourceRowModel,
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

/// Return true when the automation subtree contains the given node ID.
fn contains_node(parent: &NativeAutomationNodeSnapshot, id: &str) -> bool {
    parent.id.0 == id || parent.children.iter().any(|child| contains_node(child, id))
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
        .tree_rows
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
    assert!(contains_node(sources, "sources.source_list"));
    assert!(contains_node(sources, "sources.folder_browser"));
    assert!(!contains_node(sources, "sources.upper.source_list"));
    assert!(!contains_node(sources, "sources.lower.source_list"));
}

#[test]
fn automation_snapshot_exposes_status_bar_readout_with_projected_copy() {
    let mut model = NativeAppModel::default();
    model.status.left = String::from("Transport: running");
    model.status.center = String::from("rows: 12 | selected: 3 | anchor: 4 | search: clap");
    model.status.right = String::from("col: 2/3");

    let snapshot = capture_gui_automation_snapshot([1280.0, 720.0], &model);
    let status_bar = child(&snapshot.root, "shell.status_bar");

    assert_eq!(status_bar.label.as_deref(), Some("Status bar"));
    assert_eq!(
        status_bar.value.as_deref(),
        Some(model.status.center.as_str())
    );
    assert_eq!(
        status_bar.metadata.get("left").map(String::as_str),
        Some(model.status.left.as_str())
    );
    assert_eq!(
        status_bar.metadata.get("right").map(String::as_str),
        Some(model.status.right.as_str())
    );
    assert!(status_bar.bounds.width > 0.0);
    assert!(status_bar.bounds.height > 0.0);
}

#[test]
fn automation_snapshot_keeps_status_bar_visible_during_inline_progress() {
    let mut model = NativeAppModel::default();
    model.status.center = String::from("rows: 12 | selected: 3");
    model.progress_overlay = NativeProgressOverlayModel {
        visible: true,
        modal: false,
        title: String::from("Scanning"),
        detail: Some(String::from("source_a")),
        completed: 4,
        total: 9,
        cancelable: false,
        cancel_requested: false,
    };

    let snapshot = capture_gui_automation_snapshot([960.0, 540.0], &model);
    let status_bar = child(&snapshot.root, "shell.status_bar");

    assert!(status_bar.bounds.width > 0.0);
    assert!(status_bar.bounds.height > 0.0);
    assert_eq!(
        status_bar.metadata.get("center").map(String::as_str),
        Some(model.status.center.as_str())
    );
}
