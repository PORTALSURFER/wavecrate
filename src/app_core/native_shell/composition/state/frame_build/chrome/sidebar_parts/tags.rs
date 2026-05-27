use super::*;
use crate::app_core::native_shell::runtime_contract::{BrowserPillModel, BrowserPillState};

/// Render the left-sidebar tag editor panel.
pub(super) fn render_sidebar_tags(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let rect = sidebar_workspace_sections(ctx.layout, ctx.style).tags;
    if rect.width() <= 1.0 || rect.height() <= 1.0 {
        return;
    }
    render_section_panel(ctx, primitives, rect);
    render_section_title(ctx, text_runs, rect, "TAGS");
    render_tag_library_toggle(ctx, primitives, text_runs, rect);

    let input = sidebar_tag_input_rect(rect, ctx.sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: input,
            color: ctx.style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        input,
        ctx.style.border_emphasis,
        ctx.sizing.border_width,
    );
    render_input_selection_and_caret(ctx, primitives, input);
    emit_text(
        text_runs,
        TextRun {
            text: if ctx.model.browser.pill_editor().input_value.is_empty() {
                String::from("Add tag")
            } else {
                ctx.model.browser.pill_editor().input_value.clone()
            },
            position: sidebar_tag_input_text_rect(input, ctx.sizing).min,
            font_size: ctx.sizing.font_meta,
            color: if ctx.model.browser.pill_editor().input_value.is_empty() {
                ctx.style.text_muted
            } else {
                ctx.style.text_primary
            },
            max_width: Some(
                sidebar_tag_input_text_rect(input, ctx.sizing)
                    .width()
                    .max(24.0),
            ),
            align: TextAlign::Left,
        },
    );

    for (pill, pill_rect) in sidebar_tag_pill_rects(rect, ctx.sizing, ctx.model) {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: pill_rect,
                color: tag_pill_fill(ctx, pill.state),
            }),
        );
        push_border(
            primitives,
            pill_rect,
            tag_pill_border(ctx, pill.state),
            ctx.sizing.border_width,
        );
        let text_rect = inset_rect(pill_rect, ctx.sizing.text_inset_x, ctx.sizing.text_inset_y);
        emit_text(
            text_runs,
            TextRun {
                text: truncate_to_width(
                    &pill.label,
                    text_rect.width().max(18.0),
                    ctx.sizing.font_meta,
                ),
                position: text_rect.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_primary,
                max_width: Some(text_rect.width().max(18.0)),
                align: TextAlign::Left,
            },
        );
        let close_rect = Rect::from_min_max(
            Point::new(
                (pill_rect.max.x - ctx.sizing.font_meta - 4.0).max(pill_rect.min.x),
                pill_rect.min.y,
            ),
            pill_rect.max,
        );
        emit_text(
            text_runs,
            TextRun {
                text: String::from("x"),
                position: inset_rect(close_rect, 2.0, ctx.sizing.text_inset_y).min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(close_rect.width().max(8.0)),
                align: TextAlign::Center,
            },
        );
    }
}

/// Render the expanded tag-library panel beside the left sidebar.
pub(super) fn render_tag_library_panel(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let Some(panel_rect) = tag_library_panel_rect(ctx.layout, ctx.sizing, ctx.model) else {
        return;
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: panel_rect,
            color: blend_color(ctx.style.bg_secondary, ctx.style.surface_base, 0.36),
        }),
    );
    push_border(
        primitives,
        panel_rect,
        ctx.style.border_emphasis,
        ctx.sizing.border_width,
    );

    let header = tag_library_header_rect(panel_rect, ctx.sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: header,
            color: ctx.style.surface_overlay,
        }),
    );
    push_border_sides(
        primitives,
        header,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides {
            top: false,
            bottom: true,
            left: false,
            right: false,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from("Tag Editor"),
            position: inset_rect(header, ctx.sizing.text_inset_x, ctx.sizing.text_inset_y).min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_primary,
            max_width: Some((header.width() - 28.0).max(24.0)),
            align: TextAlign::Left,
        },
    );
    render_close_button(
        ctx,
        primitives,
        text_runs,
        tag_library_close_rect(header, ctx.sizing),
    );

    let playback_title = tag_library_group_title_rect(panel_rect, ctx.sizing, 0);
    render_group_title(ctx, text_runs, playback_title, "Playback");
    for (index, pill) in ctx
        .model
        .browser
        .pill_editor()
        .exclusive_pills
        .iter()
        .enumerate()
    {
        let row = tag_library_playback_row_rect(panel_rect, ctx.sizing, index);
        render_tag_library_row(ctx, primitives, text_runs, row, &pill.label, pill.state);
    }

    let tags_title = tag_library_tags_title_rect(panel_rect, ctx.sizing);
    render_group_title(ctx, text_runs, tags_title, "Used Tags");
    for (pill, row) in tag_library_option_row_rects(panel_rect, ctx.sizing, ctx.model) {
        render_tag_library_row(ctx, primitives, text_runs, row, &pill.label, pill.state);
    }
    if let Some(create) = ctx.model.browser.pill_editor().create_pill.as_ref() {
        let row = tag_library_create_row_rect(panel_rect, ctx.sizing, ctx.model);
        render_tag_library_row(ctx, primitives, text_runs, row, &create.label, create.state);
    }
}

/// Return the sidebar tag input hit/render rectangle.
pub(in crate::app_core::native_shell::composition::state) fn sidebar_tag_input_rect(
    rect: Rect,
    sizing: SizingTokens,
) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let height = sizing.browser_row_height.max(18.0);
    Rect::from_min_max(
        Point::new(
            rect.min.x + pad,
            (rect.max.y - pad - height).max(rect.min.y + pad),
        ),
        Point::new(rect.max.x - pad, rect.max.y - pad),
    )
}

/// Return the compact tag-section expand button rectangle.
pub(in crate::app_core::native_shell::composition::state) fn sidebar_tag_expand_button_rect(
    rect: Rect,
    sizing: SizingTokens,
) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let side = (sizing.font_meta + 6.0).max(14.0);
    Rect::from_min_max(
        Point::new(rect.max.x - pad - side, rect.min.y + sizing.text_inset_y),
        Point::new(rect.max.x - pad, rect.min.y + sizing.text_inset_y + side),
    )
}

/// Return the expanded tag-library panel rectangle.
pub(in crate::app_core::native_shell::composition::state) fn tag_library_panel_rect(
    layout: &ShellLayout,
    sizing: SizingTokens,
    model: &AppModel,
) -> Option<Rect> {
    if !model.browser_actions.pill_editor_open() {
        return None;
    }
    let gap = sizing.panel_gap.max(3.0);
    let width = layout
        .content
        .width()
        .mul_add(0.28, 0.0)
        .clamp(190.0, 270.0)
        .min((layout.content.width() - gap).max(0.0));
    if width <= 1.0 {
        return None;
    }
    Some(Rect::from_min_max(
        Point::new(layout.sidebar.max.x + gap, layout.sidebar.min.y),
        Point::new(layout.sidebar.max.x + gap + width, layout.sidebar.max.y),
    ))
}

/// Return the inset text box inside the sidebar tag input.
pub(in crate::app_core::native_shell::composition::state) fn sidebar_tag_input_text_rect(
    input: Rect,
    sizing: SizingTokens,
) -> Rect {
    inset_rect(input, sizing.text_inset_x, sizing.text_inset_y)
}

/// Return visible sidebar tag pill rectangles paired with their pill models.
pub(in crate::app_core::native_shell::composition::state) fn sidebar_tag_pill_rects(
    rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
) -> Vec<(&BrowserPillModel, Rect)> {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 3.0;
    let title_height = sizing.font_meta + sizing.text_inset_y + 2.0;
    let input = sidebar_tag_input_rect(rect, sizing);
    let row_height = sizing.browser_row_height.max(18.0);
    let col_width = ((rect.width() - pad * 2.0 - gap) * 0.5).max(36.0);
    let mut out = Vec::new();
    let mut pills: Vec<_> = model.browser.pill_editor().accepted_pills.iter().collect();
    if pills.is_empty() {
        pills.extend(
            model
                .browser
                .pill_editor()
                .option_pills
                .iter()
                .filter(|pill| !matches!(pill.state, BrowserPillState::Off))
                .take(4),
        );
    }
    if pills.is_empty() {
        pills.extend(model.browser.pill_editor().option_pills.iter().take(4));
    }
    if let Some(create) = model.browser.pill_editor().create_pill.as_ref() {
        pills.push(create);
    }
    for (index, pill) in pills.into_iter().take(12).enumerate() {
        let col = index % 2;
        let row = index / 2;
        let min_x = rect.min.x + pad + (col_width + gap) * col as f32;
        let min_y = rect.min.y + pad + title_height + (row_height + gap) * row as f32;
        let pill_rect = Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new(
                (min_x + col_width).min(rect.max.x - pad),
                min_y + row_height,
            ),
        );
        if pill_rect.max.y <= input.min.y - gap {
            out.push((pill, pill_rect));
        }
    }
    out
}

/// Return expanded tag-library normal-tag row rectangles paired with pill models.
pub(in crate::app_core::native_shell::composition::state) fn tag_library_option_row_rects(
    panel_rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
) -> Vec<(&BrowserPillModel, Rect)> {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 1.0;
    let row_height = sizing.browser_row_height.max(18.0);
    let mut y = tag_library_tags_title_rect(panel_rect, sizing).max.y + gap;
    let bottom = tag_library_input_top(panel_rect, sizing) - gap;
    let mut rows = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for pill in model
        .browser
        .pill_editor()
        .accepted_pills
        .iter()
        .chain(model.browser.pill_editor().option_pills.iter())
    {
        if !seen.insert(pill.id.clone()) {
            continue;
        }
        let row = Rect::from_min_max(
            Point::new(panel_rect.min.x + pad, y),
            Point::new(panel_rect.max.x - pad, (y + row_height).min(bottom)),
        );
        if row.height() < row_height * 0.65 || row.max.y > bottom {
            break;
        }
        rows.push((pill, row));
        y += row_height + gap;
    }
    rows
}

fn render_input_selection_and_caret(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    input: Rect,
) {
    let editor = ctx.model.browser.pill_editor();
    if !editor.input_focused {
        return;
    }
    let text_rect = sidebar_tag_input_text_rect(input, ctx.sizing);
    let char_width = (ctx.sizing.font_meta * 0.56).max(1.0);
    if let Some((start, end)) = normalized_selection(editor.input_selection) {
        let min_x =
            text_rect.min.x.min(text_rect.max.x).max(text_rect.min.x) + char_width * start as f32;
        let max_x = text_rect.min.x + char_width * end as f32;
        let selection = Rect::from_min_max(
            Point::new(min_x.min(text_rect.max.x), text_rect.min.y),
            Point::new(max_x.min(text_rect.max.x), text_rect.max.y),
        );
        if selection.width() > 0.5 {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: selection,
                    color: blend_color(ctx.style.accent_mint, ctx.style.surface_overlay, 0.45),
                }),
            );
        }
    }
    let caret_index = editor.input_caret.min(editor.input_value.chars().count());
    let caret_x = (text_rect.min.x + char_width * caret_index as f32)
        .min(text_rect.max.x)
        .max(text_rect.min.x);
    let caret = Rect::from_min_max(
        Point::new(caret_x, text_rect.min.y),
        Point::new(
            (caret_x + ctx.sizing.border_width.max(1.0)).min(text_rect.max.x),
            text_rect.max.y,
        ),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: caret,
            color: ctx.style.text_primary,
        }),
    );
}

fn normalized_selection(selection: Option<(usize, usize)>) -> Option<(usize, usize)> {
    let (a, b) = selection?;
    (a != b).then_some(if a < b { (a, b) } else { (b, a) })
}

fn render_tag_library_toggle(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    rect: Rect,
) {
    let button = sidebar_tag_expand_button_rect(rect, ctx.sizing);
    let active = ctx.model.browser_actions.pill_editor_open();
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: button,
            color: if active {
                blend_color(ctx.style.highlight_cyan, ctx.style.surface_overlay, 0.45)
            } else {
                ctx.style.surface_overlay
            },
        }),
    );
    push_border(
        primitives,
        button,
        if active {
            ctx.style.highlight_cyan
        } else {
            ctx.style.border_emphasis
        },
        ctx.sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: if active {
                String::from("<")
            } else {
                String::from(">")
            },
            position: inset_rect(button, 2.0, 1.0).min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_primary,
            max_width: Some(button.width().max(8.0)),
            align: TextAlign::Center,
        },
    );
}

fn render_close_button(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    rect: Rect,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: ctx.style.surface_overlay,
        }),
    );
    push_border(primitives, rect, ctx.style.border, ctx.sizing.border_width);
    emit_text(
        text_runs,
        TextRun {
            text: String::from("x"),
            position: inset_rect(rect, 2.0, 1.0).min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(rect.width().max(8.0)),
            align: TextAlign::Center,
        },
    );
}

fn render_group_title(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    rect: Rect,
    title: &str,
) {
    emit_text(
        text_runs,
        TextRun {
            text: title.to_string(),
            position: rect.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(rect.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
}

fn render_tag_library_row(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    row: Rect,
    label: &str,
    state: BrowserPillState,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: row,
            color: ctx.style.surface_overlay,
        }),
    );
    push_border(primitives, row, ctx.style.border, ctx.sizing.border_width);
    let box_side = (row.height() - 6.0).clamp(8.0, 13.0);
    let checkbox = Rect::from_min_max(
        Point::new(row.min.x + 5.0, row.min.y + (row.height() - box_side) * 0.5),
        Point::new(
            row.min.x + 5.0 + box_side,
            row.min.y + (row.height() + box_side) * 0.5,
        ),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: checkbox,
            color: tag_pill_fill(ctx, state),
        }),
    );
    push_border(
        primitives,
        checkbox,
        tag_pill_border(ctx, state),
        ctx.sizing.border_width,
    );
    if matches!(state, BrowserPillState::Mixed) {
        let mark = Rect::from_min_max(
            Point::new(checkbox.min.x + 2.0, checkbox.center().y - 1.0),
            Point::new(checkbox.max.x - 2.0, checkbox.center().y + 1.0),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: mark,
                color: ctx.style.highlight_orange,
            }),
        );
    }
    let label_rect = Rect::from_min_max(
        Point::new(checkbox.max.x + 7.0, row.min.y + ctx.sizing.text_inset_y),
        Point::new(row.max.x - 5.0, row.max.y - ctx.sizing.text_inset_y),
    );
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(label, label_rect.width().max(24.0), ctx.sizing.font_meta),
            position: label_rect.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_primary,
            max_width: Some(label_rect.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
}

fn tag_library_header_rect(panel_rect: Rect, sizing: SizingTokens) -> Rect {
    Rect::from_min_max(
        panel_rect.min,
        Point::new(
            panel_rect.max.x,
            panel_rect.min.y + (sizing.browser_row_height.max(18.0) + 5.0),
        ),
    )
}

fn tag_library_close_rect(header: Rect, sizing: SizingTokens) -> Rect {
    let side = (sizing.font_meta + 6.0).max(14.0);
    Rect::from_min_max(
        Point::new(header.max.x - side - 4.0, header.min.y + 4.0),
        Point::new(header.max.x - 4.0, header.min.y + 4.0 + side),
    )
}

fn tag_library_group_title_rect(panel_rect: Rect, sizing: SizingTokens, index: usize) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let y = tag_library_header_rect(panel_rect, sizing).max.y
        + pad
        + index as f32 * (sizing.font_meta + sizing.browser_row_height.max(18.0) * 2.0);
    Rect::from_min_max(
        Point::new(panel_rect.min.x + pad, y),
        Point::new(panel_rect.max.x - pad, y + sizing.font_meta + 2.0),
    )
}

fn tag_library_playback_row_rect(panel_rect: Rect, sizing: SizingTokens, index: usize) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 1.0;
    let row_height = sizing.browser_row_height.max(18.0);
    let y = tag_library_group_title_rect(panel_rect, sizing, 0).max.y
        + gap
        + index as f32 * (row_height + gap);
    Rect::from_min_max(
        Point::new(panel_rect.min.x + pad, y),
        Point::new(panel_rect.max.x - pad, y + row_height),
    )
}

fn tag_library_tags_title_rect(panel_rect: Rect, sizing: SizingTokens) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 4.0;
    let y = tag_library_playback_row_rect(panel_rect, sizing, 1).max.y + gap;
    Rect::from_min_max(
        Point::new(panel_rect.min.x + pad, y),
        Point::new(panel_rect.max.x - pad, y + sizing.font_meta + 2.0),
    )
}

fn tag_library_input_top(panel_rect: Rect, sizing: SizingTokens) -> f32 {
    sidebar_tag_input_rect(panel_rect, sizing).min.y
}

fn tag_library_create_row_rect(panel_rect: Rect, sizing: SizingTokens, model: &AppModel) -> Rect {
    tag_library_option_row_rects(panel_rect, sizing, model)
        .last()
        .map(|(_, row)| {
            let gap = sizing.border_width.max(1.0) + 1.0;
            Rect::from_min_max(
                Point::new(row.min.x, row.max.y + gap),
                Point::new(row.max.x, row.max.y + gap + row.height()),
            )
        })
        .unwrap_or_else(|| {
            let gap = sizing.border_width.max(1.0) + 1.0;
            let title = tag_library_tags_title_rect(panel_rect, sizing);
            Rect::from_min_max(
                Point::new(title.min.x, title.max.y + gap),
                Point::new(
                    title.max.x,
                    title.max.y + gap + sizing.browser_row_height.max(18.0),
                ),
            )
        })
}

/// Render the shared panel background for sidebar subsections.
fn render_section_panel(ctx: &StaticFrameCtx<'_>, primitives: &mut impl PrimitiveSink, rect: Rect) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: blend_color(ctx.style.bg_secondary, ctx.style.surface_base, 0.42),
        }),
    );
    push_border(primitives, rect, ctx.style.border, ctx.sizing.border_width);
}

/// Render an uppercase sidebar subsection title.
fn render_section_title(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    rect: Rect,
    title: &str,
) {
    let title_rect = inset_rect(
        rect,
        ctx.sizing.panel_inset.max(5.0),
        ctx.sizing.text_inset_y,
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from(title),
            position: title_rect.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(title_rect.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
}

/// Return the fill color for a tag pill state.
fn tag_pill_fill(ctx: &StaticFrameCtx<'_>, state: BrowserPillState) -> Rgba8 {
    match state {
        BrowserPillState::On => blend_color(ctx.style.accent_mint, ctx.style.surface_overlay, 0.62),
        BrowserPillState::Mixed => {
            blend_color(ctx.style.highlight_orange, ctx.style.surface_overlay, 0.45)
        }
        BrowserPillState::Off => ctx.style.surface_overlay,
    }
}

/// Return the border color for a tag pill state.
fn tag_pill_border(ctx: &StaticFrameCtx<'_>, state: BrowserPillState) -> Rgba8 {
    match state {
        BrowserPillState::On => ctx.style.accent_mint,
        BrowserPillState::Mixed => ctx.style.highlight_orange,
        BrowserPillState::Off => ctx.style.border_emphasis,
    }
}

/// Inset a rectangle without inverting its bounds.
fn inset_rect(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::from_min_max(
        Point::new(
            (rect.min.x + x).min(rect.max.x),
            (rect.min.y + y).min(rect.max.y),
        ),
        Point::new(
            (rect.max.x - x).max(rect.min.x),
            (rect.max.y - y).max(rect.min.y),
        ),
    )
}
