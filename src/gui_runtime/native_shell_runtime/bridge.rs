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
}

/// Private message surface used by the generic runtime before Sempal action reduction.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum SempalRuntimeMessage {
    /// Existing application action emitted by shortcut resolution or translated retained input.
    Action(UiAction),
    /// Raw retained-canvas input that still needs Sempal shell hit-testing.
    RetainedInput(RetainedCanvasInput),
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
        self.update_text_target_after_action(&action);
        self.inner.reduce_action(action);
        let model = self.inner.pull_model_arc();
        self.model = Arc::new(model.as_ref().into());
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
            WidgetKey::Backspace => self.rewrite_retained_text(|value| {
                value.pop();
            }),
            WidgetKey::Delete => true,
            _ => false,
        }
    }

    /// Insert one printable character into the active retained text target.
    fn handle_retained_character(&mut self, character: char) -> bool {
        if character.is_control() {
            return false;
        }
        self.rewrite_retained_text(|value| value.push(character))
    }

    /// Rewrite the active retained text target and emit the matching host action.
    fn rewrite_retained_text(&mut self, rewrite: impl FnOnce(&mut String)) -> bool {
        let Some(mut value) = self.current_text_value() else {
            return false;
        };
        rewrite(&mut value);
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
                        runtime_contract::FolderRowKind::CreateDraft | runtime_contract::FolderRowKind::RenameDraft
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
    }
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
        let motion_model = runtime_contract::NativeMotionModel::from_app_model(&self.model);
        self.shell_state.sync_from_motion_model(&motion_model);
        self.shell_state
            .build_frame_with_style_into(&layout, &style, &self.model, &mut self.frame);
        append_retained_shell_overlays(
            &mut self.shell_state,
            &layout,
            &style,
            &self.model,
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

