use super::*;
use crate::app_core::native_shell::runtime_contract::PlaybackAgeFilterChip;

pub(super) fn render_browser_filter_chips(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    toolbar: &BrowserToolbarLayout,
) {
    render_browser_rating_filter_chips(ctx, primitives, toolbar);
    render_browser_playback_age_filter_chips(ctx, primitives, toolbar);
    render_browser_marked_filter_chip(ctx, primitives, toolbar);
    render_browser_derived_label_filter_chip(ctx, primitives, toolbar);
}

fn render_browser_rating_filter_chips(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    toolbar: &BrowserToolbarLayout,
) {
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
}

fn render_browser_playback_age_filter_chips(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    toolbar: &BrowserToolbarLayout,
) {
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
        let active = ctx.model.browser.active_recency_filters[index];
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
}

fn render_browser_marked_filter_chip(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    toolbar: &BrowserToolbarLayout,
) {
    if toolbar.marked_filter_chip.width() <= 1.0 {
        return;
    }
    let active = ctx.model.browser.marked_filter_active;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: toolbar.marked_filter_chip,
            color: browser_marked_filter_chip_fill(ctx.style, active),
        }),
    );
    push_border(
        primitives,
        toolbar.marked_filter_chip,
        browser_marked_filter_chip_border(ctx.style, active),
        ctx.sizing.border_width,
    );
    let _ = emit_toolbar_svg_icon(
        primitives,
        WaveformToolbarIcon::BrowserMarked,
        centered_button_icon_rect(toolbar.marked_filter_chip, ctx.sizing),
        if active {
            ctx.style.text_primary
        } else {
            ctx.style.highlight_cyan
        },
    );
}

fn render_browser_derived_label_filter_chip(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    toolbar: &BrowserToolbarLayout,
) {
    if toolbar.derived_label_filter_chip.width() <= 1.0 {
        return;
    }
    let active = ctx.model.browser.derived_label_filter_active();
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: toolbar.derived_label_filter_chip,
            color: browser_marked_filter_chip_fill(ctx.style, active),
        }),
    );
    push_border(
        primitives,
        toolbar.derived_label_filter_chip,
        browser_marked_filter_chip_border(ctx.style, active),
        ctx.sizing.border_width,
    );
    let icon = if ctx.model.browser.derived_label_filter_negated() {
        WaveformToolbarIcon::Filter
    } else {
        WaveformToolbarIcon::BrowserMarked
    };
    let _ = emit_toolbar_svg_icon(
        primitives,
        icon,
        centered_button_icon_rect(toolbar.derived_label_filter_chip, ctx.sizing),
        if active {
            ctx.style.text_primary
        } else {
            ctx.style.highlight_cyan
        },
    );
}

pub(super) fn render_browser_toolbar_chrome(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    toolbar: &BrowserToolbarLayout,
) {
    render_toolbar_rect(
        primitives,
        toolbar.search_field,
        ctx.style.surface_base,
        blend_color(ctx.style.border_emphasis, ctx.style.text_primary, 0.35),
        ctx,
    );
    let busy_fill = if ctx.model.browser.busy {
        blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.45)
    } else {
        blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.40)
    };
    render_toolbar_rect(
        primitives,
        toolbar.activity_chip,
        busy_fill,
        ctx.style.border,
        ctx,
    );
    render_toolbar_rect(
        primitives,
        toolbar.sort_chip,
        ctx.style.surface_overlay,
        ctx.style.border,
        ctx,
    );
}

fn render_toolbar_rect(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    fill: Rgba8,
    border: Rgba8,
    ctx: &StaticFrameCtx<'_>,
) {
    if rect.width() <= 1.0 {
        return;
    }
    emit_primitive(primitives, Primitive::Rect(FillRect { rect, color: fill }));
    push_border(primitives, rect, border, ctx.sizing.border_width);
}

fn browser_playback_age_filter_icon(chip: PlaybackAgeFilterChip) -> WaveformToolbarIcon {
    match chip {
        PlaybackAgeFilterChip::NeverPlayed => WaveformToolbarIcon::BrowserNeverPlayed,
        PlaybackAgeFilterChip::OlderThanMonth => WaveformToolbarIcon::BrowserOlderThanMonth,
        PlaybackAgeFilterChip::OlderThanWeek => WaveformToolbarIcon::BrowserOlderThanWeek,
    }
}
