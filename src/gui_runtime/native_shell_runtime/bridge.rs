use self::pan_drag::WaveformPanDrag;
use self::surface::{generic_shell_surface, retained_surface_revision};
use self::text_edit::{RetainedTextEditState, RetainedTextInputTarget};
use super::*;

mod input;
mod pan_drag;
mod render;
mod surface;
mod text_edit;
mod text_input;

/// Wavecrate-owned generic Radiant runtime bridge.
///
/// This bridge is the ownership boundary for the runtime cutover: Wavecrate model
/// projection, action reduction, shortcut resolution, repaint wiring, and
/// shutdown artifacts are routed through Radiant's generic runtime API.
pub(super) struct WavecrateRuntimeBridge<B> {
    pub(super) inner: B,
    model: Arc<runtime_contract::AppModel>,
    shell_state: NativeShellState,
    layout_runtime: ShellLayoutRuntime,
    static_segments: StaticFrameSegments,
    static_segments_initialized: bool,
    frame: PaintFrame,
    layout_viewport: Option<Vector2>,
    pending_motion_model: Option<runtime_contract::NativeMotionModel>,
    motion_only_surface_refresh: bool,
    local_overlay_surface_refresh: bool,
    pending_surface_descriptor: Option<RetainedSurfaceDescriptor>,
    text_input_target: RetainedTextInputTarget,
    text_edit: RetainedTextEditState,
    waveform_pan_drag: Option<WaveformPanDrag>,
}

/// Private message surface used by the generic runtime before Wavecrate action reduction.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum WavecrateRuntimeMessage {
    /// Existing application action emitted by shortcut resolution or translated retained input.
    Action(UiAction),
    /// Radiant retained-canvas input that still needs Wavecrate shell hit-testing.
    RetainedInput(WidgetInput),
    /// Local retained text-edit state changed without reducing an app action.
    LocalTextEdit,
}

impl<B> WavecrateRuntimeBridge<B> {
    pub(super) fn new(inner: B) -> Self {
        Self {
            inner,
            model: Arc::new(runtime_contract::AppModel::default()),
            shell_state: NativeShellState::new(),
            layout_runtime: ShellLayoutRuntime::default(),
            static_segments: StaticFrameSegments::default(),
            static_segments_initialized: false,
            frame: PaintFrame::default(),
            layout_viewport: None,
            pending_motion_model: None,
            motion_only_surface_refresh: false,
            local_overlay_surface_refresh: false,
            pending_surface_descriptor: None,
            text_input_target: RetainedTextInputTarget::None,
            text_edit: RetainedTextEditState::default(),
            waveform_pan_drag: None,
        }
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

    #[cfg(test)]
    pub(super) fn static_segment_frame_for_tests(
        &self,
        segment: StaticFrameSegment,
    ) -> &PaintFrame {
        self.static_segments.frame(segment)
    }

    #[cfg(test)]
    pub(super) fn set_retained_model_for_tests(&mut self, model: runtime_contract::AppModel) {
        self.model = Arc::new(model);
    }

    #[cfg(test)]
    pub(super) fn retained_model_for_tests(&self) -> &runtime_contract::AppModel {
        self.model.as_ref()
    }

    #[cfg(test)]
    pub(super) fn text_target_value_after_action_for_tests(
        &mut self,
        action: &UiAction,
    ) -> Option<String>
    where
        B: NativeAppBridge,
    {
        self.update_text_target_after_action(action);
        (self.text_input_target != RetainedTextInputTarget::None)
            .then(|| self.text_edit.value.clone())
    }
}

impl<B: NativeAppBridge> WavecrateRuntimeBridge<B> {
    /// Reduce one app action through the host and refresh the retained compatibility model.
    fn emit_action(&mut self, action: UiAction) {
        self.inner.reduce_action(action.clone());
        let model = self.inner.pull_model_arc();
        self.model = Arc::new(model.as_ref().into());
        self.stage_surface_descriptor_from_latest_pull();
        self.update_text_target_after_action(&action);
    }

    fn stage_surface_descriptor_from_latest_pull(&mut self) {
        let dirty = self.inner.take_dirty_segments();
        let revisions = self.inner.take_segment_revisions();
        let dirty_mask = u64::from(dirty.bits());
        let descriptor = RetainedSurfaceDescriptor {
            key: 1,
            revision: retained_surface_revision(revisions),
            dirty_mask,
            volatile: true,
        };
        self.pending_surface_descriptor = Some(
            self.pending_surface_descriptor
                .map(|pending| RetainedSurfaceDescriptor {
                    dirty_mask: pending.dirty_mask | dirty_mask,
                    ..descriptor
                })
                .unwrap_or(descriptor),
        );
    }
}

impl<B: NativeAppBridge> RuntimeBridge<WavecrateRuntimeMessage> for WavecrateRuntimeBridge<B> {
    /// Project Wavecrate state into the retained canvas surface visible to Radiant.
    fn project_surface(&mut self) -> Arc<UiSurface<WavecrateRuntimeMessage>> {
        if let Some(descriptor) = self.pending_surface_descriptor.take() {
            self.local_overlay_surface_refresh = false;
            self.motion_only_surface_refresh = false;
            return generic_shell_surface(descriptor);
        }
        if self.local_overlay_surface_refresh {
            self.local_overlay_surface_refresh = false;
            let revisions = self.inner.take_segment_revisions();
            return generic_shell_surface(RetainedSurfaceDescriptor {
                key: 1,
                revision: retained_surface_revision(revisions),
                dirty_mask: 0,
                volatile: true,
            });
        }
        if self.motion_only_surface_refresh {
            self.motion_only_surface_refresh = false;
            let revisions = self.inner.take_segment_revisions();
            return generic_shell_surface(RetainedSurfaceDescriptor {
                key: 1,
                revision: retained_surface_revision(revisions),
                dirty_mask: 0,
                volatile: true,
            });
        }
        let model = self.inner.pull_model_arc();
        self.model = Arc::new(model.as_ref().into());
        let dirty = self.inner.take_dirty_segments();
        let revisions = self.inner.take_segment_revisions();
        generic_shell_surface(RetainedSurfaceDescriptor {
            key: 1,
            revision: retained_surface_revision(revisions),
            dirty_mask: u64::from(dirty.bits()),
            volatile: true,
        })
    }

    /// Apply one generic runtime message and request repaint when retained state changed.
    fn update(&mut self, message: WavecrateRuntimeMessage) -> Command<WavecrateRuntimeMessage> {
        match message {
            WavecrateRuntimeMessage::Action(action) => {
                self.emit_action(action);
                Command::none()
            }
            WavecrateRuntimeMessage::LocalTextEdit => Command::request_repaint(),
            WavecrateRuntimeMessage::RetainedInput(input) => {
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
    ) -> RadiantShortcutResolution<WavecrateRuntimeMessage> {
        if self.text_input_target != RetainedTextInputTarget::None {
            self.sync_text_edit_from_model();
            if self.resolve_retained_text_key_press(press) {
                return RadiantShortcutResolution {
                    action: Some(WavecrateRuntimeMessage::LocalTextEdit),
                    handled: true,
                    pending_chord: None,
                };
            }
            return RadiantShortcutResolution::unhandled();
        }
        let resolution = hotkeys::resolve_hotkey_press(
            pending_chord.map(keypress_from_radiant),
            keypress_from_radiant(press),
            wavecrate_focus_context(&self.model, focus),
        );
        RadiantShortcutResolution {
            action: resolution.action.map(WavecrateRuntimeMessage::Action),
            handled: resolution.handled,
            pending_chord: resolution.pending_chord.map(keypress_to_radiant),
        }
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn crate::gui::repaint::RepaintSignal>) {
        self.inner.install_repaint_signal(signal);
    }

    fn native_file_drop(
        &mut self,
        drop: radiant::runtime::NativeFileDrop,
    ) -> Command<WavecrateRuntimeMessage> {
        let phase = match drop.phase {
            radiant::runtime::NativeFileDropPhase::Hover => NativeFileDropPhase::Hover,
            radiant::runtime::NativeFileDropPhase::Cancel => NativeFileDropPhase::Cancel,
            radiant::runtime::NativeFileDropPhase::Drop => NativeFileDropPhase::Drop,
        };
        self.inner.handle_native_file_drop(NativeFileDropEvent {
            phase,
            path: drop.path,
            position: drop.position.map(|position| (position.x, position.y)),
        });
        self.model = Arc::new(self.inner.pull_model_arc().as_ref().into());
        self.stage_surface_descriptor_from_latest_pull();
        Command::request_repaint()
    }

    fn needs_animation(&mut self) -> bool {
        if let Some(motion_model) = self.inner.pull_motion_model() {
            let motion_model: runtime_contract::NativeMotionModel = motion_model.into();
            self.shell_state.sync_from_motion_model(&motion_model);
            if self.shell_state.needs_animation() {
                self.motion_only_surface_refresh = true;
                self.pending_motion_model = Some(motion_model);
            } else {
                self.motion_only_surface_refresh = false;
                self.pending_motion_model = None;
            }
        } else {
            self.motion_only_surface_refresh = false;
            self.pending_motion_model = None;
        }
        self.shell_state.needs_animation()
    }

    /// Render the retained Wavecrate shell into a paint frame when Radiant requests the canvas.
    fn render_retained_surface(
        &mut self,
        descriptor: RetainedSurfaceDescriptor,
        _rect: Rect,
        viewport: Vector2,
    ) -> Option<PaintFrame> {
        self.render_retained_surface_frame(descriptor, viewport)
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.inner
            .on_runtime_exit()
            .and_then(|artifact| serde_json::to_value(artifact).ok())
    }
}
