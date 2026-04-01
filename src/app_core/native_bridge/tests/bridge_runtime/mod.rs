use super::*;
use crate::app::state::BrowserDuplicateCleanupState;
use crate::app_core::state::{InlineFolderEdit, InlineFolderEditKind};

mod dirty_graph;
mod gui_test;
mod projection;
mod pull_prep;
mod waveform_queue;

fn browser_row_bucket_label(
    model: &crate::app_core::actions::NativeAppModel,
    row_label: &str,
) -> Option<String> {
    model
        .browser
        .rows
        .iter()
        .find(|row| row.label == row_label)
        .and_then(|row| row.bucket_label.clone())
}

/// Focused projection tests for browser viewport refresh semantics.
mod projection_browser_viewport {
    use super::*;

    /// Manual browser viewport actions must refresh the projected row window
    /// immediately so wheel/scrollbar input updates both the semantic snapshot and
    /// the rendered browser list in the same interaction.
    #[test]
    fn set_browser_view_start_action_refreshes_projected_model_immediately() {
        let mut bridge = test_bridge(16);
        bridge.controller.ui.browser.viewport.visible =
            crate::app_core::state::VisibleRows::All { total: 40 };

        let initial = bridge.project_model();
        assert_eq!(initial.browser.view_start_row, 0);

        bridge.on_action(NativeUiAction::SetBrowserViewStart { visible_row: 1 });

        let updated = bridge.project_model();
        assert_eq!(updated.browser.view_start_row, 1);
    }

    /// Focus-only browser actions should preserve the current manual viewport start
    /// so native guard-band autoscroll can continue from the rows already on
    /// screen instead of snapping back to the retained host slice start.
    #[test]
    fn focus_browser_row_preserves_manual_viewport_start_in_projected_model() {
        let mut bridge = test_bridge(16);
        bridge.controller.ui.browser.viewport.visible =
            crate::app_core::state::VisibleRows::All { total: 40 };

        bridge.on_action(NativeUiAction::SetBrowserViewStart { visible_row: 7 });
        let scrolled = bridge.project_model();
        assert_eq!(scrolled.browser.view_start_row, 7);

        bridge.on_action(NativeUiAction::FocusBrowserRow { visible_row: 18 });
        let refocused = bridge.project_model();
        assert_eq!(refocused.browser.view_start_row, 7);
    }
}

/// Focused projection tests for inline folder editing surfaces.
mod projection_folder_edits {
    use super::*;

    /// Folder-create input updates must refresh the projected draft text immediately.
    #[test]
    fn set_folder_create_input_action_refreshes_projected_model_immediately() {
        let mut bridge = test_bridge(16);
        bridge.controller.ui.sources.folders.inline_edit = Some(InlineFolderEdit {
            kind: InlineFolderEditKind::Create {
                parent: PathBuf::new(),
            },
            name: String::new(),
            focus_requested: true,
            select_all_on_focus_requested: false,
        });
        bridge
            .controller
            .ui
            .sources
            .folders
            .rows
            .push(crate::app::state::FolderRowView {
                path: PathBuf::new(),
                name: String::from("Root"),
                depth: 0,
                has_children: true,
                expanded: true,
                selected: false,
                negated: false,
                hotkey: None,
                is_root: true,
                file_scope_mode: Some(crate::app::state::FolderFileScopeMode::AllDescendants),
            });

        let initial = bridge.project_model();
        let initial_draft = initial
            .sources
            .folder_rows
            .iter()
            .find(|row| row.kind == crate::app_core::actions::NativeFolderRowKind::CreateDraft)
            .expect("folder create draft should be projected");
        assert_eq!(initial_draft.input_value.as_deref(), Some(""));

        bridge.on_action(NativeUiAction::SetFolderCreateInput {
            value: String::from("drums"),
        });

        let updated = bridge.project_model();
        let updated_draft = updated
            .sources
            .folder_rows
            .iter()
            .find(|row| row.kind == crate::app_core::actions::NativeFolderRowKind::CreateDraft)
            .expect("folder create draft should still be projected");
        assert_eq!(updated_draft.input_value.as_deref(), Some("drums"));
    }

    /// Canceling folder-create should remove the draft from the next projected model immediately.
    #[test]
    fn cancel_folder_create_action_refreshes_projected_model_immediately() {
        let mut bridge = test_bridge(16);
        bridge.controller.ui.sources.folders.inline_edit = Some(InlineFolderEdit {
            kind: InlineFolderEditKind::Create {
                parent: PathBuf::new(),
            },
            name: String::from("drums"),
            focus_requested: true,
            select_all_on_focus_requested: false,
        });
        bridge
            .controller
            .ui
            .sources
            .folders
            .rows
            .push(crate::app::state::FolderRowView {
                path: PathBuf::new(),
                name: String::from("Root"),
                depth: 0,
                has_children: true,
                expanded: true,
                selected: false,
                negated: false,
                hotkey: None,
                is_root: true,
                file_scope_mode: Some(crate::app::state::FolderFileScopeMode::AllDescendants),
            });

        let initial = bridge.project_model();
        assert!(initial
            .sources
            .folder_rows
            .iter()
            .any(|row| row.kind == crate::app_core::actions::NativeFolderRowKind::CreateDraft));

        bridge.on_action(NativeUiAction::CancelFolderCreate);

        let updated = bridge.project_model();
        assert!(updated
            .sources
            .folder_rows
            .iter()
            .all(|row| row.kind != crate::app_core::actions::NativeFolderRowKind::CreateDraft));
    }

    /// Starting folder rename should immediately project an inline rename row.
    #[test]
    fn start_folder_rename_action_refreshes_projected_model_immediately() {
        let mut bridge = test_bridge(16);
        bridge
            .controller
            .ui
            .sources
            .folders
            .rows
            .push(crate::app::state::FolderRowView {
                path: PathBuf::new(),
                name: String::from("Root"),
                depth: 0,
                has_children: true,
                expanded: true,
                selected: false,
                negated: false,
                hotkey: None,
                is_root: true,
                file_scope_mode: Some(crate::app::state::FolderFileScopeMode::AllDescendants),
            });
        bridge
            .controller
            .ui
            .sources
            .folders
            .rows
            .push(crate::app::state::FolderRowView {
                path: PathBuf::from("drums"),
                name: String::from("drums"),
                depth: 1,
                has_children: false,
                expanded: false,
                selected: true,
                negated: false,
                hotkey: None,
                is_root: false,
                file_scope_mode: None,
            });
        bridge.controller.ui.sources.folders.focused = Some(1);

        bridge.on_action(NativeUiAction::StartFolderRename);

        let updated = bridge.project_model();
        let draft = updated
            .sources
            .folder_rows
            .iter()
            .find(|row| row.kind == crate::app_core::actions::NativeFolderRowKind::RenameDraft)
            .expect("folder rename draft should be projected");
        assert_eq!(draft.input_value.as_deref(), Some("drums"));
        assert!(draft.select_all_on_focus);
    }
}
