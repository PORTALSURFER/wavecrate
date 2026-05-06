use super::*;

/// Ordered filter rows rendered in the left sidebar.
const FILTER_ROWS: [&str; 6] = ["Format", "Bit Depth", "Channels", "BPM", "Key", "Rating"];

/// Render the left-sidebar browser filters panel.
pub(super) fn render_sidebar_filters(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let rect = sidebar_workspace_sections(ctx.layout, ctx.style).filters;
    if rect.width() <= 1.0 || rect.height() <= 1.0 {
        return;
    }
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: blend_color(ctx.style.bg_secondary, ctx.style.surface_base, 0.38),
        }),
    );
    push_border(primitives, rect, ctx.style.border, ctx.sizing.border_width);
    let title_rect = inset_rect(
        rect,
        ctx.sizing.panel_inset.max(5.0),
        ctx.sizing.text_inset_y,
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from("FILTERS"),
            position: title_rect.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(title_rect.width().max(24.0)),
            align: TextAlign::Left,
        },
    );

    for (index, row_rect) in sidebar_filter_row_rects(rect, ctx.sizing)
        .into_iter()
        .enumerate()
    {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: row_rect,
                color: ctx.style.surface_overlay,
            }),
        );
        push_border(
            primitives,
            row_rect,
            ctx.style.border,
            ctx.sizing.border_width,
        );
        let label_rect = Rect::from_min_max(
            Point::new(
                row_rect.min.x + ctx.sizing.text_inset_x,
                row_rect.min.y + ctx.sizing.text_inset_y,
            ),
            Point::new(
                row_rect.min.x + (row_rect.width() * 0.42),
                row_rect.max.y - ctx.sizing.text_inset_y,
            ),
        );
        emit_text(
            text_runs,
            TextRun {
                text: FILTER_ROWS[index].to_string(),
                position: label_rect.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_primary,
                max_width: Some(label_rect.width().max(20.0)),
                align: TextAlign::Left,
            },
        );
        if FILTER_ROWS[index] == "Rating" {
            render_rating_filter_chips(ctx, primitives, text_runs, row_rect);
        } else {
            let summary = filter_summary(ctx, FILTER_ROWS[index]);
            let value_rect = Rect::from_min_max(
                Point::new(label_rect.max.x + 4.0, label_rect.min.y),
                Point::new(
                    row_rect.max.x - ctx.sizing.text_inset_x - 10.0,
                    label_rect.max.y,
                ),
            );
            emit_text(
                text_runs,
                TextRun {
                    text: summary,
                    position: value_rect.min,
                    font_size: ctx.sizing.font_meta,
                    color: ctx.style.text_muted,
                    max_width: Some(value_rect.width().max(24.0)),
                    align: TextAlign::Right,
                },
            );
            emit_text(
                text_runs,
                TextRun {
                    text: String::from(">"),
                    position: Point::new(
                        row_rect.max.x - ctx.sizing.text_inset_x - 6.0,
                        label_rect.min.y,
                    ),
                    font_size: ctx.sizing.font_meta,
                    color: ctx.style.text_muted,
                    max_width: Some(8.0),
                    align: TextAlign::Center,
                },
            );
        }
    }
}

/// Return row rectangles for the sidebar filter controls.
pub(in crate::gui::native_shell::state) fn sidebar_filter_row_rects(
    rect: Rect,
    sizing: SizingTokens,
) -> Vec<Rect> {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 2.0;
    let title_height = sizing.font_meta + sizing.text_inset_y + 4.0;
    let available = (rect.height() - pad * 2.0 - title_height - gap * 5.0).max(0.0);
    let row_height = (available / 6.0)
        .min(sizing.browser_row_height.max(18.0))
        .max(8.0);
    (0..6)
        .map(|index| {
            let min_y = rect.min.y + pad + title_height + (row_height + gap) * index as f32;
            Rect::from_min_max(
                Point::new(rect.min.x + pad, min_y),
                Point::new(rect.max.x - pad, (min_y + row_height).min(rect.max.y - pad)),
            )
        })
        .collect()
}

/// Return hit/render rectangles for the rating chips inside the rating row.
pub(in crate::gui::native_shell::state) fn sidebar_rating_chip_rects(
    rating_row: Rect,
    sizing: SizingTokens,
) -> [Rect; 8] {
    let chip_gap = 2.0_f32.max(sizing.border_width);
    let left = rating_row.min.x + (rating_row.width() * 0.43);
    let right = rating_row.max.x - sizing.text_inset_x;
    let available = (right - left - chip_gap * 7.0).max(0.0);
    let side = (available / 8.0).min(rating_row.height() - 4.0).max(0.0);
    std::array::from_fn(|index| {
        let x = left + (side + chip_gap) * index as f32;
        Rect::from_min_max(
            Point::new(x, rating_row.min.y + 2.0),
            Point::new((x + side).min(right), rating_row.min.y + 2.0 + side),
        )
    })
}

/// Render the rating filter chip strip.
fn render_rating_filter_chips(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    row_rect: Rect,
) {
    for (index, chip_rect) in sidebar_rating_chip_rects(row_rect, ctx.sizing)
        .into_iter()
        .enumerate()
    {
        if chip_rect.width() <= 1.0 {
            continue;
        }
        let active = ctx.model.browser.active_rating_filters[index];
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: chip_rect,
                color: if active {
                    blend_color(ctx.style.accent_mint, ctx.style.surface_overlay, 0.58)
                } else {
                    ctx.style.bg_tertiary
                },
            }),
        );
        push_border(
            primitives,
            chip_rect,
            if active {
                ctx.style.accent_mint
            } else {
                ctx.style.border
            },
            ctx.sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: rating_chip_label(index).to_string(),
                position: inset_rect(chip_rect, 1.0, 1.0).min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_primary,
                max_width: Some(chip_rect.width().max(6.0)),
                align: TextAlign::Center,
            },
        );
    }
}

/// Return the compact summary value for a filter row.
fn filter_summary(ctx: &StaticFrameCtx<'_>, label: &str) -> String {
    match label {
        "Format" => option_summary(ctx.model.sidebar_filters.formats.len(), "WAV"),
        "Bit Depth" => option_summary(ctx.model.sidebar_filters.bit_depths.len(), "Unavailable"),
        "Channels" => option_summary(ctx.model.sidebar_filters.channels.len(), "Unavailable"),
        "BPM" => {
            let filters = &ctx.model.sidebar_filters.bpms;
            if filters.is_empty() {
                String::from("Any")
            } else {
                filters
                    .iter()
                    .map(|facet| match facet {
                        crate::app_core::app_api::state::BrowserBpmFacet::Unknown => "?",
                        crate::app_core::app_api::state::BrowserBpmFacet::Slow => "<90",
                        crate::app_core::app_api::state::BrowserBpmFacet::Mid => "90-129",
                        crate::app_core::app_api::state::BrowserBpmFacet::Fast => "130+",
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }
        "Key" => option_summary(ctx.model.sidebar_filters.keys.len(), "Unknown"),
        _ => {
            let active = ctx
                .model
                .browser
                .active_rating_filters
                .iter()
                .filter(|active| **active)
                .count();
            if active == 0 {
                String::from("Any")
            } else {
                format!("{active} active")
            }
        }
    }
}

/// Return the compact summary for single-option unavailable/unknown facets.
fn option_summary(active_count: usize, label: &str) -> String {
    if active_count == 0 {
        String::from("Any")
    } else {
        label.to_string()
    }
}

/// Return the display label for a rating chip index.
fn rating_chip_label(index: usize) -> &'static str {
    match index {
        0 => "-3",
        1 => "-2",
        2 => "-1",
        3 => "0",
        4 => "1",
        5 => "2",
        6 => "3",
        7 => "L",
        _ => "",
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
