use super::*;

/// Approximate the rendered width of one inline browser metadata label.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_text_width(
    text: &str,
    sizing: SizingTokens,
) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    ((text.chars().count() as f32) * (sizing.font_meta * 0.56).max(1.0)).ceil()
}

/// Return the horizontal gap between a sample label and its inline metadata label.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_gap(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.min(6.0).max(3.0)
}

/// Split one inline browser metadata payload into stable per-chip labels.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_labels(
    text: &str,
) -> impl Iterator<Item = &str> + '_ {
    text.split(" · ")
        .map(str::trim)
        .filter(|label| !label.is_empty())
}

/// Materialize inline browser metadata labels once when a cache boundary owns them.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_labels_owned(
    text: &str,
) -> Vec<String> {
    browser_inline_tag_labels(text)
        .map(str::to_owned)
        .collect::<Vec<_>>()
}

/// Return the filled chip width needed for one inline browser metadata label.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_width(
    text: &str,
    sizing: SizingTokens,
) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    browser_inline_tag_text_width(text, sizing) + (browser_inline_tag_chip_padding_x(sizing) * 2.0)
}

/// Compute chip rects for one inline browser metadata cluster.
///
/// During live resize the sample-label band can briefly shrink below the
/// nominal metadata-chip minimum height. In that case the chips compress to
/// the available row height or disappear if no positive-height rect remains.
#[cfg(test)]
pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_rects(
    sample_label: Rect,
    text: &str,
    trailing_reserved_width: f32,
    sizing: SizingTokens,
) -> Vec<Rect> {
    if text.is_empty() || sample_label.width() <= 0.0 || sample_label.height() <= 0.0 {
        return Vec::new();
    }
    let labels = browser_inline_tag_labels_owned(text);
    browser_inline_tag_chip_rects_for_labels(sample_label, &labels, trailing_reserved_width, sizing)
}

/// Return reserved width for a pre-split inline metadata cluster.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_reserved_width_for_labels(
    labels: &[String],
    sizing: SizingTokens,
) -> f32 {
    if labels.is_empty() {
        return 0.0;
    }
    let chips_width = labels
        .iter()
        .map(|label| browser_inline_tag_chip_width(label, sizing))
        .sum::<f32>();
    let chip_gap_count = labels.len().saturating_sub(1) as f32;
    chips_width
        + (chip_gap_count * browser_inline_tag_chip_gap(sizing))
        + browser_inline_tag_gap(sizing)
}

/// Compute chip rects for pre-split inline browser metadata labels.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_rects_for_labels(
    sample_label: Rect,
    labels: &[String],
    trailing_reserved_width: f32,
    sizing: SizingTokens,
) -> Vec<Rect> {
    if labels.is_empty() || sample_label.width() <= 0.0 || sample_label.height() <= 0.0 {
        return Vec::new();
    }
    let chip_gap = browser_inline_tag_chip_gap(sizing);
    let total_width = labels
        .iter()
        .map(|label| browser_inline_tag_chip_width(label, sizing))
        .sum::<f32>()
        + (labels.len().saturating_sub(1) as f32 * chip_gap);
    let right_edge = (sample_label.max.x - trailing_reserved_width).max(sample_label.min.x);
    let start_x = (right_edge - total_width).max(sample_label.min.x);
    let chip_height = browser_inline_tag_chip_height(sample_label, sizing);
    if chip_height <= 0.0 || right_edge <= start_x {
        return Vec::new();
    }
    let min_y = sample_label.min.y + ((sample_label.height() - chip_height) * 0.5).floor();
    let max_y = (min_y + chip_height).min(sample_label.max.y);
    if max_y <= min_y {
        return Vec::new();
    }
    let mut x = start_x;
    labels
        .iter()
        .map(|label| {
            let width = browser_inline_tag_chip_width(label, sizing);
            let rect = Rect::from_min_max(
                Point::new(x, min_y),
                Point::new((x + width).min(right_edge), max_y),
            );
            x = (rect.max.x + chip_gap).min(right_edge);
            rect
        })
        .collect()
}

/// Return the inset text origin for one inline browser metadata chip.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_text_origin(
    chip_rect: Rect,
    sizing: SizingTokens,
) -> Point {
    Point::new(
        chip_rect.min.x + browser_inline_tag_chip_padding_x(sizing),
        chip_rect.min.y + ((chip_rect.height() - sizing.font_meta) * 0.5).floor(),
    )
}

/// Return the desired metadata-chip height for one browser sample label.
///
/// The nominal minimum is `10px`, but cramped rows during resize may provide
/// less vertical space. In that case this helper caps the chip to the
/// available label height instead of panicking on inverted clamp bounds.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_height(
    sample_label: Rect,
    sizing: SizingTokens,
) -> f32 {
    let available_height = sample_label.height().max(0.0);
    if available_height <= 0.0 {
        return 0.0;
    }
    let desired_height =
        (sizing.font_meta + (browser_inline_tag_chip_padding_y(sizing) * 2.0)).round();
    let min_height = 10.0_f32.min(available_height);
    desired_height.clamp(min_height, available_height)
}

pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_padding_x(
    sizing: SizingTokens,
) -> f32 {
    sizing.text_inset_x.min(5.0).max(3.0)
}

pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_padding_y(
    sizing: SizingTokens,
) -> f32 {
    sizing.text_inset_y.min(3.0).max(1.0)
}

pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_gap(
    sizing: SizingTokens,
) -> f32 {
    sizing.border_width.max(1.0) + 2.0
}
