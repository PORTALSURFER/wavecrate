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
    assert!(!projected.duplicate_cleanup_active);

    ui.browser.selection.selected_visible = Some(0);
    ui.browser.search.random_navigation_mode = true;
    let projected = project_browser_actions_model(&ui);
    assert!(projected.can_rename);
    assert!(projected.can_delete);
    assert!(projected.can_tag);
    assert!(projected.random_navigation_enabled);
    assert!(!projected.duplicate_cleanup_active);

    ui.browser.duplicate_cleanup = Some(crate::app::state::BrowserDuplicateCleanupState::new(
        crate::sample_sources::SourceId::from_string("source"),
        String::from("sample-id"),
        std::path::PathBuf::from("anchor.wav"),
        String::from("Duplicates of anchor"),
        vec![0],
        vec![1.0],
        0,
    ));
    let projected = project_browser_actions_model(&ui);
    assert!(projected.duplicate_cleanup_active);
}

/// Browser rename prompts should win over destructive waveform prompts when both are present.
#[test]
fn confirm_prompt_prefers_browser_rename_when_multiple_prompts_exist() {
    let mut ui = UiState::default();
    ui.browser.pending_action = Some(SampleBrowserActionPrompt::Rename {
        target: std::path::PathBuf::from("kick.wav"),
        name: String::from("kick"),
        input_error: None,
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

/// Folder-drop conflict prompts should project warning copy and inline validation.
#[test]
fn confirm_prompt_projects_folder_drop_conflict_prompt() {
    let mut ui = UiState::default();
    ui.browser.pending_action = Some(SampleBrowserActionPrompt::MoveToFolderConflict {
        source_id: crate::sample_sources::SourceId::from_string("source"),
        source_relative: std::path::PathBuf::from("kick.wav"),
        target_folder: std::path::PathBuf::from("dest"),
        name: String::from("kick_001"),
        input_error: Some(String::from(
            "A file named dest/kick_001.wav already exists",
        )),
    });

    let projected = project_confirm_prompt_model(&ui);

    assert!(projected.visible);
    assert_eq!(projected.kind, Some(ConfirmPromptKind::BrowserRename));
    assert_eq!(projected.title, "Name conflict");
    assert_eq!(projected.confirm_label, "Move");
    assert_eq!(projected.target_label.as_deref(), Some("Folder: dest"));
    assert_eq!(projected.input_value.as_deref(), Some("kick_001"));
    assert_eq!(
        projected.input_error.as_deref(),
        Some("A file named dest/kick_001.wav already exists")
    );
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

/// Drag overlay projection should carry the cursor anchor while a drag is active.
#[test]
fn drag_overlay_projection_includes_pointer_anchor_for_active_drag() {
    let mut ui = UiState::default();
    ui.drag.payload = Some(crate::app::state::DragPayload::Sample {
        source_id: crate::sample_sources::SourceId::from_string("source"),
        relative_path: std::path::PathBuf::from("kick.wav"),
    });
    ui.drag.label = String::from("kick");
    ui.drag.position = Some(crate::app::state::UiPoint::new(24.4, 96.7));

    let projected = project_drag_overlay_model(&ui);

    assert!(projected.active);
    assert_eq!(projected.label, "kick");
    assert_eq!(projected.pointer_x, Some(24));
    assert_eq!(projected.pointer_y, Some(97));
}

/// Drag overlay projection should clear the floating chip anchor when no drag is active.
#[test]
fn drag_overlay_projection_clears_pointer_anchor_without_active_drag() {
    let mut ui = UiState::default();
    ui.drag.position = Some(crate::app::state::UiPoint::new(24.4, 96.7));

    let projected = project_drag_overlay_model(&ui);

    assert!(!projected.active);
    assert_eq!(projected.pointer_x, None);
    assert_eq!(projected.pointer_y, None);
}
