use super::*;

pub(super) struct BrowserPillEditorLayout {
    pub(super) auto_rename_rect: Rect,
    pub(super) input_rect: Rect,
    pub(super) input_text_rect: Rect,
    pub(super) playback_rects: [Rect; 2],
    pub(super) normal_tag_rects: Vec<Rect>,
    pub(super) create_tag_rect: Option<Rect>,
}

pub(super) fn browser_pill_editor_rect(
    rows_rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
) -> Option<Rect> {
    browser_pill_editor_panel_rect(rows_rect, sizing, model)
}

pub(super) fn browser_pill_editor_layout(
    rows_rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
) -> Option<BrowserPillEditorLayout> {
    let rect = browser_pill_editor_rect(rows_rect, sizing, model)?;
    let pad = sizing.panel_inset.max(8.0);
    let content_min_x = rect.min.x + pad;
    let content_max_x = rect.max.x - pad;
    let field_height = sizing.browser_row_height.max(22.0);
    let auto_rename_rect = auto_rename_rect(rect, sizing, content_min_x, content_max_x);
    let input_rect = input_rect(sizing, content_min_x, content_max_x, auto_rename_rect);
    let input_text_rect = input_text_rect(sizing, input_rect);
    let pill_gap = sizing.border_width.max(1.0) + 4.0;
    let playback_rects = playback_rects(
        input_rect,
        content_min_x,
        content_max_x,
        field_height,
        pill_gap,
    );
    let normal_tag_rects = normal_tag_rects(
        model,
        playback_rects,
        content_min_x,
        content_max_x,
        field_height,
        pill_gap,
    );
    let create_tag_rect = create_tag_rect(
        model,
        &normal_tag_rects,
        playback_rects,
        content_min_x,
        content_max_x,
        field_height,
    );
    Some(BrowserPillEditorLayout {
        auto_rename_rect,
        input_rect,
        input_text_rect,
        playback_rects,
        normal_tag_rects,
        create_tag_rect,
    })
}

fn auto_rename_rect(
    rect: Rect,
    sizing: SizingTokens,
    content_min_x: f32,
    content_max_x: f32,
) -> Rect {
    let top = rect.min.y + sizing.panel_inset.max(8.0) + sizing.font_body + 10.0;
    Rect::from_min_max(
        Point::new(content_min_x, top),
        Point::new(content_max_x, top + sizing.browser_row_height.max(22.0)),
    )
}

fn input_rect(
    sizing: SizingTokens,
    content_min_x: f32,
    content_max_x: f32,
    auto_rename_rect: Rect,
) -> Rect {
    let top = auto_rename_rect.max.y + 8.0;
    Rect::from_min_max(
        Point::new(content_min_x, top),
        Point::new(content_max_x, top + sizing.browser_row_height.max(22.0)),
    )
}

fn input_text_rect(sizing: SizingTokens, input_rect: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(
            input_rect.min.x + sizing.text_inset_x,
            input_rect.min.y + sizing.text_inset_y,
        ),
        Point::new(
            input_rect.max.x - sizing.text_inset_x,
            input_rect.max.y - sizing.text_inset_y,
        ),
    )
}

fn playback_rects(
    input_rect: Rect,
    content_min_x: f32,
    content_max_x: f32,
    field_height: f32,
    pill_gap: f32,
) -> [Rect; 2] {
    let two_col_width = ((content_max_x - content_min_x - pill_gap) * 0.5).max(40.0);
    let top = input_rect.max.y + 10.0;
    [
        Rect::from_min_max(
            Point::new(content_min_x, top),
            Point::new(content_min_x + two_col_width, top + field_height),
        ),
        Rect::from_min_max(
            Point::new(content_min_x + two_col_width + pill_gap, top),
            Point::new(content_max_x, top + field_height),
        ),
    ]
}

fn normal_tag_rects(
    model: &AppModel,
    playback_rects: [Rect; 2],
    content_min_x: f32,
    content_max_x: f32,
    field_height: f32,
    pill_gap: f32,
) -> Vec<Rect> {
    let tags_top = playback_rects[0].max.y + 12.0;
    let tag_cols = 3usize;
    let tag_width = ((content_max_x - content_min_x - pill_gap * (tag_cols - 1) as f32)
        / tag_cols as f32)
        .max(40.0);
    let mut rects = Vec::with_capacity(model.browser.pill_editor().option_pills.len());
    for index in 0..model.browser.pill_editor().option_pills.len() {
        let col = index % tag_cols;
        let row = index / tag_cols;
        let min_x = content_min_x + (tag_width + pill_gap) * col as f32;
        let min_y = tags_top + (field_height + pill_gap) * row as f32;
        rects.push(Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new((min_x + tag_width).min(content_max_x), min_y + field_height),
        ));
    }
    rects
}

fn create_tag_rect(
    model: &AppModel,
    normal_tag_rects: &[Rect],
    playback_rects: [Rect; 2],
    content_min_x: f32,
    content_max_x: f32,
    field_height: f32,
) -> Option<Rect> {
    model.browser.pill_editor().create_pill.as_ref().map(|_| {
        let y = normal_tag_rects
            .last()
            .map(|rect| rect.max.y + 12.0)
            .unwrap_or(playback_rects[0].max.y + 12.0);
        Rect::from_min_max(
            Point::new(content_min_x, y),
            Point::new(content_max_x, y + field_height),
        )
    })
}
