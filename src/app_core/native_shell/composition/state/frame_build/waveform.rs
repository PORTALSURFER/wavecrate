use super::StaticFrameCtx;
use super::*;

pub(super) fn render_waveform_static(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    motion_model: Option<&NativeMotionModel>,
) {
    let waveform_inner = ctx.layout.waveform_plot;
    emit_waveform_bpm_grid(state, primitives, waveform_inner, ctx.model, ctx.style);
    push_waveform_image(
        primitives,
        waveform_inner,
        ctx.model.waveform.waveform_image.as_deref(),
    );
    let owned_motion_model;
    let motion_model = if let Some(motion_model) = motion_model {
        motion_model
    } else {
        owned_motion_model = NativeMotionModel::from_app_model(ctx.model);
        &owned_motion_model
    };
    let waveform_toolbar_buttons = waveform_toolbar_buttons(
        ctx.layout,
        ctx.style,
        motion_model,
        state.waveform_bpm_input_active,
        state.waveform_bpm_input_display.as_deref(),
    );
    let waveform_toolbar_left = waveform_toolbar_left_edge(
        &waveform_toolbar_buttons,
        ctx.layout.waveform_header.max.x - ctx.sizing.text_inset_x,
    );
    push_waveform_header_overlay(
        primitives,
        text_runs,
        ctx.layout,
        ctx.style,
        motion_model,
        Some(waveform_toolbar_left - ctx.sizing.action_button_gap),
    );
    render_waveform_toolbar_buttons(
        primitives,
        text_runs,
        ctx.style,
        ctx.sizing,
        &waveform_toolbar_buttons,
        state.hovered_waveform_toolbar_hint,
        state.waveform_toolbar_flash.map(|flash| flash.hint),
        ctx.motion_wave,
        state.waveform_bpm_editor_visual.is_some(),
    );
}

/// Render BPM-aligned waveform grid lines when beat snapping is enabled.
pub(super) fn emit_waveform_bpm_grid(
    state: &mut NativeShellState,
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    model: &AppModel,
    style: &StyleTokens,
) {
    if !model.waveform_chrome.bpm_snap_enabled {
        return;
    }
    let Some(step_micros) = model.waveform.beat_step_micros.filter(|step| *step > 0) else {
        return;
    };
    let view_start = u64::from(model.waveform.view_start_micros.min(1_000_000));
    let view_end = u64::from(
        model
            .waveform
            .view_end_micros
            .min(1_000_000)
            .max(model.waveform.view_start_micros.min(1_000_000)),
    );
    if view_end <= view_start || waveform_plot.width() <= 0.0 {
        return;
    }
    let step_micros = u64::from(step_micros);
    let origin_micros = waveform_bpm_grid_origin_micros(state, model);
    let first_beat_index = first_waveform_bpm_grid_index(view_start, origin_micros, step_micros);
    let view = waveform_view_window_from_bounds(
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
        Some(model.waveform.view_start_nanos),
        Some(model.waveform.view_end_nanos),
    );
    let mut beat_index = first_beat_index;
    let mut beat_micros = origin_micros.saturating_add(beat_index.saturating_mul(step_micros));
    while beat_micros <= view_end {
        let x = waveform_plot_x_for_micros(
            waveform_plot,
            beat_micros as u32,
            view,
            WaveformPixelSnap::Nearest,
        );
        let (line_color, line_width) = waveform_bpm_grid_line_style(style, beat_index);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(x, waveform_plot.min.y),
                    Point::new(
                        (x + line_width).min(waveform_plot.max.x),
                        waveform_plot.max.y,
                    ),
                ),
                color: line_color,
            }),
        );
        beat_index = beat_index.saturating_add(1);
        beat_micros = beat_micros.saturating_add(step_micros);
    }
}

/// Return the BPM grid origin in normalized micro-units, persisting the latest selection start.
fn waveform_bpm_grid_origin_micros(state: &mut NativeShellState, model: &AppModel) -> u64 {
    let identity = (
        model.waveform.loaded_label.clone(),
        model.waveform.waveform_image_signature,
    );
    if state.last_waveform_bpm_grid_identity.as_ref() != Some(&identity) {
        state.last_waveform_bpm_grid_identity = Some(identity);
        state.last_waveform_bpm_grid_origin_micros = None;
    }
    if !model.waveform_chrome.relative_bpm_grid_enabled {
        state.last_waveform_bpm_grid_origin_micros = Some(0);
        return 0;
    }
    if let Some(selection) = model.waveform.selection_milli {
        state.last_waveform_bpm_grid_origin_micros = Some(selection.start_micros);
        return u64::from(selection.start_micros);
    }
    if model.waveform.bpm_grid_origin_micros != 0 {
        let origin = model.waveform.bpm_grid_origin_micros;
        state.last_waveform_bpm_grid_origin_micros = Some(origin);
        return u64::from(origin);
    }
    u64::from(state.last_waveform_bpm_grid_origin_micros.unwrap_or(0))
}

/// Return the first beat index that can appear inside the visible waveform span.
fn first_waveform_bpm_grid_index(view_start: u64, origin_micros: u64, step_micros: u64) -> u64 {
    if view_start <= origin_micros {
        return 0;
    }
    let distance = view_start - origin_micros;
    distance.div_ceil(step_micros)
}

fn waveform_bpm_grid_line_style(style: &StyleTokens, beat_index: u64) -> (Rgba8, f32) {
    if beat_index.is_multiple_of(4) {
        (style.grid_strong, style.sizing.border_width.max(2.0))
    } else {
        (
            blend_color(style.grid_soft, style.text_muted, 0.32),
            style.sizing.border_width.max(1.0),
        )
    }
}
