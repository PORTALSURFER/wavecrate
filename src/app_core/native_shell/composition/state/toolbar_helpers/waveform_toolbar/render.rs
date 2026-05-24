use super::super::{
    waveform_toolbar_icon_rect, waveform_toolbar_overlay_icon_color,
    waveform_toolbar_overlay_icon_rect, waveform_toolbar_visual_color,
};
use super::*;

pub(in crate::app_core::native_shell::composition::state) struct WaveformToolbarRenderContext<'a> {
    pub(in crate::app_core::native_shell::composition::state) style: &'a StyleTokens,
    pub(in crate::app_core::native_shell::composition::state) sizing: SizingTokens,
    pub(in crate::app_core::native_shell::composition::state) hovered_hint:
        Option<WaveformToolbarHoverHint>,
    pub(in crate::app_core::native_shell::composition::state) flashed_hint:
        Option<WaveformToolbarHoverHint>,
    pub(in crate::app_core::native_shell::composition::state) motion_wave: f32,
    pub(in crate::app_core::native_shell::composition::state) hide_active_bpm_value_text: bool,
}

pub(in crate::app_core::native_shell::composition::state) fn render_waveform_toolbar_buttons(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    buttons: &[WaveformToolbarButton],
    ctx: WaveformToolbarRenderContext<'_>,
) {
    for button in buttons {
        render_waveform_toolbar_button(primitives, text_runs, button, &ctx);
    }
}

fn render_waveform_toolbar_button(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    button: &WaveformToolbarButton,
    ctx: &WaveformToolbarRenderContext<'_>,
) {
    if ctx.hide_active_bpm_value_text && button.label == "BPM Value" {
        return;
    }

    let label_rect = compute_action_button_text_rect(button.rect, ctx.sizing);
    let button_hint = waveform_toolbar_hover_hint(button.label);
    let is_hovered = button_hint.is_some() && button_hint == ctx.hovered_hint;
    let is_flashed = button_hint.is_some() && button_hint == ctx.flashed_hint;
    let icon_color = waveform_toolbar_visual_color(
        ctx.style,
        button.text_color,
        button.enabled,
        button.active,
        is_hovered,
        is_flashed,
        ctx.motion_wave,
    );
    let main_icon_rect = waveform_toolbar_icon_rect(
        button.rect,
        ctx.sizing,
        button.active,
        is_hovered,
        is_flashed,
    );
    let rendered_main_icon = if let Some(icon) = toolbar_icon_for_button(button) {
        emit_toolbar_svg_icon(primitives, icon, main_icon_rect, icon_color)
    } else {
        false
    };

    if !rendered_main_icon {
        render_waveform_toolbar_text(text_runs, button, label_rect, icon_color, ctx.sizing);
    }
    if let Some(overlay_icon) = button.overlay_icon {
        render_waveform_toolbar_overlay(primitives, button, overlay_icon, icon_color, ctx);
    }
}

fn render_waveform_toolbar_text(
    text_runs: &mut impl TextRunSink,
    button: &WaveformToolbarButton,
    label_rect: Rect,
    color: Rgba8,
    sizing: SizingTokens,
) {
    emit_text(
        text_runs,
        TextRun {
            text: button
                .display_text
                .clone()
                .unwrap_or_else(|| button.label.to_string()),
            position: label_rect.min,
            font_size: sizing.font_meta,
            color,
            max_width: Some(label_rect.width().max(12.0)),
            align: TextAlign::Center,
        },
    );
}

fn render_waveform_toolbar_overlay(
    primitives: &mut impl PrimitiveSink,
    button: &WaveformToolbarButton,
    overlay_icon: WaveformToolbarIcon,
    icon_color: Rgba8,
    ctx: &WaveformToolbarRenderContext<'_>,
) {
    let _ = emit_toolbar_svg_icon(
        primitives,
        overlay_icon,
        waveform_toolbar_overlay_icon_rect(button.rect, ctx.sizing),
        waveform_toolbar_overlay_icon_color(ctx.style, icon_color),
    );
}
