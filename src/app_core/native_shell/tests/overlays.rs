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

    ui.browser.selected_visible = Some(0);
    ui.browser.random_navigation_mode = true;
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

/// Inline folder creation state should project into the shared confirm prompt model.
#[test]
fn confirm_prompt_projects_folder_create_inline_state() {
    let mut ui = UiState::default();
    ui.sources.folders.new_folder = Some(InlineFolderCreation {
        parent: std::path::PathBuf::from("drums"),
        name: String::from("kicks"),
        focus_requested: true,
    });
    let projected = project_confirm_prompt_model(&ui);
    assert!(projected.visible);
    assert_eq!(projected.kind, Some(ConfirmPromptKind::FolderCreate));
    assert_eq!(projected.confirm_label, "Create");
    assert_eq!(projected.input_value.as_deref(), Some("kicks"));
    assert_eq!(
        projected.input_placeholder.as_deref(),
        Some("New folder name")
    );
}

/// Folder-create projection should surface duplicate-name and separator validation errors.
#[test]
fn confirm_prompt_projects_folder_create_validation_errors() {
    let mut ui = UiState::default();
    ui.sources.folders.rows.push(FolderRowView {
        path: std::path::PathBuf::from("drums/existing"),
        name: String::from("existing"),
        depth: 1,
        has_children: false,
        expanded: false,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: false,
        root_filter_mode: None,
    });
    ui.sources.folders.new_folder = Some(InlineFolderCreation {
        parent: std::path::PathBuf::from("drums"),
        name: String::from("existing"),
        focus_requested: true,
    });
    let projected = project_confirm_prompt_model(&ui);
    assert_eq!(
        projected.input_error.as_deref(),
        Some("Folder already exists: drums/existing")
    );

    if let Some(new_folder) = ui.sources.folders.new_folder.as_mut() {
        new_folder.name = String::from("bad/name");
    }
    let projected = project_confirm_prompt_model(&ui);
    assert_eq!(
        projected.input_error.as_deref(),
        Some("Folder name cannot contain path separators")
    );
}

/// Folder-rename projection should surface duplicate-name and separator validation errors.
#[test]
fn confirm_prompt_projects_folder_rename_validation_errors() {
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
    ui.sources.folders.pending_action = Some(FolderActionPrompt::Rename {
        target: std::path::PathBuf::from("drums"),
        name: String::from("kicks"),
    });
    let projected = project_confirm_prompt_model(&ui);
    assert_eq!(
        projected.input_error.as_deref(),
        Some("Folder already exists: kicks")
    );

    ui.sources.folders.pending_action = Some(FolderActionPrompt::Rename {
        target: std::path::PathBuf::from("drums"),
        name: String::from("../bad"),
    });
    let projected = project_confirm_prompt_model(&ui);
    assert_eq!(
        projected.input_error.as_deref(),
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
