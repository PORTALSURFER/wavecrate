use super::*;

pub(super) fn render_browser_frame(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let search_editor_active = state.browser_search_editor_visual.is_some();
    let toolbar = {
        let (buttons, column_chips, toolbar) =
            state.cached_browser_action_hit_test(ctx.layout, ctx.style, ctx.model);
        for button in buttons {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: button.rect,
                    color: if !button.enabled {
                        ctx.style.control_disabled_fill
                    } else if button.active {
                        blend_color(ctx.style.highlight_cyan, ctx.style.surface_overlay, 0.26)
                    } else {
                        ctx.style.surface_overlay
                    },
                }),
            );
            push_border(
                primitives,
                button.rect,
                if !button.enabled {
                    ctx.style.border
                } else if button.active {
                    blend_color(ctx.style.highlight_cyan, ctx.style.text_primary, 0.32)
                } else {
                    blend_color(
                        ctx.style.border_emphasis,
                        ctx.style.text_primary,
                        ctx.style.state_hover_soft,
                    )
                },
                ctx.sizing.border_width,
            );
            let button_color = if button.enabled {
                button.text_color
            } else {
                ctx.style.text_muted
            };
            if let Some(icon) = button.icon {
                let icon_rect = centered_button_icon_rect(button.rect, ctx.sizing);
                let _ = emit_toolbar_svg_icon(primitives, icon, icon_rect, button_color);
            } else {
                let label_rect = compute_action_button_text_rect(button.rect, ctx.sizing);
                emit_text(
                    text_runs,
                    TextRun {
                        text: button.label.to_string(),
                        position: label_rect.min,
                        font_size: ctx.sizing.font_meta,
                        color: button_color,
                        max_width: Some(label_rect.width().max(12.0)),
                        align: TextAlign::Center,
                    },
                );
            }
        }
        for chip in column_chips {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: chip.rect,
                    color: if chip.selected {
                        match chip.column {
                            0 => {
                                blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.50)
                            }
                            2 => blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.50),
                            _ => blend_color(ctx.style.text_primary, ctx.style.bg_secondary, 0.42),
                        }
                    } else {
                        match chip.column {
                            0 => {
                                blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.34)
                            }
                            2 => blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.34),
                            _ => blend_color(ctx.style.text_muted, ctx.style.bg_secondary, 0.28),
                        }
                    },
                }),
            );
            push_border(
                primitives,
                chip.rect,
                if chip.selected {
                    blend_color(ctx.style.border_emphasis, ctx.style.text_primary, 0.55)
                } else {
                    ctx.style.border
                },
                ctx.sizing.border_width,
            );
            let label_rect = compute_action_button_text_rect(chip.rect, ctx.sizing);
            emit_text(
                text_runs,
                TextRun {
                    text: format!("{} ({})", chip.label, chip.item_count),
                    position: label_rect.min,
                    font_size: ctx.sizing.font_meta,
                    color: if chip.selected {
                        ctx.style.text_primary
                    } else {
                        ctx.style.text_muted
                    },
                    max_width: Some(label_rect.width().max(16.0)),
                    align: TextAlign::Center,
                },
            );
        }
        toolbar
    };

    for (index, rect) in toolbar.rating_filter_chips.iter().copied().enumerate() {
        if rect.width() <= 1.0 {
            continue;
        }
        let level = BROWSER_RATING_FILTER_LEVELS[index];
        let active = ctx.model.browser.active_rating_filters[index];
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: browser_rating_filter_chip_fill(ctx.style, level, active),
            }),
        );
        push_border(
            primitives,
            rect,
            browser_rating_filter_chip_border(ctx.style, level, active),
            ctx.sizing.border_width,
        );
    }
    for (index, rect) in toolbar
        .playback_age_filter_chips
        .iter()
        .copied()
        .enumerate()
    {
        if rect.width() <= 1.0 {
            continue;
        }
        let chip = BROWSER_PLAYBACK_AGE_FILTER_CHIPS[index];
        let active = ctx.model.browser.active_playback_age_filters[index];
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: browser_playback_age_filter_chip_fill(ctx.style, chip, active),
            }),
        );
        push_border(
            primitives,
            rect,
            browser_playback_age_filter_chip_border(ctx.style, chip, active),
            ctx.sizing.border_width,
        );
        let _ = emit_toolbar_svg_icon(
            primitives,
            browser_playback_age_filter_icon(chip),
            centered_button_icon_rect(rect, ctx.sizing),
            if active {
                ctx.style.text_primary
            } else {
                ctx.style.text_muted
            },
        );
    }
    if toolbar.marked_filter_chip.width() > 1.0 {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: toolbar.marked_filter_chip,
                color: browser_marked_filter_chip_fill(
                    ctx.style,
                    ctx.model.browser.marked_filter_active,
                ),
            }),
        );
        push_border(
            primitives,
            toolbar.marked_filter_chip,
            browser_marked_filter_chip_border(ctx.style, ctx.model.browser.marked_filter_active),
            ctx.sizing.border_width,
        );
        let _ = emit_toolbar_svg_icon(
            primitives,
            WaveformToolbarIcon::BrowserMarked,
            centered_button_icon_rect(toolbar.marked_filter_chip, ctx.sizing),
            if ctx.model.browser.marked_filter_active {
                ctx.style.text_primary
            } else {
                ctx.style.highlight_cyan
            },
        );
    }
    if toolbar.search_field.width() > 1.0 {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: toolbar.search_field,
                color: ctx.style.surface_base,
            }),
        );
        push_border(
            primitives,
            toolbar.search_field,
            blend_color(ctx.style.border_emphasis, ctx.style.text_primary, 0.35),
            ctx.sizing.border_width,
        );
    }
    if toolbar.activity_chip.width() > 1.0 {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: toolbar.activity_chip,
                color: if ctx.model.browser.busy {
                    blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.45)
                } else {
                    blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.40)
                },
            }),
        );
        push_border(
            primitives,
            toolbar.activity_chip,
            ctx.style.border,
            ctx.sizing.border_width,
        );
    }
    if toolbar.sort_chip.width() > 1.0 {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: toolbar.sort_chip,
                color: ctx.style.surface_overlay,
            }),
        );
        push_border(
            primitives,
            toolbar.sort_chip,
            ctx.style.border,
            ctx.sizing.border_width,
        );
    }
    let cached_text = state.cached_browser_segment_text(ctx.layout, ctx.style, ctx.model);
    render_browser_tabs(primitives, text_runs, ctx, true, cached_text.as_ref());

    if toolbar.search_field.width() > 1.0 && !search_editor_active {
        emit_text(
            text_runs,
            TextRun {
                text: cached_text.search_label.clone(),
                position: cached_text.toolbar_text_layout.search_label.min,
                font_size: ctx.sizing.font_meta,
                color: if ctx.model.browser.search_query.is_empty() {
                    ctx.style.text_muted
                } else {
                    ctx.style.text_primary
                },
                max_width: Some(
                    cached_text
                        .toolbar_text_layout
                        .search_label
                        .width()
                        .max(24.0),
                ),
                align: TextAlign::Left,
            },
        );
    }
    if toolbar.activity_chip.width() > 1.0 {
        emit_text(
            text_runs,
            TextRun {
                text: cached_text.activity_label.clone(),
                position: cached_text.toolbar_text_layout.activity_label.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_primary,
                max_width: Some(
                    cached_text
                        .toolbar_text_layout
                        .activity_label
                        .width()
                        .max(20.0),
                ),
                align: TextAlign::Center,
            },
        );
    }
    if toolbar.sort_chip.width() > 1.0 {
        emit_text(
            text_runs,
            TextRun {
                text: cached_text.sort_label.clone(),
                position: cached_text.toolbar_text_layout.sort_label.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(cached_text.toolbar_text_layout.sort_label.width().max(20.0)),
                align: TextAlign::Center,
            },
        );
    }
}

fn centered_button_icon_rect(button_rect: Rect, sizing: SizingTokens) -> Rect {
    let side = button_rect
        .width()
        .min(button_rect.height())
        .min((button_rect.height() - (sizing.text_inset_y * 0.8)).max(8.0))
        .clamp(8.0, 20.0);
    let min_x = button_rect.min.x + ((button_rect.width() - side) * 0.5);
    let min_y = button_rect.min.y + ((button_rect.height() - side) * 0.5);
    Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(min_x + side, min_y + side),
    )
}

fn browser_playback_age_filter_icon(
    chip: crate::app::PlaybackAgeFilterChip,
) -> WaveformToolbarIcon {
    match chip {
        crate::app::PlaybackAgeFilterChip::NeverPlayed => WaveformToolbarIcon::BrowserNeverPlayed,
        crate::app::PlaybackAgeFilterChip::OlderThanMonth => {
            WaveformToolbarIcon::BrowserOlderThanMonth
        }
        crate::app::PlaybackAgeFilterChip::OlderThanWeek => {
            WaveformToolbarIcon::BrowserOlderThanWeek
        }
    }
}
