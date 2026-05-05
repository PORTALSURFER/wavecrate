use super::*;

/// Return the width reserved for the focused-row similarity trigger.
pub(in crate::gui::native_shell::state) fn browser_similarity_button_reserved_width(
    visible: bool,
    sizing: SizingTokens,
) -> f32 {
    if !visible {
        return 0.0;
    }
    browser_similarity_button_width(sizing) + browser_similarity_button_gap(sizing)
}

/// Return the width reserved for the compact right-edge similarity strength bar.
pub(in crate::gui::native_shell::state) fn browser_similarity_strength_reserved_width(
    visible: bool,
    sizing: SizingTokens,
) -> f32 {
    if !visible {
        return 0.0;
    }
    browser_similarity_strength_width(sizing) + browser_similarity_strength_gap(sizing)
}

/// Return the leading sample-column button rect used to trigger row similarity mode.
pub(in crate::gui::native_shell::state) fn browser_similarity_button_rect(
    row_rect: Rect,
    sizing: SizingTokens,
) -> Option<Rect> {
    if row_rect.width() <= 0.0 || row_rect.height() <= 0.0 {
        return None;
    }
    let sample_column = compute_browser_row_text_layout(row_rect, sizing)
        .columns
        .sample;
    if sample_column.width() <= 0.0 || sample_column.height() <= 0.0 {
        return None;
    }
    let inset = sizing.text_inset_x.min(5.0).max(2.0);
    let width = browser_similarity_button_width(sizing)
        .min((sample_column.width() - (inset * 2.0)).max(0.0));
    let height = browser_similarity_button_height(row_rect, sizing);
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let min_x = sample_column.min.x + inset;
    let min_y = row_rect.min.y + ((row_rect.height() - height) * 0.5).floor();
    Some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(min_x + width, (min_y + height).min(row_rect.max.y - inset)),
    ))
}

/// Return the compact right-edge track rect used to show row similarity strength.
pub(in crate::gui::native_shell::state) fn browser_similarity_strength_track_rect(
    sample_label: Rect,
    sizing: SizingTokens,
) -> Option<Rect> {
    if sample_label.width() <= 0.0 || sample_label.height() <= 0.0 {
        return None;
    }
    let width = browser_similarity_strength_width(sizing).min(sample_label.width().max(0.0));
    let height = browser_similarity_strength_height(sample_label, sizing);
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let inset = sizing.text_inset_x.min(4.0).max(2.0);
    let max_x = (sample_label.max.x - inset).max(sample_label.min.x);
    let min_x = (max_x - width).max(sample_label.min.x);
    let min_y = sample_label.min.y + ((sample_label.height() - height) * 0.5).floor();
    let max_y = (min_y + height).min(sample_label.max.y);
    (max_x > min_x && max_y > min_y).then_some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(max_x, max_y),
    ))
}

/// Return the fill rect used inside the compact similarity strength track.
pub(in crate::gui::native_shell::state) fn browser_similarity_strength_fill_rect(
    track_rect: Rect,
    strength: u8,
) -> Option<Rect> {
    if track_rect.width() <= 0.0 || track_rect.height() <= 0.0 || strength == 0 {
        return None;
    }
    let fill_width = (track_rect.width() * (f32::from(strength) / 255.0))
        .round()
        .clamp(0.0, track_rect.width());
    (fill_width > 0.0).then_some(Rect::from_min_max(
        track_rect.min,
        Point::new(track_rect.min.x + fill_width, track_rect.max.y),
    ))
}

/// Return the centered icon rect used inside the browser similarity button.
pub(in crate::gui::native_shell::state) fn browser_similarity_button_icon_rect(
    button_rect: Rect,
    sizing: SizingTokens,
) -> Rect {
    let side = button_rect
        .width()
        .min(button_rect.height())
        .min((button_rect.height() - (sizing.text_inset_y * 0.8)).max(8.0))
        .clamp(8.0, 16.0);
    let min_x = button_rect.min.x + ((button_rect.width() - side) * 0.5);
    let min_y = button_rect.min.y + ((button_rect.height() - side) * 0.5);
    Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(min_x + side, min_y + side),
    )
}

/// Render the focused-row similarity button using the shared native icon pipeline.
pub(in crate::gui::native_shell::state) fn render_browser_similarity_button(
    primitives: &mut impl PrimitiveSink,
    button_rect: Rect,
    style: &StyleTokens,
    sizing: SizingTokens,
    active: bool,
    icon_color: Rgba8,
) {
    let button_fill = if active {
        translucent_overlay_color(style.surface_overlay, style.highlight_cyan, 0.82)
    } else {
        translucent_overlay_color(style.surface_overlay, style.text_primary, 0.14)
    };
    let button_border = if active {
        blend_color(style.highlight_cyan, style.text_primary, 0.42)
    } else {
        blend_color(style.border_emphasis, style.text_primary, 0.26)
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: button_rect,
            color: button_fill,
        }),
    );
    push_border(primitives, button_rect, button_border, sizing.border_width);
    let _ = emit_toolbar_svg_icon(
        primitives,
        WaveformToolbarIcon::Similarity,
        browser_similarity_button_icon_rect(button_rect, sizing),
        icon_color,
    );
}

fn browser_similarity_button_width(sizing: SizingTokens) -> f32 {
    (sizing.font_meta * 4.4).round().clamp(28.0, 40.0)
}

fn browser_similarity_button_height(row_rect: Rect, sizing: SizingTokens) -> f32 {
    let inset = sizing.row_corner_inset.max(2.0);
    (row_rect.height() - (inset * 2.0))
        .round()
        .clamp(12.0, 20.0)
}

fn browser_similarity_button_gap(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.min(6.0).max(4.0)
}

fn browser_similarity_strength_width(sizing: SizingTokens) -> f32 {
    (sizing.font_meta * 4.2).round().clamp(36.0, 48.0)
}

fn browser_similarity_strength_height(sample_label: Rect, sizing: SizingTokens) -> f32 {
    (sizing.font_meta * 0.84)
        .round()
        .clamp(6.0, sample_label.height().max(6.0).min(10.0))
}

fn browser_similarity_strength_gap(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.min(5.0).max(3.0)
}
