use super::*;

pub(super) fn render_browser_row_focus_content(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
    model: &AppModel,
) {
    if !row.focused {
        return;
    }
    let mut label = FocusedBrowserLabel::new(row, style);
    let similarity_button_width = label.reserve_similarity_button(model, style);
    label.reserve_playback_age_marker(row, style, similarity_button_width);
    label.reserve_rating_and_tags(row, style);
    label.emit_missing_marker(text_runs, row, style);
    emit_focused_browser_index_label(text_runs, row, style);
    emit_focused_browser_similarity_button(primitives, row, style, model);
    label.emit_sample_label(text_runs, row, style);
}

struct FocusedBrowserLabel {
    position: Point,
    max_width: f32,
    max_x: f32,
}

impl FocusedBrowserLabel {
    fn new(row: &CachedBrowserRow, style: &StyleTokens) -> Self {
        Self {
            position: row.text_layout.sample_label.min,
            max_width: row
                .text_layout
                .sample_label
                .width()
                .max(style.sizing.font_body),
            max_x: row.text_layout.sample_label.max.x,
        }
    }

    fn reserve_similarity_button(&mut self, model: &AppModel, style: &StyleTokens) -> f32 {
        let reserved_width = browser_similarity_button_reserved_width(
            !model.browser.duplicate_cleanup_active,
            style.sizing,
        );
        self.reserve_label_prefix(reserved_width);
        reserved_width
    }

    fn reserve_playback_age_marker(
        &mut self,
        row: &CachedBrowserRow,
        style: &StyleTokens,
        similarity_button_width: f32,
    ) {
        let reserved_width = browser_playback_age_marker_reserved_width(
            row.rect,
            style.sizing,
            similarity_button_width,
        );
        self.reserve_label_prefix(reserved_width);
    }

    fn reserve_rating_and_tags(&mut self, row: &CachedBrowserRow, style: &StyleTokens) {
        let rating_width =
            browser_rating_indicator_reserved_width(row.rating_level, row.locked, style.sizing);
        let tag_width =
            browser_inline_tag_reserved_width_for_labels(&row.inline_tag_labels, style.sizing);
        self.max_width = (self.max_width - rating_width - tag_width).max(20.0);
    }

    fn emit_missing_marker(
        &mut self,
        text_runs: &mut impl TextRunSink,
        row: &CachedBrowserRow,
        style: &StyleTokens,
    ) {
        if !row.missing {
            return;
        }
        let marker_advance =
            browser_missing_marker_advance(style.sizing.font_body).min(self.max_width.max(0.0));
        emit_text(
            text_runs,
            TextRun {
                text: String::from(BROWSER_MISSING_CONTENT_MARKER),
                position: self.position,
                font_size: style.sizing.font_body,
                color: style.accent_danger,
                max_width: Some(marker_advance),
                align: TextAlign::Left,
            },
        );
        self.reserve_label_prefix(marker_advance);
    }

    fn emit_sample_label(
        self,
        text_runs: &mut impl TextRunSink,
        row: &CachedBrowserRow,
        style: &StyleTokens,
    ) {
        emit_text(
            text_runs,
            TextRun {
                text: row.label.clone(),
                position: self.position,
                font_size: style.sizing.font_body,
                color: focused_browser_row_color(style),
                max_width: Some(self.max_width),
                align: TextAlign::Left,
            },
        );
    }

    fn reserve_label_prefix(&mut self, width: f32) {
        if width <= 0.0 {
            return;
        }
        self.position.x = (self.position.x + width).min(self.max_x);
        self.max_width = (self.max_x - self.position.x).max(4.0);
    }
}

fn emit_focused_browser_index_label(
    text_runs: &mut impl TextRunSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
) {
    emit_text(
        text_runs,
        TextRun {
            text: row.visible_row_label.clone(),
            position: row.text_layout.index_label.min,
            font_size: style.sizing.font_meta,
            color: focused_browser_row_color(style),
            max_width: Some(row.text_layout.index_label.width().max(12.0)),
            align: TextAlign::Right,
        },
    );
}

fn emit_focused_browser_similarity_button(
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
    model: &AppModel,
) {
    if model.browser.duplicate_cleanup_active {
        return;
    }
    let Some(button_rect) = browser_similarity_button_rect(row.rect, style.sizing) else {
        return;
    };
    render_browser_similarity_button(
        primitives,
        button_rect,
        style,
        style.sizing,
        model.browser.similarity_filtered && row.visible_row == 0,
        focused_browser_row_color(style),
    );
}
