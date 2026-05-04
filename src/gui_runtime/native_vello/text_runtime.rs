use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(super) fn activate_waveform_bpm_input(&mut self) {
        self.text_input_target = TextInputTarget::WaveformBpm;
        let text = self
            .waveform_bpm_input_buffer
            .clone()
            .unwrap_or_else(|| self.waveform_bpm_text_from_model());
        self.waveform_bpm_input_buffer = Some(text.clone());
        let mut editor = SingleLineTextEditorState::collapsed_at_end(&text);
        editor.select_all(&text);
        self.text_editor_state = Some(editor);
        self.sync_waveform_bpm_editor_state();
        self.apply_invalidation_scope(RuntimeInvalidationScope::StaticAndOverlays);
    }

    pub(super) fn activate_text_input_target(&mut self, target: TextInputTarget) {
        if matches!(target, TextInputTarget::None | TextInputTarget::WaveformBpm) {
            return;
        }
        let select_all_on_focus = self
            .folder_inline_edit_row()
            .is_some_and(|row| row.select_all_on_focus);
        let current_text = match target {
            TextInputTarget::BrowserSearch => self.model.browser.search_query.clone(),
            TextInputTarget::BrowserPillEditor => {
                self.model.browser.pill_editor.input_value.clone()
            }
            TextInputTarget::FolderSearch => self.model.sources.tree_search_query.clone(),
            TextInputTarget::FolderCreate => self
                .folder_inline_edit_row()
                .and_then(|row| row.input_value.clone())
                .unwrap_or_default(),
            TextInputTarget::PromptInput => self
                .model
                .confirm_prompt
                .input_value
                .clone()
                .unwrap_or_default(),
            TextInputTarget::None | TextInputTarget::WaveformBpm => String::new(),
        };
        self.text_input_target = target;
        self.text_input_buffer = Some(current_text.clone());
        let mut editor = SingleLineTextEditorState::collapsed_at_end(&current_text);
        if select_all_on_focus {
            editor.select_all(&current_text);
        }
        self.text_editor_state = Some(editor);
        self.waveform_bpm_input_buffer = None;
        self.sync_waveform_bpm_editor_state();
        self.sync_browser_search_editor_state();
        self.sync_browser_pill_editor_state();
        self.sync_folder_create_editor_state();
    }

    pub(super) fn deactivate_text_input_target(&mut self) {
        let previous_target = self.text_input_target;
        let was_waveform_bpm = self.text_input_target == TextInputTarget::WaveformBpm;
        self.clear_text_input_target_state();
        self.sync_waveform_bpm_editor_state();
        self.sync_browser_search_editor_state();
        self.sync_browser_pill_editor_state();
        self.sync_folder_create_editor_state();
        if previous_target == TextInputTarget::BrowserSearch {
            self.emit_model_action(UiAction::BlurBrowserSearch);
        }
        if was_waveform_bpm {
            self.apply_invalidation_scope(RuntimeInvalidationScope::StaticAndOverlays);
        }
    }

    pub(super) fn step_waveform_bpm_input(&mut self, delta_tenths: i16) -> bool {
        if self.text_input_target != TextInputTarget::WaveformBpm || delta_tenths == 0 {
            return false;
        }
        let current = self
            .current_text_value()
            .and_then(|value| parse_waveform_bpm_input(&value))
            .unwrap_or(120.0);
        let next = (current + (f32::from(delta_tenths) / 10.0)).max(1.0);
        let next_text = format!("{next:.1}");
        self.waveform_bpm_input_buffer = Some(next_text.clone());
        let mut editor = SingleLineTextEditorState::collapsed_at_end(&next_text);
        editor.select_all(&next_text);
        self.text_editor_state = Some(editor);
        self.sync_waveform_bpm_editor_state();
        self.emit_model_action(UiAction::SetWaveformBpmValue {
            value_tenths: bpm_tenths_from_value(next),
        });
        true
    }

    pub(super) fn build_active_text_field_visual_state(
        &mut self,
        layout: &ShellLayout,
        text_rect: UiRect,
    ) -> Option<TextFieldVisualState> {
        let text = self.current_text_value().unwrap_or_default();
        let font_size =
            StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale)
                .sizing
                .font_meta;
        let available_width = text_rect.width();
        let mut editor = self
            .text_editor_state
            .take()
            .unwrap_or_else(|| SingleLineTextEditorState::collapsed_at_end(&text));
        if let Some(cached) = self.cached_active_text_field_visual_state(
            self.text_input_target,
            &text,
            &editor,
            font_size,
            available_width,
        ) {
            self.text_editor_state = Some(editor);
            return Some(cached);
        }
        let layout_state = build_text_field_layout(
            &mut self.text_renderer,
            &mut editor,
            &text,
            font_size,
            available_width,
        );
        let visual = TextFieldVisualState {
            text: layout_state.visible_text,
            caret_offset: layout_state.caret_offset,
            selection_offsets: layout_state.selection_offsets,
        };
        self.active_text_field_visual_cache = Some(ActiveTextFieldVisualCacheEntry {
            target: self.text_input_target,
            text,
            editor: editor.clone(),
            font_size_bits: font_size.to_bits(),
            available_width_bits: available_width.to_bits(),
            visual: visual.clone(),
        });
        self.text_editor_state = Some(editor);
        Some(visual)
    }

    pub(super) fn sync_waveform_bpm_editor_state(&mut self) {
        let active = self.text_input_target == TextInputTarget::WaveformBpm;
        let display = if active {
            self.waveform_bpm_input_buffer
                .clone()
                .or_else(|| Some(self.waveform_bpm_text_from_model()))
        } else {
            None
        };
        let visual = if active {
            self.with_shell_layout(|this, layout| {
                this.shell_state
                    .waveform_bpm_text_rect(layout, &this.model)
                    .and_then(|text_rect| {
                        this.build_active_text_field_visual_state(layout, text_rect)
                    })
            })
            .flatten()
        } else {
            None
        };
        self.shell_state
            .set_waveform_bpm_editor_state(active, display, visual);
    }

    pub(super) fn sync_browser_search_editor_state(&mut self) {
        if self.text_input_target != TextInputTarget::BrowserSearch {
            self.shell_state.set_browser_search_editor_state(None);
            return;
        }
        let Some(visual) = self.with_shell_layout(|this, layout| {
            this.shell_state
                .browser_search_text_rect(layout, &this.model)
                .and_then(|text_rect| this.build_active_text_field_visual_state(layout, text_rect))
        }) else {
            self.shell_state.set_browser_search_editor_state(None);
            return;
        };
        self.shell_state.set_browser_search_editor_state(visual);
    }

    pub(super) fn sync_browser_pill_editor_state(&mut self) {
        if self.text_input_target != TextInputTarget::BrowserPillEditor {
            self.shell_state.set_browser_pill_editor_visual_state(None);
            return;
        }
        let Some(visual) = self.with_shell_layout(|this, layout| {
            this.shell_state
                .browser_pill_editor_text_rect(layout, &this.model)
                .and_then(|text_rect| this.build_active_text_field_visual_state(layout, text_rect))
        }) else {
            self.shell_state.set_browser_pill_editor_visual_state(None);
            return;
        };
        self.shell_state
            .set_browser_pill_editor_visual_state(visual);
    }

    pub(super) fn sync_folder_create_editor_state(&mut self) {
        if self.text_input_target != TextInputTarget::FolderCreate {
            self.shell_state.set_folder_create_editor_state(None);
            return;
        }
        let Some(visual) = self.with_shell_layout(|this, layout| {
            this.shell_state
                .folder_create_text_rect(layout, &this.model)
                .and_then(|text_rect| this.build_active_text_field_visual_state(layout, text_rect))
        }) else {
            self.shell_state.set_folder_create_editor_state(None);
            return;
        };
        self.shell_state.set_folder_create_editor_state(visual);
    }

    pub(super) fn backspace_text(&mut self) -> bool {
        let Some(value) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        let Some(next) = editor.backspace(&value) else {
            return false;
        };
        self.set_text_value(next)
    }

    pub(super) fn delete_text_forward(&mut self) -> bool {
        let Some(value) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        let Some(next) = editor.delete_forward(&value) else {
            return false;
        };
        self.set_text_value(next)
    }

    pub(super) fn move_text_cursor(&mut self, key: KeyCode, extend_selection: bool) -> bool {
        let Some(text) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        let moved = match key {
            KeyCode::ArrowLeft => editor.move_left(&text, extend_selection),
            KeyCode::ArrowRight => editor.move_right(&text, extend_selection),
            KeyCode::Home => editor.move_home(&text, extend_selection),
            KeyCode::End => editor.move_end(&text, extend_selection),
            _ => false,
        };
        if moved {
            self.sync_text_editor_visual_state_for_target(self.text_input_target);
        }
        moved
    }

    pub(super) fn select_all_text(&mut self) -> bool {
        let Some(text) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        editor.select_all(&text);
        self.sync_text_editor_visual_state_for_target(self.text_input_target);
        true
    }

    pub(super) fn copy_selected_text(&mut self) -> bool {
        let Some(text) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_ref() else {
            return false;
        };
        let Some(selected) = editor.selected_text(&text) else {
            return false;
        };
        self.write_clipboard_text(&selected)
    }

    pub(super) fn cut_selected_text(&mut self) -> bool {
        if !self.copy_selected_text() {
            return false;
        }
        let Some(text) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        if !editor.has_selection() {
            return false;
        }
        let next = editor.replace_selection(&text, "");
        self.set_text_value(next)
    }

    pub(super) fn paste_text(&mut self) -> bool {
        let Some(text) = self.read_clipboard_text() else {
            return false;
        };
        self.append_text(&text)
    }

    pub(super) fn update_text_target_after_action(&mut self, action: &UiAction) {
        match action {
            UiAction::FocusBrowserSearch => {
                self.activate_text_input_target(TextInputTarget::BrowserSearch)
            }
            UiAction::FocusBrowserPillEditorInput => {
                self.activate_text_input_target(TextInputTarget::BrowserPillEditor)
            }
            UiAction::BlurBrowserSearch => self.clear_text_input_target_state(),
            UiAction::FocusFolderSearch { .. } => {
                self.activate_text_input_target(TextInputTarget::FolderSearch)
            }
            UiAction::StartNewFolder
            | UiAction::StartNewFolderAtFolderRow { .. }
            | UiAction::StartNewFolderAtRoot
            | UiAction::FocusFolderCreateInput
            | UiAction::StartFolderRename => {
                self.activate_text_input_target(TextInputTarget::FolderCreate)
            }
            UiAction::ConfirmPrompt
            | UiAction::CancelPrompt
            | UiAction::ConfirmFolderCreate
            | UiAction::CancelFolderCreate
            | UiAction::CommitBrowserPillEditorInput => self.clear_text_input_target_state(),
            _ => {}
        }
        if self.text_input_target != TextInputTarget::WaveformBpm {
            self.waveform_bpm_input_buffer = None;
        }
        if self.text_input_target == TextInputTarget::None {
            self.text_input_buffer = None;
            self.text_editor_state = None;
            self.text_input_drag_active = false;
            self.shell_state.set_browser_search_editor_state(None);
            self.shell_state.set_browser_pill_editor_visual_state(None);
            self.shell_state.set_folder_create_editor_state(None);
        }
        self.sync_waveform_bpm_editor_state();
        self.sync_browser_search_editor_state();
        self.sync_browser_pill_editor_state();
        self.sync_folder_create_editor_state();
    }

    pub(super) fn clear_text_input_target_state(&mut self) {
        if self.text_input_target == TextInputTarget::WaveformBpm {
            self.waveform_bpm_input_buffer = None;
        }
        self.text_input_target = TextInputTarget::None;
        self.text_input_buffer = None;
        self.text_editor_state = None;
        self.active_text_field_visual_cache = None;
        self.text_input_drag_active = false;
    }

    pub(super) fn read_clipboard_text(&mut self) -> Option<String> {
        if let Some(clipboard) = self.clipboard.as_mut()
            && let Ok(text) = clipboard.get_text()
        {
            self.clipboard_fallback_text = text.clone();
            return Some(text);
        }
        if self.clipboard.is_none()
            && let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(text) = clipboard.get_text()
        {
            self.clipboard_fallback_text = text.clone();
            self.clipboard = Some(clipboard);
            return Some(text);
        }
        (!self.clipboard_fallback_text.is_empty()).then(|| self.clipboard_fallback_text.clone())
    }

    pub(super) fn write_clipboard_text(&mut self, text: &str) -> bool {
        self.clipboard_fallback_text = text.to_string();
        if let Some(clipboard) = self.clipboard.as_mut()
            && clipboard.set_text(text.to_string()).is_ok()
        {
            return true;
        }
        if self.clipboard.is_none()
            && let Ok(mut clipboard) = arboard::Clipboard::new()
        {
            let _ = clipboard.set_text(text.to_string());
            self.clipboard = Some(clipboard);
        }
        true
    }

    fn cached_active_text_field_visual_state(
        &self,
        target: TextInputTarget,
        text: &str,
        editor: &SingleLineTextEditorState,
        font_size: f32,
        available_width: f32,
    ) -> Option<TextFieldVisualState> {
        let cached = self.active_text_field_visual_cache.as_ref()?;
        (cached.target == target
            && cached.text == text
            && cached.editor == *editor
            && cached.font_size_bits == font_size.to_bits()
            && cached.available_width_bits == available_width.to_bits())
        .then(|| cached.visual.clone())
    }
}
