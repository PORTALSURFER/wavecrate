//! Browser toolbar color and hover rendering helpers.

use super::super::super::*;

pub(in crate::gui::native_shell::state) fn browser_marked_filter_chip_contains_point(
    chip: Rect,
    point: Point,
) -> bool {
    chip.width() > 1.0 && chip.contains(point)
}

pub(in crate::gui::native_shell::state) fn browser_playback_age_filter_chip_fill(
    style: &StyleTokens,
    chip: crate::app::PlaybackAgeFilterChip,
    active: bool,
) -> Rgba8 {
    let tint = match chip {
        crate::app::PlaybackAgeFilterChip::NeverPlayed => style.text_primary,
        crate::app::PlaybackAgeFilterChip::OlderThanMonth => style.text_muted,
        crate::app::PlaybackAgeFilterChip::OlderThanWeek => style.bg_tertiary,
    };
    let amount = if active { 0.42 } else { 0.18 };
    blend_color(
        if active {
            style.surface_overlay
        } else {
            style.surface_base
        },
        tint,
        amount,
    )
}

pub(in crate::gui::native_shell::state) fn browser_playback_age_filter_chip_border(
    style: &StyleTokens,
    chip: crate::app::PlaybackAgeFilterChip,
    active: bool,
) -> Rgba8 {
    if active {
        match chip {
            crate::app::PlaybackAgeFilterChip::NeverPlayed => {
                blend_color(style.text_primary, style.border_emphasis, 0.48)
            }
            crate::app::PlaybackAgeFilterChip::OlderThanMonth => {
                blend_color(style.text_muted, style.border_emphasis, 0.42)
            }
            crate::app::PlaybackAgeFilterChip::OlderThanWeek => {
                blend_color(style.border_emphasis, style.text_primary, 0.34)
            }
        }
    } else {
        blend_color(style.border, style.surface_overlay, 0.25)
    }
}

pub(in crate::gui::native_shell::state) fn browser_playback_age_filter_chip_hover_fill(
    style: &StyleTokens,
    chip: crate::app::PlaybackAgeFilterChip,
    active: bool,
    motion_wave: f32,
) -> Rgba8 {
    translucent_overlay_color(
        browser_playback_age_filter_chip_fill(style, chip, active),
        style.text_primary,
        if active { 0.26 } else { 0.16 } + (motion_wave * 0.04),
    )
}

pub(in crate::gui::native_shell::state) fn browser_playback_age_filter_chip_hover_border(
    style: &StyleTokens,
    chip: crate::app::PlaybackAgeFilterChip,
    active: bool,
    motion_wave: f32,
) -> Rgba8 {
    blend_color(
        browser_playback_age_filter_chip_border(style, chip, active),
        style.text_primary,
        0.46 + (motion_wave * 0.08),
    )
}

pub(in crate::gui::native_shell::state) fn render_browser_playback_age_filter_chip_hover_overlay(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    chip_rect: Rect,
    chip: crate::app::PlaybackAgeFilterChip,
    active: bool,
    motion_wave: f32,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: chip_rect,
            color: browser_playback_age_filter_chip_hover_fill(style, chip, active, motion_wave),
        }),
    );
    push_border(
        primitives,
        chip_rect,
        browser_playback_age_filter_chip_hover_border(style, chip, active, motion_wave),
        sizing.border_width,
    );
}

pub(in crate::gui::native_shell::state) fn browser_marked_filter_chip_fill(
    style: &StyleTokens,
    active: bool,
) -> Rgba8 {
    let base = if active {
        style.surface_overlay
    } else {
        style.surface_base
    };
    blend_color(base, style.highlight_cyan, if active { 0.34 } else { 0.16 })
}

pub(in crate::gui::native_shell::state) fn browser_marked_filter_chip_border(
    style: &StyleTokens,
    active: bool,
) -> Rgba8 {
    if active {
        blend_color(style.highlight_cyan, style.text_primary, 0.32)
    } else {
        blend_color(style.border, style.surface_overlay, 0.25)
    }
}

pub(in crate::gui::native_shell::state) fn browser_marked_filter_chip_hover_fill(
    style: &StyleTokens,
    active: bool,
    motion_wave: f32,
) -> Rgba8 {
    translucent_overlay_color(
        browser_marked_filter_chip_fill(style, active),
        style.highlight_cyan,
        if active { 0.34 } else { 0.22 } + (motion_wave * 0.04),
    )
}

pub(in crate::gui::native_shell::state) fn browser_marked_filter_chip_hover_border(
    style: &StyleTokens,
    active: bool,
    motion_wave: f32,
) -> Rgba8 {
    blend_color(
        browser_marked_filter_chip_border(style, active),
        style.highlight_cyan,
        0.56 + (motion_wave * 0.08),
    )
}

pub(in crate::gui::native_shell::state) fn render_browser_search_field_hover_overlay(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    search_field_rect: Rect,
    motion_wave: f32,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: search_field_rect,
            color: browser_search_field_hover_fill(style, motion_wave),
        }),
    );
    push_border(
        primitives,
        search_field_rect,
        browser_search_field_hover_border(style, motion_wave),
        sizing.border_width,
    );
}

pub(in crate::gui::native_shell::state) fn render_browser_rating_filter_chip_hover_overlay(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    chip_rect: Rect,
    rating_level: i8,
    active: bool,
    motion_wave: f32,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: chip_rect,
            color: browser_rating_filter_chip_hover_fill(style, rating_level, active, motion_wave),
        }),
    );
    push_border(
        primitives,
        chip_rect,
        browser_rating_filter_chip_hover_border(style, rating_level, active, motion_wave),
        sizing.border_width,
    );
}

pub(in crate::gui::native_shell::state) fn browser_search_field_hover_fill(
    style: &StyleTokens,
    motion_wave: f32,
) -> Rgba8 {
    translucent_overlay_color(
        style.surface_base,
        style.bg_tertiary,
        0.22 + (motion_wave * 0.04),
    )
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_chip_hover_fill(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
    motion_wave: f32,
) -> Rgba8 {
    let tint = if rating_level < 0 {
        style.accent_trash
    } else if rating_level > 0 {
        style.accent_mint
    } else {
        style.highlight_orange_soft
    };
    let amount = if active { 0.34 } else { 0.2 } + (motion_wave * 0.04);
    translucent_overlay_color(
        browser_rating_filter_chip_fill(style, rating_level, active),
        tint,
        amount,
    )
}

pub(in crate::gui::native_shell::state) fn browser_search_field_hover_border(
    style: &StyleTokens,
    motion_wave: f32,
) -> Rgba8 {
    blend_color(
        style.border_emphasis,
        style.text_primary,
        0.48 + (motion_wave * 0.06),
    )
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_chip_hover_border(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
    motion_wave: f32,
) -> Rgba8 {
    let tint = if rating_level < 0 {
        style.accent_trash
    } else if rating_level > 0 {
        style.accent_mint
    } else {
        style.highlight_orange
    };
    blend_color(
        browser_rating_filter_chip_border(style, rating_level, active),
        tint,
        0.52 + (motion_wave * 0.08),
    )
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_chip_fill(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
) -> Rgba8 {
    let tint = if rating_level < 0 {
        style.accent_trash
    } else if rating_level == 4 {
        blend_color(style.accent_mint, style.text_primary, 0.28)
    } else if rating_level > 0 {
        style.accent_mint
    } else if active {
        style.highlight_orange
    } else {
        style.text_primary
    };
    let amount = if active {
        0.9
    } else if rating_level == 0 {
        0.14
    } else {
        0.18
    };
    blend_color(
        if active {
            style.surface_overlay
        } else {
            style.surface_base
        },
        tint,
        amount,
    )
}

pub(in crate::gui::native_shell::state) fn browser_rating_filter_chip_border(
    style: &StyleTokens,
    rating_level: i8,
    active: bool,
) -> Rgba8 {
    if active {
        if rating_level < 0 {
            blend_color(style.accent_trash, style.text_primary, 0.24)
        } else if rating_level == 4 {
            blend_color(style.accent_mint, style.text_primary, 0.44)
        } else if rating_level > 0 {
            blend_color(style.accent_mint, style.text_primary, 0.24)
        } else {
            blend_color(style.highlight_orange, style.text_primary, 0.22)
        }
    } else {
        blend_color(style.border, style.surface_overlay, 0.25)
    }
}
