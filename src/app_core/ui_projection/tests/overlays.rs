use super::*;

/// Browser action availability should stay disabled until focus or selection exists.
#[test]
fn browser_actions_require_focus_or_selection() {
    let mut ui = UiState::default();
    let projected = project_browser_actions_model(&ui);
    assert!(!projected.can_delete);
    assert!(!projected.can_tag);
    assert!(!projected.can_normalize_focused_sample);
    assert!(!projected.can_loop_crossfade_focused_sample);
    assert!(!projected.random_navigation_enabled);
    assert!(!projected.duplicate_cleanup_active);

    ui.browser.selection.selected_visible = Some(0);
    ui.browser.selection.last_focused_path = Some(std::path::PathBuf::from("focused.wav"));
    ui.browser.search.random_navigation_mode = true;
    let projected = project_browser_actions_model(&ui);
    assert!(projected.can_delete);
    assert!(projected.can_tag);
    assert!(projected.can_normalize_focused_sample);
    assert!(projected.can_loop_crossfade_focused_sample);
    assert!(projected.random_navigation_enabled);
    assert!(!projected.duplicate_cleanup_active);

    ui.browser.duplicate_cleanup = Some(BrowserDuplicateCleanupState::new(
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

/// Focused browser destructive actions should stay disabled for non-WAV paths.
#[test]
fn browser_actions_disable_wav_only_edits_for_non_wav_focus() {
    let mut ui = UiState::default();
    ui.browser.selection.selected_visible = Some(0);
    ui.browser.selection.last_focused_path = Some(std::path::PathBuf::from("focused.flac"));

    let projected = project_browser_actions_model(&ui);

    assert!(projected.can_delete);
    assert!(projected.can_tag);
    assert!(!projected.can_normalize_focused_sample);
    assert!(!projected.can_loop_crossfade_focused_sample);
}

/// Browser naming prompts should win over destructive waveform prompts when both are present.
#[test]
fn confirm_prompt_prefers_browser_name_conflict_when_multiple_prompts_exist() {
    let mut ui = UiState::default();
    ui.browser.pending_action = Some(SampleBrowserActionPrompt::MoveToFolderConflict {
        source_id: crate::sample_sources::SourceId::from_string("source"),
        source_relative: std::path::PathBuf::from("kick.wav"),
        target_folder: std::path::PathBuf::from("dest"),
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
    assert_eq!(projected.kind, Some(ConfirmPromptKind::BrowserNameConflict));
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
    assert_eq!(projected.kind, Some(ConfirmPromptKind::BrowserNameConflict));
    assert_eq!(projected.title, "Name conflict");
    assert_eq!(projected.confirm_label, "Move");
    assert_eq!(projected.target_label.as_deref(), Some("Folder: dest"));
    assert_eq!(projected.input_value.as_deref(), Some("kick_001"));
    assert_eq!(
        projected.input_error.as_deref(),
        Some("A file named dest/kick_001.wav already exists")
    );
}

/// Browser delete prompts should project through the shared destructive prompt overlay.
#[test]
fn confirm_prompt_projects_browser_delete_prompt() {
    let mut ui = UiState::default();
    ui.browser.pending_action = Some(SampleBrowserActionPrompt::Delete {
        targets: vec![
            std::path::PathBuf::from("kick.wav"),
            std::path::PathBuf::from("snare.wav"),
        ],
    });

    let projected = project_confirm_prompt_model(&ui);

    assert!(projected.visible);
    assert_eq!(projected.kind, Some(ConfirmPromptKind::DestructiveEdit));
    assert_eq!(projected.title, "Delete samples");
    assert_eq!(projected.confirm_label, "Delete");
    assert_eq!(projected.target_label.as_deref(), Some("2 samples"));
    assert_eq!(projected.input_value, None);
}

/// Folder delete prompts should project through the shared destructive prompt overlay.
#[test]
fn confirm_prompt_projects_folder_delete_prompt() {
    let mut ui = UiState::default();
    ui.sources.folders.pending_action = Some(FolderActionPrompt::Delete {
        target: std::path::PathBuf::from("Drums"),
    });

    let projected = project_confirm_prompt_model(&ui);

    assert!(projected.visible);
    assert_eq!(projected.kind, Some(ConfirmPromptKind::DestructiveEdit));
    assert_eq!(projected.title, "Delete folder");
    assert_eq!(projected.confirm_label, "Delete");
    assert_eq!(projected.target_label.as_deref(), Some("Drums"));
    assert_eq!(projected.input_value, None);
}

/// Options-panel identifier editing should project through the shared prompt overlay.
#[test]
fn confirm_prompt_projects_default_identifier_prompt() {
    let mut ui = UiState::default();
    ui.options_panel.pending_prompt = Some(OptionsPanelPrompt::DefaultIdentifier {
        value: String::from("portal"),
    });

    let projected = project_confirm_prompt_model(&ui);

    assert!(projected.visible);
    assert_eq!(
        projected.kind,
        Some(ConfirmPromptKind::OptionsDefaultIdentifier)
    );
    assert_eq!(projected.title, "Default identifier");
    assert_eq!(projected.confirm_label, "Save");
    assert_eq!(projected.input_value.as_deref(), Some("portal"));
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
    ui.drag.payload = Some(DragPayload::Sample {
        source_id: crate::sample_sources::SourceId::from_string("source"),
        relative_path: std::path::PathBuf::from("kick.wav"),
    });
    ui.drag.label = String::from("kick");
    ui.drag.position = Some(UiPoint::new(24.4, 96.7));

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
    ui.drag.position = Some(UiPoint::new(24.4, 96.7));

    let projected = project_drag_overlay_model(&ui);

    assert!(!projected.active);
    assert_eq!(projected.pointer_x, None);
    assert_eq!(projected.pointer_y, None);
}
