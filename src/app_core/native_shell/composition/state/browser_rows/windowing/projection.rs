use super::*;

pub(in crate::gui::native_shell::state) fn browser_rows_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    window_start: usize,
) -> BrowserRowsCacheKey {
    let sizing = style.sizing;
    let rows = model.browser.rows.as_slice();
    let list_rect = browser_rows_list_rect(layout.browser_rows, sizing, model);
    let row_capacity = super::scrollbars::browser_rows_capacity(list_rect, sizing) as u32;
    let content_rect = super::scrollbars::browser_rows_content_rect(
        list_rect,
        model.browser.visible_count,
        sizing,
    );
    let focused_visible_row = rows
        .iter()
        .find(|row| row.focused)
        .map(|row| row.visible_row as u32)
        .unwrap_or(u32::MAX);
    let window_end = (window_start + super::scrollbars::browser_rows_capacity(list_rect, sizing))
        .min(model.browser.rows.len());
    let row_text_revision = browser_row_text_revision(&rows[window_start..window_end]);
    BrowserRowsCacheKey {
        root_min_x: f32_to_bits(layout.root.rect.min.x),
        root_min_y: f32_to_bits(layout.root.rect.min.y),
        root_max_x: f32_to_bits(layout.root.rect.max.x),
        root_max_y: f32_to_bits(layout.root.rect.max.y),
        browser_rows_min_x: f32_to_bits(content_rect.min.x),
        browser_rows_min_y: f32_to_bits(content_rect.min.y),
        browser_rows_max_x: f32_to_bits(content_rect.max.x),
        browser_rows_max_y: f32_to_bits(content_rect.max.y),
        browser_row_height: f32_to_bits(sizing.browser_row_height),
        browser_row_gap: f32_to_bits(sizing.browser_row_gap),
        browser_rows_max_per_column: usize_to_u32(sizing.browser_rows_max_per_column),
        row_capacity,
        browser_row_count: rows.len() as u32,
        focused_visible_row,
        map_active: model.map.active as u32,
        duplicate_cleanup_active: model.browser.duplicate_cleanup_active as u32,
        visible_count: model.browser.visible_count as u32,
        window_start: usize_to_u32(window_start),
        row_text_revision,
        ui_scale: f32_to_bits(layout.ui_scale),
    }
}

pub(in crate::gui::native_shell::state) fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

pub(in crate::gui::native_shell::state) fn f32_to_bits(value: f32) -> u32 {
    value.to_bits()
}

#[cfg(test)]
pub(in crate::gui::native_shell::state) fn rendered_browser_rows(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Vec<CachedBrowserRow> {
    let mut truncation_cache = BrowserRowTruncationCache::default();
    let mut frame_counts = BrowserRowTruncationFrameCounts::default();
    rendered_browser_rows_cached(
        layout,
        model,
        style,
        &mut truncation_cache,
        &mut frame_counts,
    )
}

/// Build rendered browser rows while reusing a retained truncation cache.
#[cfg(test)]
pub(in crate::gui::native_shell::state) fn rendered_browser_rows_cached(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
    truncation_cache: &mut BrowserRowTruncationCache,
    frame_counts: &mut BrowserRowTruncationFrameCounts,
) -> Vec<CachedBrowserRow> {
    rendered_browser_rows_cached_with_window_start(
        layout,
        model,
        style,
        truncation_cache,
        frame_counts,
    )
    .0
}

/// Build rendered browser rows and return the resolved viewport start used.
#[cfg(test)]
pub(in crate::gui::native_shell::state) fn rendered_browser_rows_cached_with_window_start(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
    truncation_cache: &mut BrowserRowTruncationCache,
    frame_counts: &mut BrowserRowTruncationFrameCounts,
) -> (Vec<CachedBrowserRow>, usize) {
    rendered_browser_rows_cached_with_window_start_and_previous(
        layout,
        model,
        style,
        truncation_cache,
        frame_counts,
        None,
    )
}

/// Build rendered browser rows while preserving a prior visible viewport start.
pub(in crate::gui::native_shell::state) fn rendered_browser_rows_cached_with_window_start_and_previous(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
    truncation_cache: &mut BrowserRowTruncationCache,
    frame_counts: &mut BrowserRowTruncationFrameCounts,
    previous_visible_start: Option<usize>,
) -> (Vec<CachedBrowserRow>, usize) {
    let sizing = style.sizing;
    if model.map.active || model.browser.rows.is_empty() {
        return (Vec::new(), 0);
    }

    let (window_start, window_end) = super::viewport::browser_rows_window_bounds_with_previous(
        layout,
        model,
        sizing,
        previous_visible_start,
    );
    let window = &model.browser.rows[window_start..window_end];
    let content_rect = super::scrollbars::browser_rows_content_rect(
        browser_rows_list_rect(layout.browser_rows, sizing, model),
        model.browser.visible_count,
        sizing,
    );
    let row_rects = build_stacked_rows(
        content_rect,
        window.len(),
        sizing.browser_row_gap,
        sizing.browser_row_height,
    );

    let mut rendered = Vec::with_capacity(window.len());
    for (row, rect) in window.iter().zip(row_rects) {
        let row_text_layout = compute_browser_row_text_layout(rect, sizing);
        let rating_reserved_width =
            browser_rating_indicator_reserved_width(row.rating_level, row.locked, sizing);
        let similarity_strength_reserved_width = browser_similarity_strength_reserved_width(
            row.similarity_display_strength.is_some(),
            sizing,
        );
        let similarity_button_reserved_width = browser_similarity_button_reserved_width(
            row.focused && !model.browser.duplicate_cleanup_active,
            sizing,
        );
        let bucket_label_width = browser_inline_tag_max_width(
            row_text_layout.sample_label.width()
                - similarity_button_reserved_width
                - similarity_strength_reserved_width,
            rating_reserved_width,
        );
        let bucket_label_source = row.bucket_label.clone().unwrap_or_default();
        let bucket_label = if bucket_label_width > 0.0 {
            truncate_browser_row_text_cached(
                truncation_cache,
                frame_counts,
                row.visible_row,
                BrowserRowTextKind::Bucket,
                &bucket_label_source,
                bucket_label_width,
                sizing.font_meta,
            )
        } else {
            String::new()
        };
        let inline_tag_labels = browser_inline_tag_labels_owned(&bucket_label);
        let inline_tag_rects = browser_inline_tag_chip_rects_for_labels(
            row_text_layout.sample_label,
            &inline_tag_labels,
            similarity_strength_reserved_width,
            sizing,
        );
        let label_width = (row_text_layout.sample_label.width()
            - rating_reserved_width
            - similarity_button_reserved_width
            - similarity_strength_reserved_width
            - browser_inline_tag_reserved_width_for_labels(&inline_tag_labels, sizing))
        .max(20.0);
        let label = truncate_browser_row_text_cached(
            truncation_cache,
            frame_counts,
            row.visible_row,
            BrowserRowTextKind::Sample,
            &row.label,
            label_width,
            sizing.font_body,
        );
        rendered.push(CachedBrowserRow {
            visible_row: row.visible_row,
            visible_row_label: row.visible_row.to_string(),
            label_rendered_width: browser_approx_text_width(&label, sizing.font_body),
            label,
            bucket_label,
            inline_tag_labels,
            inline_tag_rects,
            text_layout: row_text_layout,
            column: row.column.min(2),
            rating_level: row.rating_level.clamp(-3, 3),
            playback_age_bucket: row.playback_age_bucket,
            similarity_display_strength: row.similarity_display_strength,
            selected: row.selected,
            focused: row.focused,
            missing: row.missing,
            locked: row.locked,
            marked: row.marked,
            rect,
        });
    }
    (rendered, window_start)
}

/// Cap inline browser metadata width so the primary sample label keeps most of the row.
fn browser_inline_tag_max_width(sample_width: f32, rating_reserved_width: f32) -> f32 {
    let sample_width = sample_width.max(0.0);
    let available = (sample_width - rating_reserved_width - 24.0).max(0.0);
    available.min((sample_width * 0.38).clamp(44.0, 120.0))
}
