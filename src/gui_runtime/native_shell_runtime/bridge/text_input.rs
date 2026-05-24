use super::*;

mod commands;

impl<B: NativeAppBridge> WavecrateRuntimeBridge<B> {
    /// Rewrite the active retained text target and emit the matching host action.
    fn edit_retained_text(
        &mut self,
        rewrite: impl FnOnce(&mut RetainedTextEditState) -> bool,
    ) -> bool {
        if self.text_input_target == RetainedTextInputTarget::None {
            return false;
        }
        self.sync_text_edit_from_model();
        let changed = rewrite(&mut self.text_edit);
        if !changed {
            return true;
        }
        let value = self.text_edit.value.clone();
        self.emit_text_value(value);
        true
    }

    pub(super) fn sync_text_edit_from_model(&mut self) {
        if let Some(value) = self.current_text_value() {
            self.text_edit.sync(self.text_input_target, value);
        }
    }

    fn remove_last_browser_pill_editor_chip(&mut self) -> bool {
        let Some(pill) = self.model.browser.pill_editor().accepted_pills.last() else {
            return true;
        };
        self.emit_action(UiAction::ToggleBrowserSidebarNormalTag {
            label: pill.id.clone(),
        });
        true
    }

    /// Return the current projected text for the active retained text target.
    fn current_text_value(&self) -> Option<String> {
        match self.text_input_target {
            RetainedTextInputTarget::None => None,
            RetainedTextInputTarget::BrowserSearch => Some(self.model.browser.search_query.clone()),
            RetainedTextInputTarget::FolderSearch => {
                Some(self.model.sources.tree_search_query.clone())
            }
            RetainedTextInputTarget::BrowserPillEditor => {
                Some(self.model.browser.pill_editor().input_value.clone())
            }
            RetainedTextInputTarget::FolderCreate => self.folder_inline_editor_value(),
            RetainedTextInputTarget::Prompt => self.model.confirm_prompt.input_value.clone(),
        }
    }

    fn folder_inline_editor_value(&self) -> Option<String> {
        self.model
            .sources
            .tree_rows
            .iter()
            .find(|row| {
                matches!(
                    row.kind,
                    runtime_contract::FolderRowKind::CreateDraft
                        | runtime_contract::FolderRowKind::RenameDraft
                )
            })
            .and_then(|row| row.input.value.clone())
    }

    /// Keep the local retained text target synchronized with host focus actions.
    pub(super) fn update_text_target_after_action(&mut self, action: &UiAction) {
        self.text_input_target = match action {
            UiAction::FocusBrowserSearch | UiAction::SetBrowserSearch { .. } => {
                RetainedTextInputTarget::BrowserSearch
            }
            UiAction::FocusFolderSearch | UiAction::SetFolderSearch { .. } => {
                RetainedTextInputTarget::FolderSearch
            }
            UiAction::FocusBrowserTagSidebarInput | UiAction::SetBrowserTagSidebarInput { .. } => {
                RetainedTextInputTarget::BrowserPillEditor
            }
            UiAction::StartNewFolder
            | UiAction::StartNewFolderAtFolderRow { .. }
            | UiAction::StartNewFolderAtRoot
            | UiAction::StartFolderRename
            | UiAction::FocusFolderCreateInput
            | UiAction::SetFolderCreateInput { .. } => RetainedTextInputTarget::FolderCreate,
            UiAction::SetPromptInput { .. } => RetainedTextInputTarget::Prompt,
            UiAction::BlurBrowserSearch
            | UiAction::CommitBrowserTagSidebarInput
            | UiAction::ConfirmFolderCreate
            | UiAction::CancelFolderCreate
            | UiAction::ConfirmPrompt
            | UiAction::CancelPrompt
            | UiAction::HandleEscape => RetainedTextInputTarget::None,
            _ => self.text_input_target,
        };
        if self.text_input_target == RetainedTextInputTarget::FolderCreate
            && self.folder_inline_editor_value().is_none()
        {
            self.text_input_target = RetainedTextInputTarget::None;
        }
        self.sync_text_edit_from_model();
    }

    fn emit_text_value(&mut self, value: String) {
        let action = match self.text_input_target {
            RetainedTextInputTarget::None => return,
            RetainedTextInputTarget::BrowserSearch => UiAction::SetBrowserSearch { query: value },
            RetainedTextInputTarget::FolderSearch => UiAction::SetFolderSearch { query: value },
            RetainedTextInputTarget::BrowserPillEditor => {
                UiAction::SetBrowserTagSidebarInput { value }
            }
            RetainedTextInputTarget::FolderCreate => UiAction::SetFolderCreateInput { value },
            RetainedTextInputTarget::Prompt => UiAction::SetPromptInput { value },
        };
        self.emit_action(action);
    }

    pub(super) fn apply_local_text_projection(&self, model: &mut runtime_contract::AppModel) {
        if self.text_input_target != RetainedTextInputTarget::BrowserPillEditor {
            return;
        }
        model.browser.pill_editor.input_focused = true;
        model.browser.pill_editor.input_caret = self.text_edit.caret;
        model.browser.pill_editor.input_selection = self.text_edit.selection();
    }
}
