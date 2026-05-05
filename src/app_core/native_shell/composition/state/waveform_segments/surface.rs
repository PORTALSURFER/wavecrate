//! Waveform surface helpers for loading placeholders and rendered images.

use super::*;

const LOADING_WAVEFORM_PROFILE: [f32; 15] = [
    0.08, 0.14, 0.22, 0.31, 0.43, 0.55, 0.66, 0.72, 0.66, 0.55, 0.43, 0.31, 0.22, 0.14, 0.08,
];

/// Emit the neutral loading-state waveform silhouette used before waveform pixels arrive.
pub(in crate::gui::native_shell::state) fn emit_waveform_loading_placeholder(
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    style: &StyleTokens,
    motion_wave: f32,
) {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return;
    }

    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: waveform_plot,
            color: style.surface_base,
        }),
    );

    let silhouette_width = (waveform_plot.width() * 0.72)
        .clamp(48.0, waveform_plot.width())
        .min(waveform_plot.width());
    let left = waveform_plot.min.x + ((waveform_plot.width() - silhouette_width) * 0.5).max(0.0);
    let sample_count = LOADING_WAVEFORM_PROFILE.len();
    if sample_count < 2 {
        return;
    }
    let step = silhouette_width / (sample_count.saturating_sub(1) as f32);
    let rail_width = (step * 0.58).clamp(2.0, 10.0).min(waveform_plot.width());
    let center_y = waveform_plot.min.y + (waveform_plot.height() * 0.5);
    let max_half_height = (waveform_plot.height() * 0.22).clamp(8.0, waveform_plot.height() * 0.36);
    let rail_blend = (0.16 + (motion_wave * 0.08)).clamp(0.16, 0.24);
    let highlight_blend = (0.1 + (motion_wave * 0.07)).clamp(0.10, 0.17);
    let rail_color =
        translucent_overlay_color(style.surface_overlay, style.border_emphasis, rail_blend);
    let highlight_color =
        translucent_overlay_color(style.surface_overlay, style.text_muted, highlight_blend);

    for (index, height_ratio) in LOADING_WAVEFORM_PROFILE.into_iter().enumerate() {
        let half_height = (max_half_height * height_ratio).clamp(2.0, waveform_plot.height() * 0.5);
        let center_x = left + (step * index as f32);
        let half_width = rail_width * 0.5;
        let rail = Rect::from_min_max(
            Point::new(
                (center_x - half_width).clamp(waveform_plot.min.x, waveform_plot.max.x),
                (center_y - half_height).clamp(waveform_plot.min.y, waveform_plot.max.y),
            ),
            Point::new(
                (center_x + half_width).clamp(waveform_plot.min.x, waveform_plot.max.x),
                (center_y + half_height).clamp(waveform_plot.min.y, waveform_plot.max.y),
            ),
        );
        if rail.width() <= 0.0 || rail.height() <= 0.0 {
            continue;
        }
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: rail,
                color: rail_color,
            }),
        );

        let highlight_inset_x = (rail.width() * 0.28).min(2.5);
        let highlight_inset_y = (rail.height() * 0.18).min(2.5);
        let highlight = Rect::from_min_max(
            Point::new(
                (rail.min.x + highlight_inset_x).min(rail.max.x),
                (rail.min.y + highlight_inset_y).min(rail.max.y),
            ),
            Point::new(
                (rail.max.x - highlight_inset_x).max(rail.min.x),
                (rail.max.y - highlight_inset_y).max(rail.min.y),
            ),
        );
        if highlight.width() <= 0.0 || highlight.height() <= 0.0 {
            continue;
        }
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: highlight,
                color: highlight_color,
            }),
        );
    }
}

pub(in crate::gui::native_shell::state) fn push_waveform_image(
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    image: Option<&ImageRgba>,
) {
    let Some(image) = image else {
        return;
    };
    if image.width == 0
        || image.height == 0
        || waveform_plot.width() <= 0.0
        || waveform_plot.height() <= 0.0
    {
        return;
    }

    let has_visible_pixels = image
        .pixels
        .chunks_exact(4)
        .any(|pixel| pixel.get(3).copied().unwrap_or(0) > 0);
    if !has_visible_pixels {
        return;
    }
    emit_primitive(
        primitives,
        Primitive::Image(DrawImage {
            rect: waveform_plot,
            image: std::sync::Arc::new(image.clone()),
        }),
    );
}
