use super::*;

impl<B: NativeAppBridge> WavecrateRuntimeBridge<B> {
    /// Handle one non-text key intent routed through the focused retained canvas.
    pub(in crate::gui_runtime::native_shell_runtime::bridge) fn handle_retained_key_press(
        &mut self,
        key: WidgetKey,
    ) -> bool {
        match key {
            WidgetKey::Enter => self.handle_retained_enter_key(),
            WidgetKey::Backspace => self.handle_retained_backspace_key(),
            WidgetKey::Delete => self.handle_retained_delete_key(),
            WidgetKey::ArrowLeft => self.edit_retained_text(|edit| {
                edit.move_caret(edit.caret.saturating_sub(1), false);
                false
            }),
            WidgetKey::ArrowRight => self.edit_retained_text(|edit| {
                edit.move_caret(edit.caret + 1, false);
                false
            }),
            WidgetKey::Home => self.edit_retained_text(|edit| {
                edit.move_caret(0, false);
                false
            }),
            WidgetKey::End => self.edit_retained_text(|edit| {
                edit.move_caret(edit.value.chars().count(), false);
                false
            }),
            _ => false,
        }
    }

    fn handle_retained_enter_key(&mut self) -> bool {
        if self.model.confirm_prompt.visible {
            self.emit_action(UiAction::ConfirmPrompt);
            return true;
        }
        if self.text_input_target == RetainedTextInputTarget::BrowserPillEditor {
            self.emit_action(UiAction::CommitBrowserTagSidebarInput);
        }
        self.text_input_target = RetainedTextInputTarget::None;
        true
    }

    fn handle_retained_backspace_key(&mut self) -> bool {
        self.sync_text_edit_from_model();
        if self.text_input_target == RetainedTextInputTarget::BrowserPillEditor
            && self.text_edit.value.is_empty()
            && !self.text_edit.has_selection()
        {
            return self.remove_last_browser_pill_editor_chip();
        }
        self.edit_retained_text(|edit| edit.backspace())
    }

    /// Handle one backend-level text edit command from Radiant's native runtime.
    pub(in crate::gui_runtime::native_shell_runtime::bridge) fn handle_retained_text_edit(
        &mut self,
        command: TextEditCommand,
    ) -> bool {
        match command {
            TextEditCommand::MoveLeft { extend_selection } => self.edit_retained_text(|edit| {
                edit.move_caret(edit.caret.saturating_sub(1), extend_selection);
                false
            }),
            TextEditCommand::MoveRight { extend_selection } => self.edit_retained_text(|edit| {
                edit.move_caret(edit.caret + 1, extend_selection);
                false
            }),
            TextEditCommand::MoveWordLeft { extend_selection } => self.edit_retained_text(|edit| {
                edit.move_word_left(extend_selection);
                false
            }),
            TextEditCommand::MoveWordRight { extend_selection } => {
                self.edit_retained_text(|edit| {
                    edit.move_word_right(extend_selection);
                    false
                })
            }
            TextEditCommand::MoveHome { extend_selection } => self.edit_retained_text(|edit| {
                edit.move_caret(0, extend_selection);
                false
            }),
            TextEditCommand::MoveEnd { extend_selection } => self.edit_retained_text(|edit| {
                edit.move_caret(edit.value.chars().count(), extend_selection);
                false
            }),
            TextEditCommand::SelectAll => self.edit_retained_text(|edit| {
                edit.caret = edit.value.chars().count();
                edit.selection_anchor = Some(0);
                false
            }),
            TextEditCommand::InsertText(text) => self.handle_retained_text_insert(&text),
            TextEditCommand::Backspace => self.handle_retained_key_press(WidgetKey::Backspace),
            TextEditCommand::Delete => self.handle_retained_key_press(WidgetKey::Delete),
            TextEditCommand::DeleteWordLeft => {
                self.edit_retained_text(RetainedTextEditState::delete_word_left)
            }
            TextEditCommand::DeleteWordRight => {
                self.edit_retained_text(RetainedTextEditState::delete_word_right)
            }
            TextEditCommand::CutSelection => self.edit_retained_text(|edit| {
                edit.replace_selection("");
                true
            }),
        }
    }

    fn handle_retained_text_insert(&mut self, text: &str) -> bool {
        let mut handled = false;
        for character in text.chars() {
            if self.handle_retained_character(character) {
                handled = true;
            }
        }
        handled
    }

    /// Insert one printable character into the active retained text target.
    pub(in crate::gui_runtime::native_shell_runtime::bridge) fn handle_retained_character(
        &mut self,
        character: char,
    ) -> bool {
        if character.is_control() {
            return false;
        }
        if self.handle_prompt_confirmation_character(character) {
            return true;
        }
        self.sync_text_edit_from_model();
        if self.handle_browser_pill_separator(character) {
            return true;
        }
        self.edit_retained_text(|edit| {
            edit.insert_char(character);
            true
        })
    }

    fn handle_prompt_confirmation_character(&mut self, character: char) -> bool {
        if !self.model.confirm_prompt.visible || self.model.confirm_prompt.input_value.is_some() {
            return false;
        }
        match character {
            'y' | 'Y' => self.emit_action(UiAction::ConfirmPrompt),
            'n' | 'N' => self.emit_action(UiAction::CancelPrompt),
            _ => return false,
        }
        true
    }

    fn handle_browser_pill_separator(&mut self, character: char) -> bool {
        if self.text_input_target != RetainedTextInputTarget::BrowserPillEditor || character != ','
        {
            return false;
        }
        let token = self.text_edit.value.clone();
        if token.split_whitespace().next().is_none() {
            self.text_edit.value.clear();
            self.text_edit.caret = 0;
            self.text_edit.clear_selection();
            return true;
        }
        self.emit_action(UiAction::SetBrowserTagSidebarInput {
            value: token.clone(),
        });
        self.emit_action(UiAction::CommitBrowserTagSidebarInput);
        self.text_input_target = RetainedTextInputTarget::BrowserPillEditor;
        self.text_edit
            .sync(RetainedTextInputTarget::BrowserPillEditor, String::new());
        true
    }

    fn handle_retained_delete_key(&mut self) -> bool {
        if self.model.confirm_prompt.visible {
            if self.text_input_target == RetainedTextInputTarget::Prompt
                && self.model.confirm_prompt.input_value.is_some()
            {
                return self.edit_retained_text(|edit| edit.delete());
            }
            return true;
        }
        if self.text_input_target != RetainedTextInputTarget::None {
            return self.edit_retained_text(|edit| edit.delete());
        }
        match self.model.focus_context {
            runtime_contract::FocusContextModel::ContentList => {
                self.emit_action(UiAction::DeleteBrowserSelection);
                true
            }
            runtime_contract::FocusContextModel::NavigationTree => {
                self.emit_action(UiAction::DeleteFocusedFolder);
                true
            }
            _ => false,
        }
    }

    pub(in crate::gui_runtime::native_shell_runtime::bridge) fn resolve_retained_text_key_press(
        &mut self,
        press: RadiantKeyPress,
    ) -> bool {
        if press.alt {
            return false;
        }
        if press.command {
            return self.resolve_retained_text_command_key(press.key);
        }
        let len = self.text_edit.value.chars().count();
        match press.key {
            RadiantKeyCode::ArrowLeft => {
                self.text_edit
                    .move_caret(self.text_edit.caret.saturating_sub(1), press.shift);
                true
            }
            RadiantKeyCode::ArrowRight => {
                self.text_edit
                    .move_caret((self.text_edit.caret + 1).min(len), press.shift);
                true
            }
            RadiantKeyCode::Home => {
                self.text_edit.move_caret(0, press.shift);
                true
            }
            RadiantKeyCode::End => {
                self.text_edit.move_caret(len, press.shift);
                true
            }
            _ => false,
        }
    }

    fn resolve_retained_text_command_key(&mut self, key: RadiantKeyCode) -> bool {
        match key {
            RadiantKeyCode::A => {
                self.text_edit.caret = self.text_edit.value.chars().count();
                self.text_edit.selection_anchor = Some(0);
                true
            }
            RadiantKeyCode::X => {
                self.text_edit.replace_selection("");
                let value = self.text_edit.value.clone();
                self.emit_text_value(value);
                true
            }
            RadiantKeyCode::C | RadiantKeyCode::V => true,
            _ => false,
        }
    }
}
