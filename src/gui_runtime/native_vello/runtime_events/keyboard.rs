use super::*;
use crate::gui::{
    focus::FocusSurface, list::EditableRowKind, panel::SplitPaneSlot, shortcuts::ShortcutResolution,
};

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    #[cfg(test)]
    pub(crate) fn handle_hotkey_press_for_tests(&mut self, key: KeyCode) {
        self.refresh_cached_model_for_pending_input();
        let handled = self.handle_hotkey_press(key);
        self.finish_keyboard_input(handled);
    }

    #[cfg(test)]
    pub(crate) fn handle_character_key_for_tests(&mut self, key: KeyCode, character: &str) {
        self.refresh_cached_model_for_pending_input();
        let handled = self.handle_character_key(key, character);
        self.finish_keyboard_input(handled);
    }

    #[cfg(test)]
    pub(crate) fn handle_mouse_wheel_for_tests(&mut self, delta: MouseScrollDelta) {
        self.handle_mouse_wheel(delta);
    }

    #[cfg(test)]
    pub(crate) fn handle_enter_for_tests(&mut self) {
        self.refresh_cached_model_for_pending_input();
        let handled = self.handle_enter_key();
        self.finish_keyboard_input(handled);
    }

    #[cfg(test)]
    pub(crate) fn handle_escape_for_tests(&mut self) {
        self.refresh_cached_model_for_pending_input();
        let handled = self.handle_escape_key();
        self.finish_keyboard_input(handled);
    }

    pub(super) fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let _ = self.with_shell_layout(|this, layout| {
            let waveform_zoom_action = this
                .last_cursor
                .and_then(|point| waveform_wheel_zoom_action(layout, &this.model, point, delta));
            let waveform_zoom_emitted = if let Some(action) = waveform_zoom_action {
                this.emit_model_action_with_profile(action, Some(InteractionProfileKind::Timeline));
                this.waveform_view_refresh_pending = true;
                true
            } else {
                false
            };
            if !waveform_zoom_emitted {
                let style = this.cached_style_for_layout(layout);
                if let Some(point) = this.last_cursor.filter(|point| {
                    this.shell_state
                        .folder_panel_contains_point(layout, &this.model, *point)
                }) {
                    let pane = this
                        .shell_state
                        .folder_panel_at_point(layout, &this.model, point)
                        .unwrap_or(this.model.sources.active_folder_pane);
                    if let Some(delta) = folder_wheel_row_delta(
                        &mut this.shell_state,
                        layout,
                        &this.model,
                        point,
                        &style,
                        delta,
                    ) {
                        let viewport_len =
                            this.shell_state
                                .folder_viewport_len(layout, &this.model, pane);
                        let current_view_start = this
                            .shell_state
                            .folder_viewport_start_row(layout, &this.model, pane)
                            .unwrap_or(0);
                        if let Some(view_start_row) = browser_list_view_start_after_wheel(
                            current_view_start,
                            this.model.sources.folder_pane(pane).tree_rows.len(),
                            viewport_len,
                            delta,
                        ) {
                            let _ =
                                this.process_folder_view_start_immediately(pane, view_start_row);
                        }
                        return;
                    }
                }
                let fallback_point = Point::new(
                    (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
                    (layout.browser_rows.min.y + layout.browser_rows.max.y) * 0.5,
                );
                let point = this
                    .last_cursor
                    .filter(|point| layout.browser_panel.contains(*point))
                    .unwrap_or(fallback_point);
                if let Some(delta) =
                    browser_list_wheel_row_delta(layout, &this.model, point, &style, delta)
                {
                    let viewport_len = this.shell_state.browser_viewport_len(layout, &this.model);
                    let current_view_start = this
                        .shell_state
                        .browser_viewport_start_row(layout, &this.model)
                        .unwrap_or(this.model.browser.view_start_row);
                    if let Some(visible_row) = browser_list_view_start_after_wheel(
                        current_view_start,
                        this.model.browser.visible_count,
                        viewport_len,
                        delta,
                    ) {
                        let _ = this.process_wheel_rows_immediately(visible_row);
                    }
                }
            }
        });
    }

    pub(super) fn handle_keyboard_input(&mut self, event: winit::event::KeyEvent) {
        let key = match event.physical_key {
            PhysicalKey::Code(code) => key_code_from_winit(code),
            _ => None,
        };
        let allow_repeat = event.repeat && key.is_some_and(|key| self.allows_key_repeat(key));
        if event.state != ElementState::Pressed || (event.repeat && !allow_repeat) {
            return;
        }
        self.refresh_cached_model_for_pending_input();
        let mut handled = false;
        if matches!(event.logical_key, Key::Named(NamedKey::Escape)) {
            handled = self.handle_escape_key();
        }
        if !handled && matches!(event.logical_key, Key::Named(NamedKey::Backspace)) {
            handled = if self.text_input_target == TextInputTarget::BrowserPillEditor {
                self.backspace_text() || self.remove_last_browser_pill_editor_chip()
            } else {
                self.backspace_text()
            };
        }
        if !handled && matches!(event.logical_key, Key::Named(NamedKey::Delete)) {
            handled = self.delete_text_forward();
        }
        if !handled && matches!(event.logical_key, Key::Named(NamedKey::Enter)) {
            handled = self.handle_enter_key();
        }
        if !handled && matches!(event.logical_key, Key::Named(NamedKey::Tab)) {
            handled = self.text_input_target == TextInputTarget::BrowserPillEditor
                && self.complete_browser_pill_editor_suggestion();
        }
        if !handled && let Some(key) = key {
            handled = self.handle_text_input_key(key);
        }
        if !handled && let Some(key) = key {
            if let Some(text) = event.text.as_ref() {
                handled = self.handle_character_key(key, text);
            } else if self.text_input_target == TextInputTarget::None {
                handled = self.handle_hotkey_press(key);
            }
        }
        self.finish_keyboard_input(handled);
    }

    fn refresh_cached_model_after_folder_create_action(&mut self, action: &UiAction) {
        if folder_create_action_requires_immediate_model_refresh(action) {
            self.refresh_cached_model_for_pending_input();
        }
    }

    fn finish_keyboard_input(&mut self, handled: bool) {
        if handled && !self.frame_state.has_pending_rebuild() {
            self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
        }
    }

    fn handle_hotkey_press(&mut self, key: KeyCode) -> bool {
        let handled_by_shell = matches!(self.model.focus_context, FocusSurface::None)
            && self.shell_state.handle_key(key);
        if handled_by_shell {
            return true;
        }
        let resolution = action_from_key(
            key,
            self.modifiers,
            &self.model,
            self.pending_hotkey_chord,
            |pending_chord, press, focus| {
                self.bridge
                    .resolve_hotkey_press(pending_chord, press, focus)
            },
        );
        self.handle_hotkey_resolution(resolution)
    }

    fn handle_hotkey_resolution(&mut self, resolution: ShortcutResolution<UiAction>) -> bool {
        self.pending_hotkey_chord = resolution.pending_chord;
        let Some(action) = resolution.action else {
            return resolution.handled;
        };
        self.emit_keyboard_action(action);
        true
    }

    fn handle_character_key(&mut self, key: KeyCode, character: &str) -> bool {
        if self.handle_text_input_key(key) || self.handle_text_input_text(character) {
            return true;
        }
        if self.text_input_target != TextInputTarget::None {
            return false;
        }
        self.handle_hotkey_press(key)
    }

    fn handle_text_input_key(&mut self, key: KeyCode) -> bool {
        if self.text_input_target == TextInputTarget::BrowserPillEditor {
            match key {
                KeyCode::ArrowUp => return self.move_browser_pill_editor_suggestion(-1),
                KeyCode::ArrowDown => return self.move_browser_pill_editor_suggestion(1),
                _ => {}
            }
        }
        match key {
            KeyCode::ArrowUp => {
                return self.step_waveform_bpm_input(if self.modifiers.shift_key() {
                    1
                } else {
                    10
                });
            }
            KeyCode::ArrowDown => {
                return self.step_waveform_bpm_input(if self.modifiers.shift_key() {
                    -1
                } else {
                    -10
                });
            }
            _ => {}
        }
        if self.text_input_target == TextInputTarget::None {
            return false;
        }
        if self.move_text_cursor(key, self.modifiers.shift_key()) {
            return true;
        }
        if (self.modifiers.control_key() || self.modifiers.super_key()) && !self.modifiers.alt_key()
        {
            return match key {
                KeyCode::A => self.select_all_text(),
                KeyCode::C => self.copy_selected_text(),
                KeyCode::V => self.paste_text(),
                KeyCode::X => self.cut_selected_text(),
                _ => false,
            };
        }
        false
    }

    fn handle_text_input_text(&mut self, text: &str) -> bool {
        if self.text_input_target == TextInputTarget::None
            || self.modifiers.control_key()
            || self.modifiers.super_key()
            || self.modifiers.alt_key()
        {
            return false;
        }
        let appended: String = text.chars().filter(|ch| !ch.is_control()).collect();
        if appended.is_empty() {
            return false;
        }
        self.append_text(&appended)
    }

    fn handle_enter_key(&mut self) -> bool {
        if matches!(
            self.text_input_target,
            TextInputTarget::BrowserSearch
                | TextInputTarget::BrowserPillEditor
                | TextInputTarget::FolderSearch
                | TextInputTarget::WaveformBpm
        ) {
            if self.text_input_target == TextInputTarget::BrowserPillEditor {
                self.emit_keyboard_action(UiAction::CommitBrowserPillEditorInput);
            }
            self.deactivate_text_input_target();
            return true;
        }
        if self.text_input_target == TextInputTarget::FolderCreate {
            if folder_create_confirm_enabled(&self.model) {
                self.emit_keyboard_action(UiAction::ConfirmFolderCreate);
            }
            return true;
        }
        if self.text_input_target == TextInputTarget::None
            && matches!(self.model.focus_context, FocusSurface::ContentList)
            && self.model.browser.duplicate_cleanup_active
        {
            self.emit_model_action(UiAction::ConfirmBrowserDuplicateCleanup);
            return true;
        }
        false
    }

    fn handle_escape_key(&mut self) -> bool {
        self.pending_hotkey_chord = None;
        if self.model.confirm_prompt.visible {
            self.emit_model_action(UiAction::CancelPrompt);
            self.deactivate_text_input_target();
            return true;
        }
        if self.text_input_target == TextInputTarget::FolderCreate {
            self.emit_keyboard_action(UiAction::CancelFolderCreate);
            return true;
        }
        if self.text_input_target != TextInputTarget::None {
            self.deactivate_text_input_target();
            return true;
        }
        self.emit_keyboard_action(UiAction::HandleEscape);
        true
    }

    fn emit_keyboard_action(&mut self, action: UiAction) {
        let action = rewrite_folder_create_hotkey_action(
            action,
            &self.model,
            self.shell_state.hovered_folder_pane(),
            self.shell_state.hovered_folder_row_index(),
        );
        self.update_text_target_after_action(&action);
        self.emit_model_action(action.clone());
        self.refresh_cached_model_after_folder_create_action(&action);
    }

    /// Move the highlighted browser pill-editor suggestion by one step.
    fn move_browser_pill_editor_suggestion(&mut self, delta: isize) -> bool {
        let count = self.browser_pill_editor_suggestion_count();
        if count == 0 {
            self.browser_pill_editor_suggestion_index = None;
            return true;
        }
        let current = self.browser_pill_editor_suggestion_index.unwrap_or(0);
        let next = if delta < 0 {
            current.checked_sub(1).unwrap_or(count - 1)
        } else {
            (current + 1) % count
        };
        self.browser_pill_editor_suggestion_index = Some(next);
        true
    }

    /// Complete the browser pill-editor input from the highlighted suggestion.
    fn complete_browser_pill_editor_suggestion(&mut self) -> bool {
        let Some(label) = self.browser_pill_editor_suggestion_label() else {
            return true;
        };
        self.set_text_value(label)
    }

    /// Count selectable browser pill-editor suggestions.
    fn browser_pill_editor_suggestion_count(&self) -> usize {
        let editor = &self.model.browser.pill_editor;
        editor.option_pills.len() + usize::from(editor.create_pill.is_some())
    }

    /// Return the highlighted browser pill-editor suggestion label.
    fn browser_pill_editor_suggestion_label(&self) -> Option<String> {
        let editor = &self.model.browser.pill_editor;
        let count = self.browser_pill_editor_suggestion_count();
        if count == 0 {
            return None;
        }
        let index = self.browser_pill_editor_suggestion_index.unwrap_or(0) % count;
        editor
            .option_pills
            .get(index)
            .or_else(|| {
                (index == editor.option_pills.len())
                    .then_some(editor.create_pill.as_ref())
                    .flatten()
            })
            .map(|pill| pill.label.clone())
    }

    /// Remove the trailing selected pill when Backspace starts from an empty input.
    fn remove_last_browser_pill_editor_chip(&mut self) -> bool {
        if self
            .current_text_value()
            .is_some_and(|value| !value.trim().is_empty())
        {
            return false;
        }
        let Some(pill) = self
            .model
            .browser
            .pill_editor
            .option_pills
            .iter()
            .rev()
            .find(|pill| {
                !matches!(
                    pill.state,
                    crate::compat_app_contract::BrowserPillState::Off
                )
            })
        else {
            return true;
        };
        self.emit_keyboard_action(UiAction::ToggleBrowserPillOption {
            label: pill.id.clone(),
        });
        true
    }
}

fn folder_create_confirm_enabled(model: &AppModel) -> bool {
    model
        .sources
        .tree_rows
        .iter()
        .find(|row| {
            matches!(
                row.kind,
                EditableRowKind::CreateDraft | EditableRowKind::RenameDraft
            )
        })
        .and_then(|row| row.input_error.as_ref())
        .is_none_or(|error| error.trim().is_empty())
}

fn rewrite_folder_create_hotkey_action(
    action: UiAction,
    model: &AppModel,
    hovered_folder_pane: Option<SplitPaneSlot>,
    hovered_folder_row_index: Option<usize>,
) -> UiAction {
    if action != UiAction::StartNewFolder
        || !matches!(model.focus_context, FocusSurface::NavigationTree)
    {
        return action;
    }
    let Some(row_index) = hovered_folder_row_index else {
        return action;
    };
    let pane = hovered_folder_pane.unwrap_or(model.sources.active_folder_pane);
    let pane_model = model.sources.folder_pane(pane);
    let Some(row) = pane_model
        .tree_rows
        .get(row_index)
        .or_else(|| model.sources.tree_rows.get(row_index))
    else {
        return action;
    };
    if matches!(
        row.kind,
        EditableRowKind::CreateDraft | EditableRowKind::RenameDraft
    ) {
        return action;
    }
    row.backing_index
        .map(|index| UiAction::StartNewFolderAtFolderRow { index })
        .unwrap_or(action)
}

fn folder_create_action_requires_immediate_model_refresh(action: &UiAction) -> bool {
    matches!(
        action,
        UiAction::StartNewFolder
            | UiAction::StartNewFolderAtFolderRow { .. }
            | UiAction::StartNewFolderAtRoot
            | UiAction::StartFolderRename
            | UiAction::ConfirmFolderCreate
            | UiAction::CancelFolderCreate
    )
}
