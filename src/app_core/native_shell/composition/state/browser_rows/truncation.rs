//! Text truncation and text-revision helpers for browser rows.

use super::*;

pub(in crate::gui::native_shell::state) fn truncate_to_width(
    text: &str,
    max_width: f32,
    font_size: f32,
) -> String {
    let max_width = max_width.max(0.0);
    let approx_char_width = browser_approx_text_width("W", font_size).max(1.0);
    let max_chars = (max_width / approx_char_width).floor() as usize;
    if max_chars == 0 {
        return String::new();
    }
    let mut chars = text.chars();
    let mut output = String::with_capacity(max_chars);
    for _ in 0..max_chars {
        match chars.next() {
            Some(ch) => output.push(ch),
            None => return output,
        }
    }
    if chars.next().is_none() {
        return output;
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let truncated_chars = max_chars.saturating_sub(3);
    let new_len = output
        .char_indices()
        .nth(truncated_chars)
        .map_or(output.len(), |(idx, _)| idx);
    output.truncate(new_len);
    output.push_str("...");
    output
}

/// Approximate one-line browser text width using the shell's truncation heuristic.
pub(in crate::gui::native_shell::state) fn browser_approx_text_width(
    text: &str,
    font_size: f32,
) -> f32 {
    let approx_char_width = (font_size * 0.56).max(1.0);
    text.chars().count() as f32 * approx_char_width
}

/// Build a truncation-cache invalidation key from the current layout/style/row-revision state.
pub(in crate::gui::native_shell::state) fn browser_row_truncation_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    rows_key: BrowserRowsCacheKey,
) -> BrowserRowTruncationCacheKey {
    let content_rect = browser_rows_content_rect(
        layout.browser_rows,
        rows_key.visible_count as usize,
        style.sizing,
    );
    BrowserRowTruncationCacheKey {
        browser_rows_min_x: f32_to_bits(content_rect.min.x),
        browser_rows_min_y: f32_to_bits(content_rect.min.y),
        browser_rows_max_x: f32_to_bits(content_rect.max.x),
        browser_rows_max_y: f32_to_bits(content_rect.max.y),
        font_body_bits: f32_to_bits(style.sizing.font_body),
        font_meta_bits: f32_to_bits(style.sizing.font_meta),
        ui_scale: f32_to_bits(layout.ui_scale),
        row_text_revision: rows_key.row_text_revision,
    }
}

/// Resolve one truncated browser-row text string from cache or compute it on miss.
pub(in crate::gui::native_shell::state) fn truncate_browser_row_text_cached(
    truncation_cache: &mut BrowserRowTruncationCache,
    frame_counts: &mut BrowserRowTruncationFrameCounts,
    row_id: usize,
    text_kind: BrowserRowTextKind,
    text: &str,
    max_width: f32,
    font_size: f32,
) -> String {
    let key = BrowserRowTruncationEntryKey {
        row_id: usize_to_u32(row_id),
        width_bucket: truncation_width_bucket(max_width),
        font_size_bucket: truncation_font_size_bucket(font_size),
        text_kind,
    };
    truncation_cache.resolve(key, text, max_width, font_size, frame_counts)
}

/// Quantize truncation width inputs into stable cache buckets.
pub(in crate::gui::native_shell::state) fn truncation_width_bucket(width: f32) -> u16 {
    ((width.max(0.0) * 2.0).round().clamp(0.0, u16::MAX as f32)) as u16
}

/// Quantize truncation font-size inputs into stable cache buckets.
pub(in crate::gui::native_shell::state) fn truncation_font_size_bucket(font_size: f32) -> u16 {
    ((font_size.max(0.0) * 64.0)
        .round()
        .clamp(0.0, u16::MAX as f32)) as u16
}

/// Hash visible browser-row labels into one revision fingerprint.
pub(in crate::gui::native_shell::state) fn browser_row_text_revision(
    rows: &[BrowserRowModel],
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    rows.len().hash(&mut hasher);
    for row in rows {
        row.visible_row.hash(&mut hasher);
        row.label.hash(&mut hasher);
        row.bucket_label.hash(&mut hasher);
        row.column.hash(&mut hasher);
        row.rating_level.hash(&mut hasher);
        row.playback_age_bucket.hash(&mut hasher);
        row.similarity_display_strength.hash(&mut hasher);
        row.locked.hash(&mut hasher);
        row.marked.hash(&mut hasher);
        row.processing_state.hash(&mut hasher);
    }
    hasher.finish()
}
