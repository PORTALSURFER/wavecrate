use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;

pub(super) fn render_browser_rows_window(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    browser_rows: &[CachedBrowserRow],
) {
    let last_row_max_y = browser_rows.last().map(|row| row.rect.max.y);
    for row in browser_rows {
        let row_border_stroke = browser_row_border_stroke(ctx.layout);
        let row_border_rect = browser_row_border_rect(row.rect, row_border_stroke);
        let row_columns = row.text_layout.columns;
        let similarity_active =
            ctx.model.browser.similarity_filtered || ctx.model.browser.duplicate_cleanup_active;
        let similarity_button = (!ctx.model.browser.duplicate_cleanup_active)
            .then_some(row)
            .filter(|row| row.focused)
            .and_then(|row| browser_similarity_button_rect(row.rect, ctx.sizing));
        let similarity_button_reserved_width =
            browser_similarity_button_reserved_width(similarity_button.is_some(), ctx.sizing);
        let similarity_strength_reserved_width = browser_similarity_strength_reserved_width(
            row.similarity_display_strength.is_some(),
            ctx.sizing,
        );
        let base_fill = if row.marked && similarity_active {
            browser_marked_similarity_row_fill(ctx.style, row.visible_row, row.visible_row == 0)
        } else if row.marked {
            browser_marked_row_fill(ctx.style, row.visible_row)
        } else if similarity_active {
            browser_similarity_row_fill(ctx.style, row.visible_row, row.visible_row == 0)
        } else {
            browser_row_stripe_fill(ctx.style, row.visible_row)
        };
        let base_fill = browser_processing_row_fill(ctx.style, base_fill, row.processing_state);
        let age_marker_reserved_width = browser_playback_age_marker_reserved_width(
            row.rect,
            ctx.sizing,
            similarity_button_reserved_width,
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: row.rect,
                color: base_fill,
            }),
        );
        render_browser_processing_marker(primitives, row, ctx.style, ctx.sizing);
        if similarity_active && row.visible_row == 0 {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: row.text_layout.columns.index,
                    color: similarity_anchor_browser_index_fill(ctx.style),
                }),
            );
        }
        if let Some(marker_rect) =
            browser_playback_age_marker_rect(row.rect, ctx.sizing, similarity_button_reserved_width)
        {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: marker_rect,
                    color: browser_playback_age_marker_color(ctx.style, row.playback_age_bucket),
                }),
            );
        }
        let marked_marker_width = if row.marked { 4.0 } else { 0.0 };
        if row.marked {
            if let Some(marker_rect) = browser_locked_marker_rect(row.rect, ctx.sizing, 0.0) {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: marker_rect,
                        color: ctx.style.highlight_cyan,
                    }),
                );
            }
        }
        if row.locked {
            if let Some(marker_rect) =
                browser_locked_marker_rect(row.rect, ctx.sizing, marked_marker_width)
            {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: marker_rect,
                        color: ctx.style.accent_mint,
                    }),
                );
            }
        }
        for separator_x in [row_columns.index.max.x, row_columns.sample.max.x] {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: Rect::from_min_max(
                        Point::new(separator_x, row.rect.min.y),
                        Point::new(
                            (separator_x + ctx.sizing.border_width).min(row.rect.max.x),
                            row.rect.max.y,
                        ),
                    ),
                    color: blend_color(ctx.style.border, ctx.style.grid_soft, 0.36),
                }),
            );
        }
        push_browser_row_border(
            primitives,
            row_border_rect,
            ctx.style.border,
            row_border_stroke,
            BorderSides {
                top: true,
                bottom: Some(row.rect.max.y) == last_row_max_y,
                left: false,
                right: false,
            },
        );
        let chip_rect = row.text_layout.bucket_chip;
        let chip_color = match row.column {
            0 => blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.54),
            2 => blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.54),
            _ => blend_color(ctx.style.text_muted, ctx.style.bg_secondary, 0.54),
        };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: chip_rect,
                color: chip_color,
            }),
        );
        push_border(
            primitives,
            chip_rect,
            ctx.style.border,
            ctx.sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: row.visible_row_label.clone(),
                position: row.text_layout.index_label.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(row.text_layout.index_label.width().max(12.0)),
                align: TextAlign::Right,
            },
        );
        let mut label_position = row.text_layout.sample_label.min;
        let mut label_max_width = row.text_layout.sample_label.width().max(20.0);
        if similarity_button_reserved_width > 0.0 {
            label_position.x = (label_position.x + similarity_button_reserved_width)
                .min(row.text_layout.sample_label.max.x);
            label_max_width = (row.text_layout.sample_label.max.x - label_position.x).max(4.0);
        }
        if age_marker_reserved_width > 0.0 {
            label_position.x = (label_position.x + age_marker_reserved_width)
                .min(row.text_layout.sample_label.max.x);
            label_max_width = (row.text_layout.sample_label.max.x - label_position.x).max(4.0);
        }
        if row.missing {
            let marker_advance =
                browser_missing_marker_advance(ctx.sizing.font_body).min(label_max_width.max(0.0));
            emit_text(
                text_runs,
                TextRun {
                    text: String::from(BROWSER_MISSING_CONTENT_MARKER),
                    position: label_position,
                    font_size: ctx.sizing.font_body,
                    color: ctx.style.accent_trash,
                    max_width: Some(marker_advance),
                    align: TextAlign::Left,
                },
            );
            label_position.x =
                (label_position.x + marker_advance).min(row.text_layout.sample_label.max.x);
            label_max_width = (row.text_layout.sample_label.max.x - label_position.x).max(4.0);
        }
        let inline_tag_reserved_width =
            browser_inline_tag_reserved_width_for_labels(&row.inline_tag_labels, ctx.sizing);
        let rating_reserved_width =
            browser_rating_indicator_reserved_width(row.rating_level, row.locked, ctx.sizing);
        let rating_indicator_layout = browser_rating_indicator_layout(
            BrowserRatingIndicatorAnchor {
                sample_label: row.text_layout.sample_label,
                label_origin_x: label_position.x,
                label_rendered_width: row.label_rendered_width.min(label_max_width.max(0.0)),
                right_limit_x: row.text_layout.sample_label.max.x
                    - inline_tag_reserved_width
                    - similarity_strength_reserved_width,
            },
            row.rating_level,
            row.locked,
            ctx.sizing,
        );
        if let Some(indicators) = rating_indicator_layout {
            label_max_width = (label_max_width
                - rating_reserved_width
                - inline_tag_reserved_width
                - similarity_strength_reserved_width)
                .max(4.0);
            for rect in indicators.rects.into_iter().take(indicators.count) {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect,
                        color: browser_rating_indicator_color(ctx.style, row.rating_level),
                    }),
                );
                push_border(
                    primitives,
                    rect,
                    blend_color(ctx.style.border, ctx.style.text_primary, 0.28),
                    ctx.sizing.border_width,
                );
            }
        } else {
            label_max_width =
                (label_max_width - inline_tag_reserved_width - similarity_strength_reserved_width)
                    .max(4.0);
        }
        emit_text(
            text_runs,
            TextRun {
                text: row.label.clone(),
                position: label_position,
                font_size: ctx.sizing.font_body,
                color: ctx.style.text_primary,
                max_width: Some(label_max_width),
                align: TextAlign::Left,
            },
        );
        if !row.bucket_label.is_empty() {
            for (chip_rect, chip_label) in row
                .inline_tag_rects
                .iter()
                .copied()
                .zip(row.inline_tag_labels.iter())
            {
                let text_origin = browser_inline_tag_text_origin(chip_rect, ctx.sizing);
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: chip_rect,
                        color: blend_color(ctx.style.surface_overlay, ctx.style.bg_tertiary, 0.54),
                    }),
                );
                push_border(
                    primitives,
                    chip_rect,
                    blend_color(ctx.style.border_emphasis, ctx.style.text_muted, 0.18),
                    ctx.sizing.border_width,
                );
                emit_text(
                    text_runs,
                    TextRun {
                        text: chip_label.clone(),
                        position: text_origin,
                        font_size: ctx.sizing.font_meta,
                        color: ctx.style.text_primary,
                        max_width: Some((chip_rect.max.x - text_origin.x).max(4.0)),
                        align: TextAlign::Left,
                    },
                );
            }
        }
        if let Some(strength) = row.similarity_display_strength {
            if let Some(track_rect) =
                browser_similarity_strength_track_rect(row.text_layout.sample_label, ctx.sizing)
            {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: track_rect,
                        color: translucent_overlay_color(
                            ctx.style.surface_overlay,
                            ctx.style.text_muted,
                            0.12,
                        ),
                    }),
                );
                if let Some(fill_rect) = browser_similarity_strength_fill_rect(track_rect, strength)
                {
                    emit_primitive(
                        primitives,
                        Primitive::Rect(FillRect {
                            rect: fill_rect,
                            color: blend_color(
                                ctx.style.highlight_cyan_soft,
                                ctx.style.highlight_cyan,
                                0.38,
                            ),
                        }),
                    );
                }
            }
        }
        if let Some(button_rect) = similarity_button {
            let button_active = similarity_active && row.visible_row == 0;
            render_browser_similarity_button(
                primitives,
                button_rect,
                ctx.style,
                ctx.sizing,
                button_active,
                if button_active {
                    ctx.style.text_primary
                } else {
                    ctx.style.text_muted
                },
            );
        }
    }
    let list_rect = browser_rows_list_rect(ctx.layout.browser_rows, ctx.sizing, ctx.model);
    if let Some(scrollbar) = browser_scrollbar_layout(
        list_rect,
        browser_rows,
        ctx.model.browser.visible_count,
        ctx.sizing,
    ) {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: scrollbar.track,
                color: blend_color(ctx.style.border, ctx.style.bg_secondary, 0.22),
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: scrollbar.thumb,
                color: blend_color(ctx.style.text_muted, ctx.style.text_primary, 0.32),
            }),
        );
    }
    render_browser_pill_editor_overlay(ctx, primitives, text_runs);
}

fn render_browser_processing_marker(
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
    sizing: SizingTokens,
) {
    let Some(color) = browser_processing_marker_color(style, row.processing_state) else {
        return;
    };
    let marker_width = (sizing.border_width * 3.0).clamp(2.0, 5.0);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(
                row.rect.min,
                Point::new(
                    (row.rect.min.x + marker_width).min(row.rect.max.x),
                    row.rect.max.y,
                ),
            ),
            color,
        }),
    );
}

fn render_browser_pill_editor_overlay(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let Some(layout) = browser_pill_editor_layout(ctx.layout.browser_rows, ctx.sizing, ctx.model)
    else {
        return;
    };
    let sidebar = ctx.model.browser.pill_editor();
    let panel_rect = browser_pill_editor_rect(ctx.layout.browser_rows, ctx.sizing, ctx.model)
        .unwrap_or(ctx.layout.browser_rows);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: panel_rect,
            color: ctx.style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        panel_rect,
        ctx.style.border_emphasis,
        ctx.sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: sidebar.header_label.clone(),
            position: Point::new(
                layout.input_rect.min.x,
                panel_rect.min.y + ctx.sizing.panel_inset.max(8.0),
            ),
            font_size: ctx.sizing.font_body,
            color: ctx.style.text_primary,
            max_width: Some((panel_rect.width() - ctx.sizing.panel_inset * 2.0).max(24.0)),
            align: TextAlign::Left,
        },
    );
    render_sidebar_toggle_button(
        primitives,
        text_runs,
        ctx,
        layout.auto_rename_rect,
        "Auto-rename",
        sidebar.primary_action_enabled,
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: layout.input_rect,
            color: ctx.style.surface_base,
        }),
    );
    push_border(
        primitives,
        layout.input_rect,
        ctx.style.border,
        ctx.sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: if sidebar.input_value.is_empty() {
                sidebar.input_placeholder.clone()
            } else {
                sidebar.input_value.clone()
            },
            position: layout.input_text_rect.min,
            font_size: ctx.sizing.font_meta,
            color: if sidebar.input_value.is_empty() {
                ctx.style.text_muted
            } else {
                ctx.style.text_primary
            },
            max_width: Some(layout.input_text_rect.width().max(20.0)),
            align: TextAlign::Left,
        },
    );
    for (pill, rect) in sidebar
        .exclusive_pills
        .iter()
        .zip(layout.playback_rects.iter())
    {
        render_sidebar_tag_pill(primitives, text_runs, ctx, *rect, pill);
    }
    for (pill, rect) in sidebar
        .option_pills
        .iter()
        .zip(layout.normal_tag_rects.iter())
    {
        render_sidebar_tag_pill(primitives, text_runs, ctx, *rect, pill);
    }
    if let (Some(create), Some(rect)) = (sidebar.create_pill.as_ref(), layout.create_tag_rect) {
        render_sidebar_tag_pill(primitives, text_runs, ctx, rect, create);
    }
}

fn render_sidebar_toggle_button(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    ctx: &StaticFrameCtx<'_>,
    rect: Rect,
    label: &str,
    active: bool,
) {
    let (fill, border, text) = if active {
        (
            blend_color(ctx.style.highlight_cyan, ctx.style.surface_overlay, 0.24),
            blend_color(ctx.style.highlight_cyan, ctx.style.text_primary, 0.32),
            ctx.style.text_primary,
        )
    } else {
        (
            ctx.style.surface_base,
            ctx.style.border,
            ctx.style.text_muted,
        )
    };
    emit_primitive(primitives, Primitive::Rect(FillRect { rect, color: fill }));
    push_border(primitives, rect, border, ctx.sizing.border_width);
    emit_text(
        text_runs,
        TextRun {
            text: label.to_string(),
            position: Point::new(
                rect.min.x + ctx.sizing.text_inset_x,
                rect.min.y + ctx.sizing.text_inset_y,
            ),
            font_size: ctx.sizing.font_meta,
            color: text,
            max_width: Some((rect.width() - ctx.sizing.text_inset_x * 2.0).max(10.0)),
            align: TextAlign::Left,
        },
    );
}

fn render_sidebar_tag_pill(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    ctx: &StaticFrameCtx<'_>,
    rect: Rect,
    pill: &native_model::BrowserPillModel,
) {
    let (fill, border, text) = match pill.state {
        native_model::BrowserPillState::Off => (
            ctx.style.surface_base,
            ctx.style.border,
            ctx.style.text_muted,
        ),
        native_model::BrowserPillState::On => (
            blend_color(ctx.style.highlight_cyan, ctx.style.surface_overlay, 0.24),
            blend_color(ctx.style.highlight_cyan, ctx.style.text_primary, 0.32),
            ctx.style.text_primary,
        ),
        native_model::BrowserPillState::Mixed => (
            blend_color(
                ctx.style.highlight_orange_soft,
                ctx.style.surface_overlay,
                0.26,
            ),
            blend_color(ctx.style.highlight_orange, ctx.style.text_primary, 0.26),
            ctx.style.text_primary,
        ),
    };
    emit_primitive(primitives, Primitive::Rect(FillRect { rect, color: fill }));
    push_border(primitives, rect, border, ctx.sizing.border_width);
    emit_text(
        text_runs,
        TextRun {
            text: pill.label.clone(),
            position: Point::new(
                rect.min.x + ctx.sizing.text_inset_x,
                rect.min.y + ctx.sizing.text_inset_y,
            ),
            font_size: ctx.sizing.font_meta,
            color: text,
            max_width: Some((rect.width() - ctx.sizing.text_inset_x * 2.0).max(10.0)),
            align: TextAlign::Left,
        },
    );
}

struct BrowserPillEditorLayout {
    auto_rename_rect: Rect,
    input_rect: Rect,
    input_text_rect: Rect,
    playback_rects: [Rect; 2],
    normal_tag_rects: Vec<Rect>,
    create_tag_rect: Option<Rect>,
}

fn browser_pill_editor_rect(
    rows_rect: Rect,
    _sizing: SizingTokens,
    model: &AppModel,
) -> Option<Rect> {
    browser_pill_editor_panel_rect(rows_rect, _sizing, model)
}

fn browser_pill_editor_layout(
    rows_rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
) -> Option<BrowserPillEditorLayout> {
    let rect = browser_pill_editor_rect(rows_rect, sizing, model)?;
    let pad = sizing.panel_inset.max(8.0);
    let content_min_x = rect.min.x + pad;
    let content_max_x = rect.max.x - pad;
    let field_height = sizing.browser_row_height.max(22.0);
    let auto_rename_top = rect.min.y + pad + sizing.font_body + 10.0;
    let auto_rename_rect = Rect::from_min_max(
        Point::new(content_min_x, auto_rename_top),
        Point::new(content_max_x, auto_rename_top + field_height),
    );
    let input_top = auto_rename_rect.max.y + 8.0;
    let input_rect = Rect::from_min_max(
        Point::new(content_min_x, input_top),
        Point::new(content_max_x, input_top + field_height),
    );
    let input_text_rect = Rect::from_min_max(
        Point::new(
            input_rect.min.x + sizing.text_inset_x,
            input_rect.min.y + sizing.text_inset_y,
        ),
        Point::new(
            input_rect.max.x - sizing.text_inset_x,
            input_rect.max.y - sizing.text_inset_y,
        ),
    );
    let pill_gap = sizing.border_width.max(1.0) + 4.0;
    let two_col_width = ((content_max_x - content_min_x - pill_gap) * 0.5).max(40.0);
    let playback_top = input_rect.max.y + 10.0;
    let playback_rects = [
        Rect::from_min_max(
            Point::new(content_min_x, playback_top),
            Point::new(content_min_x + two_col_width, playback_top + field_height),
        ),
        Rect::from_min_max(
            Point::new(content_min_x + two_col_width + pill_gap, playback_top),
            Point::new(content_max_x, playback_top + field_height),
        ),
    ];
    let tags_top = playback_rects[0].max.y + 12.0;
    let tag_cols = 3usize;
    let tag_width = ((content_max_x - content_min_x - pill_gap * (tag_cols - 1) as f32)
        / tag_cols as f32)
        .max(40.0);
    let mut normal_tag_rects = Vec::with_capacity(model.browser.pill_editor().option_pills.len());
    for index in 0..model.browser.pill_editor().option_pills.len() {
        let col = index % tag_cols;
        let row = index / tag_cols;
        let min_x = content_min_x + (tag_width + pill_gap) * col as f32;
        let min_y = tags_top + (field_height + pill_gap) * row as f32;
        normal_tag_rects.push(Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new((min_x + tag_width).min(content_max_x), min_y + field_height),
        ));
    }
    let create_tag_rect = model.browser.pill_editor().create_pill.as_ref().map(|_| {
        let y = normal_tag_rects
            .last()
            .map(|rect| rect.max.y + 12.0)
            .unwrap_or(tags_top);
        Rect::from_min_max(
            Point::new(content_min_x, y),
            Point::new(content_max_x, y + field_height),
        )
    });
    Some(BrowserPillEditorLayout {
        auto_rename_rect,
        input_rect,
        input_text_rect,
        playback_rects,
        normal_tag_rects,
        create_tag_rect,
    })
}
