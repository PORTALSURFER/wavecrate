use super::*;

mod browser_toolbar;
mod status;

use self::status::push_status_right_motion_overlay;

impl NativeShellState {
    /// Build only heavier motion-driven chrome overlays into reusable buffers.
    pub(crate) fn build_chrome_motion_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
        frame: &mut NativeViewFrame,
    ) {
        let motion_wave = interaction_wave(self.pulse_phase);
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;

        self.render_waveform_toolbar_motion(primitives, text_runs, layout, style, model);
        self.render_browser_toolbar_motion(primitives, style, model, motion_wave);
        self.render_shell_button_motion(primitives, text_runs, layout, style, motion_wave);
        render_browser_tab_motion(primitives, layout, style, model, motion_wave);
        push_status_right_motion_overlay(primitives, text_runs, layout, style, &model.status_right);

        frame.clear_color = style.clear_color;
    }

    fn render_waveform_toolbar_motion(
        &self,
        primitives: &mut impl PrimitiveSink,
        text_runs: &mut impl TextRunSink,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
    ) {
        let sizing = style.sizing;
        let motion_wave = interaction_wave(self.pulse_phase);
        let buttons = waveform_toolbar_buttons(
            layout,
            style,
            model,
            self.waveform_bpm_input_active,
            self.waveform_bpm_input_display.as_deref(),
        );
        let toolbar_left = waveform_toolbar_left_edge(
            &buttons,
            layout.waveform_header.max.x - sizing.text_inset_x,
        );
        push_waveform_header_overlay(
            primitives,
            text_runs,
            layout,
            style,
            model,
            Some(toolbar_left - sizing.action_button_gap),
        );
        self.render_waveform_bpm_motion(primitives, text_runs, style, &buttons, motion_wave);
        render_waveform_toolbar_buttons(
            primitives,
            text_runs,
            &buttons,
            WaveformToolbarRenderContext {
                style,
                sizing,
                hovered_hint: self.hovered_waveform_toolbar_hint,
                flashed_hint: self.waveform_toolbar_flash.map(|flash| flash.hint),
                motion_wave,
                hide_active_bpm_value_text: self.waveform_bpm_editor_visual.is_some(),
            },
        );
    }

    fn render_waveform_bpm_motion(
        &self,
        primitives: &mut impl PrimitiveSink,
        text_runs: &mut impl TextRunSink,
        style: &StyleTokens,
        buttons: &[WaveformToolbarButton],
        motion_wave: f32,
    ) {
        if !self.waveform_bpm_input_active {
            return;
        }
        let sizing = style.sizing;
        let Some(bpm_button) = buttons.iter().find(|button| button.label == "BPM Value") else {
            return;
        };
        if let Some(visual) = self.waveform_bpm_editor_visual.as_ref() {
            render_active_waveform_bpm_editor(
                primitives,
                text_runs,
                style,
                sizing,
                bpm_button.rect,
                compute_action_button_text_rect(bpm_button.rect, sizing),
                visual,
            );
            return;
        }
        render_waveform_bpm_input_focus_overlay(
            primitives,
            style,
            sizing,
            bpm_button.rect,
            motion_wave,
        );
    }

    fn render_shell_button_motion(
        &self,
        primitives: &mut impl PrimitiveSink,
        text_runs: &mut impl TextRunSink,
        layout: &ShellLayout,
        style: &StyleTokens,
        motion_wave: f32,
    ) {
        self.render_source_add_button_motion(primitives, text_runs, layout, style, motion_wave);
        self.render_status_options_button_motion(primitives, layout, style, motion_wave);
    }

    fn render_source_add_button_motion(
        &self,
        primitives: &mut impl PrimitiveSink,
        text_runs: &mut impl TextRunSink,
        layout: &ShellLayout,
        style: &StyleTokens,
        motion_wave: f32,
    ) {
        let Some(button_rect) = source_add_button_rect(layout.sidebar_header, style.sizing) else {
            return;
        };
        let hovered = self.hovered_source_add_button;
        let flashed = self.source_add_button_flash_ticks > 0;
        if hovered || flashed {
            render_source_add_button_overlay(
                primitives,
                text_runs,
                style,
                style.sizing,
                button_rect,
                hovered,
                flashed,
                motion_wave,
            );
        }
    }

    fn render_status_options_button_motion(
        &self,
        primitives: &mut impl PrimitiveSink,
        layout: &ShellLayout,
        style: &StyleTokens,
        motion_wave: f32,
    ) {
        let Some(button_rect) = top_bar_options_button_rect(layout.top_bar, style.sizing) else {
            return;
        };
        let hovered = self.hovered_status_options_button;
        let flashed = self.status_options_button_flash_ticks > 0;
        if hovered || flashed {
            render_status_options_button(
                primitives,
                style,
                style.sizing,
                button_rect,
                "",
                self.status_options_button_error,
                hovered,
                flashed,
                motion_wave,
            );
        }
    }
}

fn render_browser_tab_motion(
    primitives: &mut impl PrimitiveSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
    motion_wave: f32,
) {
    let tabs = resolve_browser_tabs_surface_layout(
        layout.browser_tabs,
        style.sizing,
        &BrowserTabsSurfaceContent {
            items_label: String::new(),
            map_label: String::new(),
        },
    );
    let selected_fill = blend_color(
        style.surface_overlay,
        style.bg_tertiary,
        style.state_selected_blend + (motion_wave * 0.1),
    );
    let (samples_fill, map_fill) = if model.map_active {
        (style.surface_base, selected_fill)
    } else {
        (selected_fill, style.surface_base)
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: tabs.items,
            color: samples_fill,
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: tabs.map,
            color: map_fill,
        }),
    );
    push_border(
        primitives,
        tabs.items,
        style.border,
        style.sizing.border_width,
    );
    push_border(
        primitives,
        tabs.map,
        blend_color(style.accent_mint, style.text_primary, 0.42),
        style.sizing.border_width,
    );
}
