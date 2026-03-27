use super::*;

/// Browser action availability should stay disabled until focus or selection exists.
#[test]
fn browser_actions_require_focus_or_selection() {
    let mut ui = UiState::default();
    let projected = project_browser_actions_model(&ui);
    assert!(!projected.can_rename);
    assert!(!projected.can_delete);
    assert!(!projected.can_tag);
    assert!(!projected.random_navigation_enabled);

    ui.browser.selection.selected_visible = Some(0);
    ui.browser.search.random_navigation_mode = true;
    let projected = project_browser_actions_model(&ui);
    assert!(projected.can_rename);
    assert!(projected.can_delete);
    assert!(projected.can_tag);
    assert!(projected.random_navigation_enabled);
}

/// Browser rename prompts should win over destructive waveform prompts when both are present.
#[test]
fn confirm_prompt_prefers_browser_rename_when_multiple_prompts_exist() {
    let mut ui = UiState::default();
    ui.browser.pending_action = Some(SampleBrowserActionPrompt::Rename {
        target: std::path::PathBuf::from("kick.wav"),
        name: String::from("kick"),
    });
    ui.waveform.pending_destructive = Some(DestructiveEditPrompt {
        edit: DestructiveSelectionEdit::TrimSelection,
        title: String::from("Trim selection"),
        message: String::from("Apply trim?"),
    });
    let projected = project_confirm_prompt_model(&ui);
    assert!(projected.visible);
    assert_eq!(projected.kind, Some(ConfirmPromptKind::BrowserRename));
}

/// Inline folder creation should stay out of the modal confirm prompt.
#[test]
fn inline_folder_create_does_not_project_confirm_prompt() {
    let mut ui = UiState::default();
    ui.sources.folders.inline_edit = Some(InlineFolderEdit {
        kind: InlineFolderEditKind::Create {
            parent: std::path::PathBuf::from("drums"),
        },
        name: String::from("kicks"),
        focus_requested: true,
        select_all_on_focus_requested: false,
    });
    let projected = project_confirm_prompt_model(&ui);
    assert!(!projected.visible);
    assert_eq!(projected.kind, None);
}

/// Folder-create projection should insert a stable inline draft row with validation state.
#[test]
fn inline_folder_create_projects_draft_row_and_validation() {
    let mut ui = UiState::default();
    ui.sources.selected = Some(0);
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::new(),
        name: String::from("Root"),
        depth: 0,
        has_children: true,
        expanded: true,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: true,
        root_filter_mode: None,
    });
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::from("drums"),
        name: String::from("drums"),
        depth: 1,
        has_children: true,
        expanded: true,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: false,
        root_filter_mode: None,
    });
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::from("drums/existing"),
        name: String::from("existing"),
        depth: 2,
        has_children: false,
        expanded: false,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: false,
        root_filter_mode: None,
    });
    ui.sources.folders.focused = Some(1);
    ui.sources.folders.inline_edit = Some(InlineFolderEdit {
        kind: InlineFolderEditKind::Create {
            parent: std::path::PathBuf::from("drums"),
        },
        name: String::from("existing"),
        focus_requested: true,
        select_all_on_focus_requested: false,
    });
    let projected = project_sources_model(&ui);
    let draft = projected
        .folder_rows
        .iter()
        .find(|row| row.kind == FolderRowKind::CreateDraft)
        .expect("inline draft row should be projected");
    assert_eq!(projected.focused_folder_row, Some(1));
    assert_eq!(draft.depth, 2);
    assert_eq!(draft.input_value.as_deref(), Some("existing"));
    assert_eq!(draft.input_placeholder.as_deref(), Some("New folder name"));
    assert_eq!(
        draft.input_error.as_deref(),
        Some("Folder already exists: drums/existing")
    );
    assert_eq!(projected.folder_rows[2].kind, FolderRowKind::CreateDraft);

    if let Some(edit) = ui.sources.folders.inline_edit.as_mut() {
        edit.name = String::from("bad/name");
    }
    let projected = project_sources_model(&ui);
    let draft = projected
        .folder_rows
        .iter()
        .find(|row| row.kind == FolderRowKind::CreateDraft)
        .expect("inline draft row should still be projected");
    assert_eq!(
        draft.input_error.as_deref(),
        Some("Folder name cannot contain path separators")
    );
}

/// Root-level folder creation should insert the draft row directly below the root row.
#[test]
fn root_inline_folder_create_inserts_after_root_row() {
    let mut ui = UiState::default();
    ui.sources.selected = Some(0);
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::new(),
        name: String::from("Root"),
        depth: 0,
        has_children: true,
        expanded: true,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: true,
        root_filter_mode: None,
    });
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::from("drums"),
        name: String::from("drums"),
        depth: 1,
        has_children: false,
        expanded: false,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: false,
        root_filter_mode: None,
    });
    ui.sources.folders.inline_edit = Some(InlineFolderEdit {
        kind: InlineFolderEditKind::Create {
            parent: std::path::PathBuf::new(),
        },
        name: String::from("fresh"),
        focus_requested: true,
        select_all_on_focus_requested: false,
    });

    let projected = project_sources_model(&ui);

    assert_eq!(projected.folder_rows[1].kind, FolderRowKind::CreateDraft);
    assert_eq!(projected.folder_rows[1].depth, 1);
}

/// Inline folder rename should replace the existing row and surface validation errors.
#[test]
fn inline_folder_rename_projects_inline_row_and_validation() {
    let mut ui = UiState::default();
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::from("drums"),
        name: String::from("drums"),
        depth: 1,
        has_children: false,
        expanded: false,
        selected: true,
        negated: false,
        hotkey: None,
        is_root: false,
        root_filter_mode: None,
    });
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::from("kicks"),
        name: String::from("kicks"),
        depth: 1,
        has_children: false,
        expanded: false,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: false,
        root_filter_mode: None,
    });
    ui.sources.folders.focused = Some(0);
    ui.sources.folders.inline_edit = Some(InlineFolderEdit {
        kind: InlineFolderEditKind::Rename {
            target: std::path::PathBuf::from("drums"),
        },
        name: String::from("kicks"),
        focus_requested: true,
        select_all_on_focus_requested: true,
    });
    let projected = project_sources_model(&ui);
    assert_eq!(projected.focused_folder_row, Some(0));
    let draft = &projected.folder_rows[0];
    assert_eq!(draft.kind, FolderRowKind::RenameDraft);
    assert_eq!(
        draft.input_error.as_deref(),
        Some("Folder already exists: kicks")
    );
    assert_eq!(draft.input_value.as_deref(), Some("kicks"));
    assert_eq!(draft.input_placeholder.as_deref(), Some("Folder name"));
    assert!(draft.input_focused);
    assert!(draft.select_all_on_focus);
    assert_eq!(draft.source_index, Some(0));

    if let Some(edit) = ui.sources.folders.inline_edit.as_mut() {
        edit.name = String::from("../bad");
        edit.select_all_on_focus_requested = false;
    }
    let projected = project_sources_model(&ui);
    let draft = &projected.folder_rows[0];
    assert_eq!(
        draft.input_error.as_deref(),
        Some("Folder name cannot contain path separators")
    );
}

/// Progress overlay projection should preserve modal and cancel-requested flags.
#[test]
fn progress_overlay_projection_preserves_cancel_state() {
    let mut ui = UiState::default();
    ui.progress.visible = true;
    ui.progress.modal = true;
    ui.progress.title = String::from("Scanning");
    ui.progress.completed = 3;
    ui.progress.total = 9;
    ui.progress.cancelable = true;
    ui.progress.cancel_requested = true;
    let projected = project_progress_overlay_model(&ui);
    assert!(projected.visible);
    assert!(projected.modal);
    assert!(projected.cancelable);
    assert!(projected.cancel_requested);
    assert_eq!(projected.completed, 3);
    assert_eq!(projected.total, 9);
}

/// Destructive folder actions should require focus on a non-root folder row.
#[test]
fn folder_actions_require_non_root_focus_for_destructive_actions() {
    let mut ui = UiState::default();
    ui.sources.selected = Some(0);
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::new(),
        name: String::from("Root"),
        depth: 0,
        has_children: true,
        expanded: true,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: true,
        root_filter_mode: None,
    });
    ui.sources.folders.focused = Some(0);
    let projected = project_sources_model(&ui);
    assert!(projected.folder_actions.can_create_folder);
    assert!(projected.folder_actions.can_create_folder_at_root);
    assert!(!projected.folder_actions.can_rename_folder);
    assert!(!projected.folder_actions.can_delete_folder);

    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::from("drums"),
        name: String::from("drums"),
        depth: 1,
        has_children: false,
        expanded: false,
        selected: true,
        negated: false,
        hotkey: None,
        is_root: false,
        root_filter_mode: None,
    });
    ui.sources.folders.focused = Some(1);
    let projected = project_sources_model(&ui);
    assert!(projected.folder_actions.can_rename_folder);
    assert!(projected.folder_actions.can_delete_folder);
}

/// Root folder creation should remain available even when there are no source rows yet.
#[test]
fn folder_actions_allow_root_creation_when_no_sources_exist() {
    let ui = UiState::default();
    let projected = project_sources_model(&ui);
    assert!(!projected.folder_actions.can_create_folder);
    assert!(projected.folder_actions.can_create_folder_at_root);
}

/// Recovery log clearing should stay disabled while delete recovery work is still running.
#[test]
fn folder_actions_disable_recovery_clear_while_recovery_is_running() {
    let mut ui = UiState::default();
    ui.sources
        .folders
        .delete_recovery
        .entries
        .push(FolderDeleteRecoveryEntry {
            source_label: String::from("source"),
            relative_path: std::path::PathBuf::from("drums"),
            action: FolderDeleteRecoveryAction::Restore,
            status: FolderDeleteRecoveryStatus::Completed,
            detail: None,
        });
    ui.sources.folders.delete_recovery.in_progress = true;
    let projected = project_sources_model(&ui);
    assert!(!projected.folder_actions.can_clear_recovery_log);
}
