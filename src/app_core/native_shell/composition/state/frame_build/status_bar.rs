//! Status-bar progress helpers used by the native-shell frame builder.

use super::super::*;

pub(super) fn render_status_bar(
    state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sizing = style.sizing;
    let cached_text = state.cached_status_bar_text(layout, style, model);
    emit_text(
        text_runs,
        TextRun {
            text: cached_text.left_label.clone(),
            position: cached_text.left_text_rect.min,
            font_size: sizing.font_status,
            color: style.text_muted,
            max_width: Some(cached_text.left_text_rect.width().max(36.0)),
            align: TextAlign::Left,
        },
    );
    if cached_text.inline_progress_active {
        emit_text(
            text_runs,
            TextRun {
                text: cached_text.progress_label.clone(),
                position: cached_text.center_text_rect.min,
                font_size: sizing.font_status,
                color: style.text_primary,
                max_width: Some(cached_text.center_text_rect.width().max(36.0)),
                align: TextAlign::Center,
            },
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: cached_text.progress_track_rect,
                color: blend_color(style.grid_soft, style.surface_overlay, 0.35),
            }),
        );
        if let Some(fill_rect) = status_progress_fill_rect(
            cached_text.progress_track_rect,
            model,
            interaction_wave(state.pulse_phase),
        ) {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: fill_rect,
                    color: footer_progress_fill_color(style, model.progress_overlay.total == 0),
                }),
            );
        }
        emit_text(
            text_runs,
            TextRun {
                text: cached_text.progress_counter.clone(),
                position: cached_text.progress_text_rect.min,
                font_size: sizing.font_status,
                color: style.text_muted,
                max_width: Some(cached_text.progress_text_rect.width().max(24.0)),
                align: TextAlign::Center,
            },
        );
    } else {
        emit_text(
            text_runs,
            TextRun {
                text: cached_text.center_label.clone(),
                position: cached_text.center_text_rect.min,
                font_size: sizing.font_status,
                color: style.text_primary,
                max_width: Some(cached_text.center_text_rect.width().max(36.0)),
                align: TextAlign::Center,
            },
        );
    }
    emit_text(
        text_runs,
        TextRun {
            text: cached_text.right_label.clone(),
            position: cached_text.right_text_rect.min,
            font_size: sizing.font_status,
            color: style.text_muted,
            max_width: Some(cached_text.right_text_rect.width().max(36.0)),
            align: TextAlign::Right,
        },
    );
}

/// Resolve the filled portion of the footer progress track.
pub(super) fn status_progress_fill_rect(
    track_rect: Rect,
    model: &AppModel,
    motion_wave: f32,
) -> Option<Rect> {
    if track_rect.width() <= 0.0 || track_rect.height() <= 0.0 {
        return None;
    }
    if model.progress_overlay.total == 0 {
        let segment_width = (track_rect.width() * 0.24).clamp(18.0, track_rect.width().max(18.0));
        let travel = (track_rect.width() - segment_width).max(0.0);
        let min_x = track_rect.min.x + (travel * motion_wave.clamp(0.0, 1.0));
        return Some(Rect::from_min_max(
            Point::new(min_x, track_rect.min.y),
            Point::new(
                (min_x + segment_width).min(track_rect.max.x),
                track_rect.max.y,
            ),
        ));
    }
    let fraction = (model.progress_overlay.completed as f32 / model.progress_overlay.total as f32)
        .clamp(0.0, 1.0);
    let fill_width = (track_rect.width() * fraction).max(0.0);
    if fill_width <= 0.0 {
        return None;
    }
    Some(Rect::from_min_max(
        track_rect.min,
        Point::new(
            track_rect.min.x + fill_width.min(track_rect.width()),
            track_rect.max.y,
        ),
    ))
}

fn footer_progress_fill_color(style: &StyleTokens, indeterminate: bool) -> Rgba8 {
    if indeterminate {
        blend_color(style.accent_mint, style.text_primary, 0.18)
    } else {
        style.accent_mint
    }
}
