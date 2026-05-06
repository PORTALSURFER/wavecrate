use super::*;

/// Sempal-owned generic Radiant runtime bridge.
///
/// This bridge is the ownership boundary for the runtime cutover: Sempal model
/// projection, action reduction, shortcut resolution, repaint wiring, and
/// shutdown artifacts are routed through Radiant's generic runtime API.
pub(super) struct SempalRuntimeBridge<B> {
    pub(super) inner: B,
    model: Arc<runtime_contract::AppModel>,
    shell_state: NativeShellState,
    layout_runtime: ShellLayoutRuntime,
    frame: PaintFrame,
    layout_viewport: Option<Vector2>,
    text_input_target: RetainedTextInputTarget,
    text_edit: RetainedTextEditState,
}

/// Private message surface used by the generic runtime before Sempal action reduction.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum SempalRuntimeMessage {
    /// Existing application action emitted by shortcut resolution or translated retained input.
    Action(UiAction),
    /// Raw retained-canvas input that still needs Sempal shell hit-testing.
    RetainedInput(RetainedCanvasInput),
    /// Local retained text-edit state changed without reducing an app action.
    LocalTextEdit,
}

/// Retained-canvas input normalized out of Radiant widget events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum RetainedCanvasInput {
    /// Pointer hover moved inside the retained Sempal canvas.
    PointerMove {
        /// Logical pointer position in the host surface.
        position: Point,
    },
    /// Pointer press started inside the retained Sempal canvas.
    PointerPress {
        /// Logical pointer position in the host surface.
        position: Point,
        /// Pressed pointer button.
        button: PointerButton,
    },
    /// Pointer press ended inside the retained Sempal canvas.
    PointerRelease {
        /// Logical pointer position in the host surface.
        position: Point,
        /// Released pointer button.
        button: PointerButton,
    },
    /// Runtime focus changed for the retained canvas widget.
    FocusChanged(bool),
    /// Non-text key intent routed to the focused retained canvas.
    KeyPress(WidgetKey),
    /// Printable character routed to the focused retained canvas.
    Character(char),
}

/// Local text-input target tracked after Sempal focus actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum RetainedTextInputTarget {
    /// No retained text input owns keyboard editing.
    #[default]
    None,
    /// Browser search owns text editing.
    BrowserSearch,
    /// Folder search owns text editing.
    FolderSearch,
    /// Browser metadata tag sidebar input owns text editing.
    BrowserPillEditor,
    /// Inline folder create/rename input owns text editing.
    FolderCreate,
    /// Visible confirmation prompt input owns text editing.
    Prompt,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RetainedTextEditState {
    target: RetainedTextInputTarget,
    value: String,
    caret: usize,
    selection_anchor: Option<usize>,
}

impl RetainedTextEditState {
    fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        (anchor != self.caret).then_some((anchor, self.caret))
    }

    fn has_selection(&self) -> bool {
        self.selection().is_some()
    }

    fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    fn sync(&mut self, target: RetainedTextInputTarget, value: String) {
        if self.target != target || self.value != value {
            self.target = target;
            self.value = value;
            self.caret = self.value.chars().count();
            self.selection_anchor = None;
        }
    }

    fn selected_bounds(&self) -> Option<(usize, usize)> {
        self.selection()
            .map(|(a, b)| if a < b { (a, b) } else { (b, a) })
    }

    fn replace_selection(&mut self, replacement: &str) -> bool {
        let Some((start, end)) = self.selected_bounds() else {
            return false;
        };
        replace_char_range(&mut self.value, start, end, replacement);
        self.caret = start + replacement.chars().count();
        self.clear_selection();
        true
    }

    fn insert_char(&mut self, character: char) {
        self.replace_selection("");
        let index = byte_index_for_char(&self.value, self.caret);
        self.value.insert(index, character);
        self.caret += 1;
    }

    fn backspace(&mut self) -> bool {
        if self.replace_selection("") {
            return true;
        }
        if self.caret == 0 {
            return false;
        }
        let end = self.caret;
        let start = self.caret - 1;
        replace_char_range(&mut self.value, start, end, "");
        self.caret = start;
        true
    }

    fn delete(&mut self) -> bool {
        if self.replace_selection("") {
            return true;
        }
        if self.caret >= self.value.chars().count() {
            return false;
        }
        replace_char_range(&mut self.value, self.caret, self.caret + 1, "");
        true
    }

    fn move_caret(&mut self, caret: usize, selecting: bool) {
        let caret = caret.min(self.value.chars().count());
        if selecting {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.caret);
            }
        } else {
            self.clear_selection();
        }
        self.caret = caret;
    }
}

impl<B> SempalRuntimeBridge<B> {
    pub(super) fn new(inner: B) -> Self {
        Self {
            inner,
            model: Arc::new(runtime_contract::AppModel::default()),
            shell_state: NativeShellState::new(),
            layout_runtime: ShellLayoutRuntime::default(),
            frame: PaintFrame::default(),
            layout_viewport: None,
            text_input_target: RetainedTextInputTarget::None,
            text_edit: RetainedTextEditState::default(),
        }
    }

    /// Build the generic retained canvas surface that Radiant owns around Sempal rendering.
    fn generic_shell_surface(
        retained: RetainedSurfaceDescriptor,
    ) -> Arc<UiSurface<SempalRuntimeMessage>> {
        Arc::new(UiSurface::new(SurfaceNode::retained_canvas_mapped(
            1,
            WidgetSizing::fixed(Vector2::new(1280.0, 720.0)),
            retained,
            |message: CanvasMessage| match message {
                CanvasMessage::Input { input } => {
                    SempalRuntimeMessage::RetainedInput(retained_input_from_widget_input(input))
                }
            },
        )))
    }

    #[cfg(test)]
    pub(super) fn capture_gui_automation_snapshot(
        &mut self,
        viewport: [f32; 2],
    ) -> NativeGuiAutomationSnapshot
    where
        B: NativeAppBridge,
    {
        let model = self.inner.project_model();
        capture_gui_automation_snapshot(viewport, model.as_ref())
    }
}

impl<B: NativeAppBridge> SempalRuntimeBridge<B> {
    /// Reduce one app action through the host and refresh the retained compatibility model.
    fn emit_action(&mut self, action: UiAction) {
        self.inner.reduce_action(action.clone());
        let model = self.inner.pull_model_arc();
        self.model = Arc::new(model.as_ref().into());
        self.update_text_target_after_action(&action);
    }

    /// Translate retained-canvas input into Sempal actions or local repaint-only state.
    fn handle_retained_canvas_input(&mut self, input: RetainedCanvasInput) -> bool {
        match input {
            RetainedCanvasInput::PointerMove { position } => {
                let layout = self.build_current_layout();
                let _effect =
                    self.shell_state
                        .handle_cursor_move_effect(&layout, &self.model, position);
                true
            }
            RetainedCanvasInput::PointerPress { position, button } => {
                if button != PointerButton::Primary {
                    return true;
                }
                let layout = self.build_current_layout();
                if let Some(action) = action_from_retained_pointer(
                    &layout,
                    &self.model,
                    &mut self.shell_state,
                    position,
                ) {
                    self.emit_action(action.into());
                    if self.text_input_target != RetainedTextInputTarget::None {
                        self.sync_text_edit_from_model();
                    }
                }
                true
            }
            RetainedCanvasInput::PointerRelease { .. } | RetainedCanvasInput::FocusChanged(_) => {
                true
            }
            RetainedCanvasInput::KeyPress(key) => self.handle_retained_key_press(key),
            RetainedCanvasInput::Character(character) => self.handle_retained_character(character),
        }
    }

    /// Build the current shell layout used by retained input hit-testing.
    fn build_current_layout(&mut self) -> ShellLayout {
        let viewport = self
            .layout_viewport
            .unwrap_or_else(|| Vector2::new(1280.0, 720.0));
        let style = StyleTokens::for_viewport_with_scale(viewport.x, 1.0);
        ShellLayout::build_with_style_and_runtime(viewport, &style, &mut self.layout_runtime)
    }

    /// Handle one non-text key intent routed through the focused retained canvas.
    fn handle_retained_key_press(&mut self, key: WidgetKey) -> bool {
        match key {
            WidgetKey::Enter => {
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
            WidgetKey::Backspace => {
                self.sync_text_edit_from_model();
                if self.text_input_target == RetainedTextInputTarget::BrowserPillEditor
                    && self.text_edit.value.is_empty()
                    && !self.text_edit.has_selection()
                {
                    return self.remove_last_browser_pill_editor_chip();
                }
                self.edit_retained_text(|edit| edit.backspace())
            }
            WidgetKey::Delete => self.edit_retained_text(|edit| edit.delete()),
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

    /// Insert one printable character into the active retained text target.
    fn handle_retained_character(&mut self, character: char) -> bool {
        if character.is_control() {
            return false;
        }
        self.sync_text_edit_from_model();
        if self.text_input_target == RetainedTextInputTarget::BrowserPillEditor && character == ','
        {
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
            return true;
        }
        self.edit_retained_text(|edit| {
            edit.insert_char(character);
            true
        })
    }

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
        let action = match self.text_input_target {
            RetainedTextInputTarget::None => return false,
            RetainedTextInputTarget::BrowserSearch => UiAction::SetBrowserSearch { query: value },
            RetainedTextInputTarget::FolderSearch => UiAction::SetFolderSearch { query: value },
            RetainedTextInputTarget::BrowserPillEditor => {
                UiAction::SetBrowserTagSidebarInput { value }
            }
            RetainedTextInputTarget::FolderCreate => UiAction::SetFolderCreateInput { value },
            RetainedTextInputTarget::Prompt => UiAction::SetPromptInput { value },
        };
        self.emit_action(action);
        true
    }

    fn sync_text_edit_from_model(&mut self) {
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
                Some(self.model.browser.pill_editor.input_value.clone())
            }
            RetainedTextInputTarget::FolderCreate => self
                .model
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
                .map(|row| row.label.clone()),
            RetainedTextInputTarget::Prompt => self.model.confirm_prompt.input_value.clone(),
        }
    }

    /// Keep the local retained text target synchronized with host focus actions.
    fn update_text_target_after_action(&mut self, action: &UiAction) {
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
            UiAction::FocusFolderCreateInput | UiAction::SetFolderCreateInput { .. } => {
                RetainedTextInputTarget::FolderCreate
            }
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
        self.sync_text_edit_from_model();
    }

    fn resolve_retained_text_key_press(&mut self, press: RadiantKeyPress) -> bool {
        if press.alt {
            return false;
        }
        if press.command {
            match press.key {
                RadiantKeyCode::A => {
                    self.text_edit.caret = self.text_edit.value.chars().count();
                    self.text_edit.selection_anchor = Some(0);
                    return true;
                }
                RadiantKeyCode::X => {
                    self.text_edit.replace_selection("");
                    let value = self.text_edit.value.clone();
                    self.emit_text_value(value);
                    return true;
                }
                RadiantKeyCode::C | RadiantKeyCode::V => return true,
                _ => return false,
            }
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

    fn apply_local_text_projection(&self, model: &mut runtime_contract::AppModel) {
        if self.text_input_target != RetainedTextInputTarget::BrowserPillEditor {
            return;
        }
        model.browser.pill_editor.input_focused = true;
        model.browser.pill_editor.input_caret = self.text_edit.caret;
        model.browser.pill_editor.input_selection = self.text_edit.selection();
    }
}

fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

fn replace_char_range(text: &mut String, start: usize, end: usize, replacement: &str) {
    let start = byte_index_for_char(text, start);
    let end = byte_index_for_char(text, end);
    text.replace_range(start..end, replacement);
}

impl<B: NativeAppBridge> RuntimeBridge<SempalRuntimeMessage> for SempalRuntimeBridge<B> {
    /// Project Sempal state into the retained canvas surface visible to Radiant.
    fn project_surface(&mut self) -> Arc<UiSurface<SempalRuntimeMessage>> {
        let model = self.inner.pull_model_arc();
        self.model = Arc::new(model.as_ref().into());
        let dirty = self.inner.take_dirty_segments();
        let revisions = self.inner.take_segment_revisions();
        Self::generic_shell_surface(RetainedSurfaceDescriptor {
            key: 1,
            revision: retained_surface_revision(revisions),
            dirty_mask: u64::from(dirty.bits()),
        })
    }

    /// Apply one generic runtime message and request repaint when retained state changed.
    fn update(&mut self, message: SempalRuntimeMessage) -> Command<SempalRuntimeMessage> {
        match message {
            SempalRuntimeMessage::Action(action) => {
                self.emit_action(action);
                Command::none()
            }
            SempalRuntimeMessage::LocalTextEdit => Command::request_repaint(),
            SempalRuntimeMessage::RetainedInput(input) => {
                let repaint = self.handle_retained_canvas_input(input);
                if repaint {
                    Command::request_repaint()
                } else {
                    Command::none()
                }
            }
        }
    }

    fn resolve_key_press(
        &mut self,
        pending_chord: Option<RadiantKeyPress>,
        press: RadiantKeyPress,
        focus: RadiantFocusSurface,
    ) -> RadiantShortcutResolution<SempalRuntimeMessage> {
        if self.text_input_target != RetainedTextInputTarget::None {
            self.sync_text_edit_from_model();
            if self.resolve_retained_text_key_press(press) {
                return RadiantShortcutResolution {
                    action: Some(SempalRuntimeMessage::LocalTextEdit),
                    handled: true,
                    pending_chord: None,
                };
            }
            return RadiantShortcutResolution::unhandled();
        }
        let resolution = hotkeys::resolve_hotkey_press(
            pending_chord.map(keypress_from_radiant),
            keypress_from_radiant(press),
            sempal_focus_context(&self.model, focus),
        );
        RadiantShortcutResolution {
            action: resolution.action.map(SempalRuntimeMessage::Action),
            handled: resolution.handled,
            pending_chord: resolution.pending_chord.map(keypress_to_radiant),
        }
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn crate::gui::repaint::RepaintSignal>) {
        self.inner.install_repaint_signal(signal);
    }

    /// Render the retained Sempal shell into a paint frame when Radiant requests the canvas.
    fn render_retained_surface(
        &mut self,
        descriptor: RetainedSurfaceDescriptor,
        _rect: Rect,
        viewport: Vector2,
    ) -> Option<PaintFrame> {
        if descriptor.key != 1 {
            return None;
        }
        let style = StyleTokens::for_viewport_with_scale(viewport.x, 1.0);
        if self.layout_viewport != Some(viewport) {
            self.layout_runtime.reset();
            self.layout_viewport = Some(viewport);
        }
        let layout =
            ShellLayout::build_with_style_and_runtime(viewport, &style, &mut self.layout_runtime);
        self.shell_state.sync_from_model(&self.model);
        let mut model = self.model.as_ref().clone();
        self.apply_local_text_projection(&mut model);
        let motion_model = runtime_contract::NativeMotionModel::from_app_model(&model);
        self.shell_state.sync_from_motion_model(&motion_model);
        self.shell_state
            .build_frame_with_style_into(&layout, &style, &model, &mut self.frame);
        append_retained_shell_overlays(
            &mut self.shell_state,
            &layout,
            &style,
            &model,
            &motion_model,
            &mut self.frame,
        );
        Some(self.frame.clone())
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.inner
            .on_runtime_exit()
            .and_then(|artifact| serde_json::to_value(artifact).ok())
    }
}

/// Append state and motion overlays that are local to the retained shell bridge.
fn append_retained_shell_overlays(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &runtime_contract::AppModel,
    motion_model: &runtime_contract::NativeMotionModel,
    frame: &mut PaintFrame,
) {
    let mut overlay = PaintFrame::default();
    shell_state.build_waveform_motion_overlay_into(layout, style, motion_model, &mut overlay);
    append_paint_frame(frame, &overlay);
    shell_state.build_chrome_motion_overlay_into(layout, style, motion_model, &mut overlay);
    append_paint_frame(frame, &overlay);
    shell_state.build_hover_overlay_into(layout, style, model, &mut overlay);
    append_paint_frame(frame, &overlay);
    shell_state.build_focus_overlay_into(layout, style, model, &mut overlay);
    append_paint_frame(frame, &overlay);
}

/// Append one overlay frame into the retained shell paint buffer.
fn append_paint_frame(frame: &mut PaintFrame, overlay: &PaintFrame) {
    frame.primitives.extend(overlay.primitives.iter().cloned());
    frame.text_runs.extend(overlay.text_runs.iter().cloned());
    frame.clear_color = overlay.clear_color;
}

/// Collapse per-segment revisions into the retained canvas revision Radiant observes.
fn retained_surface_revision(revisions: crate::app_core::actions::NativeSegmentRevisions) -> u64 {
    revisions.status_bar
        ^ revisions.browser_frame.rotate_left(7)
        ^ revisions.browser_rows_window.rotate_left(13)
        ^ revisions.map_panel.rotate_left(19)
        ^ revisions.waveform_overlay.rotate_left(29)
        ^ revisions.global_static.rotate_left(37)
}
