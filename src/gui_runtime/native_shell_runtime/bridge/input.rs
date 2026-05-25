use super::{
    RetainedTextInputTarget, WavecrateRuntimeBridge, render::retained_shell_style_for_viewport,
};
use crate::{
    app_core::{actions::NativeAppBridge, native_shell::composition::ShellLayout},
    gui::types::{Point, Vector2},
    gui_runtime::native_shell_runtime::input_routing::action_from_retained_pointer,
};
use radiant::widgets::{PointerButton, WidgetInput};

impl<B: NativeAppBridge> WavecrateRuntimeBridge<B> {
    /// Translate retained-canvas input into Wavecrate actions or local repaint-only state.
    pub(super) fn handle_retained_canvas_input(&mut self, input: WidgetInput) -> bool {
        match input {
            WidgetInput::PointerMove { position } => self.handle_pointer_move(position),
            WidgetInput::PointerPress {
                position, button, ..
            } => self.handle_pointer_press(position, button),
            WidgetInput::PointerDoubleClick {
                position, button, ..
            } => self.handle_pointer_double_click(position, button),
            WidgetInput::PointerRelease {
                button: PointerButton::Auxiliary,
                ..
            } => self.finish_pointer_drag(true),
            WidgetInput::PointerRelease {
                button: PointerButton::Primary,
                ..
            } => self.finish_pointer_drag(false),
            WidgetInput::PointerRelease { .. }
            | WidgetInput::PointerDrop { .. }
            | WidgetInput::FocusChanged(_) => self.finish_pointer_drag(false),
            WidgetInput::Wheel { .. } => true,
            WidgetInput::KeyPress(key) => self.handle_retained_key_press(key),
            WidgetInput::Character(character) => self.handle_retained_character(character),
            WidgetInput::TextEdit(command) => self.handle_retained_text_edit(command),
        }
    }

    fn handle_pointer_move(&mut self, position: Point) -> bool {
        let layout = self.build_current_layout();
        if self
            .shell_state
            .update_options_panel_drag(&layout, &self.model, position)
        {
            self.local_overlay_surface_refresh = true;
            return true;
        }
        if let Some(action) = self.waveform_pan_drag_action(&layout, position) {
            self.emit_action(action);
            self.local_overlay_surface_refresh = true;
            return true;
        }
        let _effect = self
            .shell_state
            .handle_cursor_move_effect(&layout, &self.model, position);
        self.local_overlay_surface_refresh = true;
        true
    }

    fn handle_pointer_press(&mut self, position: Point, button: PointerButton) -> bool {
        let layout = self.build_current_layout();
        if button == PointerButton::Auxiliary {
            self.begin_waveform_pan_drag(&layout, position);
            self.local_overlay_surface_refresh = true;
            return true;
        }
        if button != PointerButton::Primary {
            return true;
        }
        if self
            .shell_state
            .begin_options_panel_drag(&layout, &self.model, position)
        {
            self.local_overlay_surface_refresh = true;
            return true;
        }
        self.emit_pointer_action(&layout, position);
        true
    }

    fn handle_pointer_double_click(&mut self, position: Point, button: PointerButton) -> bool {
        if button != PointerButton::Primary {
            return true;
        }
        let layout = self.build_current_layout();
        self.emit_pointer_action(&layout, position);
        true
    }

    fn emit_pointer_action(&mut self, layout: &ShellLayout, position: Point) {
        let Some(action) =
            action_from_retained_pointer(layout, &self.model, &mut self.shell_state, position)
        else {
            return;
        };
        self.emit_action(action.into());
        if self.text_input_target != RetainedTextInputTarget::None {
            self.sync_text_edit_from_model();
        }
    }

    fn finish_pointer_drag(&mut self, clear_pan_drag: bool) -> bool {
        if clear_pan_drag {
            self.waveform_pan_drag = None;
        }
        self.shell_state.finish_options_panel_drag();
        self.local_overlay_surface_refresh = true;
        true
    }

    fn build_current_layout(&mut self) -> ShellLayout {
        let viewport = self
            .layout_viewport
            .unwrap_or_else(|| Vector2::new(1280.0, 720.0));
        let style = retained_shell_style_for_viewport(viewport);
        ShellLayout::build_with_style_and_runtime(viewport, &style, &mut self.layout_runtime)
    }
}
