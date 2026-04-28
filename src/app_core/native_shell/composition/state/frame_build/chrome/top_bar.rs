use super::*;
use crate::app::AudioEngineChipStateModel;

pub(super) fn render_top_bar_controls(
    state: &NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let surface = resolve_top_bar_surface_layout(
        ctx.layout.top_bar,
        ctx.sizing,
        &top_bar_surface_content(ctx.model),
    );
    render_volume_controls(state, ctx, primitives, text_runs, &surface);
    render_options_button(
        state,
        ctx,
        primitives,
        text_runs,
        surface.options_button_rect,
    );
    render_update_buttons(ctx, primitives, text_runs, &surface.update_buttons);
}

fn render_volume_controls(
    _state: &NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    surface: &TopBarSurfaceLayout,
) {
    if surface.volume_meter_rect.width() <= 0.0 || surface.volume_meter_rect.height() <= 0.0 {
        return;
    }
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: surface.volume_meter_rect,
            color: ctx.style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        surface.volume_meter_rect,
        ctx.style.border_emphasis,
        ctx.sizing.border_width,
    );
    let volume_level = ctx.model.volume.clamp(0.0, 1.0);
    let fill_width = (surface.volume_meter_rect.width() * volume_level)
        .clamp(1.0, surface.volume_meter_rect.width());
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(
                surface.volume_meter_rect.min,
                Point::new(
                    surface.volume_meter_rect.min.x + fill_width,
                    surface.volume_meter_rect.max.y,
                ),
            ),
            color: blend_color(ctx.style.accent_mint, ctx.style.text_primary, 0.28),
        }),
    );
    emit_text(
        text_runs,
        TextRun {
            text: format!("{volume_level:.2}"),
            position: surface.volume_value_rect.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(surface.volume_value_rect.width().max(20.0)),
            align: TextAlign::Right,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from("Vol"),
            position: surface.volume_label_rect.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(surface.volume_label_rect.width().max(18.0)),
            align: TextAlign::Left,
        },
    );
}

fn render_options_button(
    state: &NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    button_rect: Option<Rect>,
) {
    let Some(button_rect) = button_rect else {
        return;
    };
    let chip_error = ctx.model.audio_engine.chip_state == AudioEngineChipStateModel::Error;
    let chip_label = ctx.model.audio_engine.chip_label.as_str();
    render_status_options_button(
        primitives,
        ctx.style,
        ctx.sizing,
        button_rect,
        chip_label,
        chip_error,
        state.hovered_status_options_button,
        state.status_options_button_flash_ticks > 0,
        ctx.motion_wave,
    );
    render_status_options_button_label(
        text_runs,
        ctx.style,
        ctx.sizing,
        button_rect,
        chip_label,
        chip_error,
        state.hovered_status_options_button,
        state.status_options_button_flash_ticks > 0,
        ctx.motion_wave,
    );
}

fn render_update_buttons(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    buttons: &[crate::gui::native_shell::top_bar_surface::TopBarUpdateButtonLayout],
) {
    for button in buttons {
        let label_rect = compute_action_button_text_rect(button.rect, ctx.sizing);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: if button.spec.enabled {
                    ctx.style.surface_overlay
                } else {
                    ctx.style.control_disabled_fill
                },
            }),
        );
        push_border(
            primitives,
            button.rect,
            if button.spec.enabled {
                blend_color(
                    ctx.style.border_emphasis,
                    ctx.style.text_primary,
                    ctx.style.state_hover_soft,
                )
            } else {
                ctx.style.border
            },
            ctx.sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: button.spec.label.to_string(),
                position: label_rect.min,
                font_size: ctx.sizing.font_meta,
                color: if button.spec.enabled {
                    ctx.style.text_muted
                } else {
                    ctx.style.text_muted
                },
                max_width: Some(label_rect.width().max(12.0)),
                align: TextAlign::Center,
            },
        );
    }
}
