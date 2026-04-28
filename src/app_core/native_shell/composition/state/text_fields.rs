//! Visual state and overlay rendering for active single-line shell text fields.

use super::*;

/// Precomputed visual state for one active shell text field.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextFieldVisualState {
    /// Visible text substring that fits the field width.
    pub(crate) text: String,
    /// Caret x-offset inside the field text rect.
    pub(crate) caret_offset: f32,
    /// Selected x-span inside the field text rect, when any.
    pub(crate) selection_offsets: Option<(f32, f32)>,
}

/// Render one active shell text field fill, selection, text, and caret.
pub(crate) fn render_active_text_field(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    field_rect: Rect,
    text_rect: Rect,
    visual: &TextFieldVisualState,
    fill_color: Rgba8,
    border_color: Rgba8,
    selection_color: Rgba8,
    caret_color: Rgba8,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: field_rect,
            color: fill_color,
        }),
    );
    push_border(primitives, field_rect, border_color, sizing.border_width);
    if let Some((start, end)) = visual.selection_offsets
        && end > start
    {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(text_rect.min.x + start, text_rect.min.y),
                    Point::new(text_rect.min.x + end, text_rect.max.y),
                ),
                color: selection_color,
            }),
        );
    }
    if !visual.text.is_empty() {
        emit_text(
            text_runs,
            TextRun {
                text: visual.text.clone(),
                position: text_rect.min,
                font_size: sizing.font_meta,
                color: style.text_primary,
                max_width: Some(text_rect.width().max(24.0)),
                align: TextAlign::Left,
            },
        );
    }
    let caret_rect = Rect::from_min_max(
        Point::new(text_rect.min.x + visual.caret_offset, text_rect.min.y),
        Point::new(
            text_rect.min.x + visual.caret_offset + sizing.border_width.max(1.0),
            text_rect.max.y,
        ),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: caret_rect,
            color: caret_color,
        }),
    );
}

/// Render the active browser-search editor fill, selection, text, and caret.
pub(crate) fn render_active_browser_search_editor(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    search_field_rect: Rect,
    search_text_rect: Rect,
    visual: &TextFieldVisualState,
) {
    render_active_text_field(
        primitives,
        text_runs,
        style,
        sizing,
        search_field_rect,
        search_text_rect,
        visual,
        browser_search_field_active_fill(style),
        browser_search_field_active_border(style),
        browser_search_selection_fill(style),
        browser_search_caret_color(style),
    );
}

/// Render the active waveform-BPM editor fill, selection, text, and caret.
pub(crate) fn render_active_waveform_bpm_editor(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    input_rect: Rect,
    input_text_rect: Rect,
    visual: &TextFieldVisualState,
) {
    render_active_text_field(
        primitives,
        text_runs,
        style,
        sizing,
        input_rect,
        input_text_rect,
        visual,
        browser_search_field_active_fill(style),
        browser_search_field_active_border(style),
        browser_search_selection_fill(style),
        browser_search_caret_color(style),
    );
}

/// Render the active inline folder-create editor fill, selection, text, and caret.
pub(crate) fn render_active_folder_create_editor(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    input_rect: Rect,
    input_text_rect: Rect,
    visual: &TextFieldVisualState,
    has_error: bool,
) {
    render_active_text_field(
        primitives,
        text_runs,
        style,
        sizing,
        input_rect,
        input_text_rect,
        visual,
        browser_search_field_active_fill(style),
        if has_error {
            blend_color(style.accent_warning, style.accent_trash, 0.6)
        } else {
            browser_search_field_active_border(style)
        },
        browser_search_selection_fill(style),
        browser_search_caret_color(style),
    );
}

pub(crate) fn text_field_visual_signature(visual: Option<&TextFieldVisualState>) -> u64 {
    let Some(visual) = visual else {
        return 0;
    };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    visual.text.hash(&mut hasher);
    visual.caret_offset.to_bits().hash(&mut hasher);
    visual
        .selection_offsets
        .map(|(start, end)| (start.to_bits(), end.to_bits()))
        .hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn browser_search_field_active_fill(style: &StyleTokens) -> Rgba8 {
    translucent_overlay_color(style.surface_base, style.highlight_orange_soft, 0.22)
}

pub(crate) fn browser_search_field_active_border(style: &StyleTokens) -> Rgba8 {
    blend_color(style.border_emphasis, style.highlight_orange, 0.6)
}

fn browser_search_selection_fill(style: &StyleTokens) -> Rgba8 {
    translucent_overlay_color(style.highlight_orange_soft, style.text_primary, 0.22)
}

fn browser_search_caret_color(style: &StyleTokens) -> Rgba8 {
    blend_color(style.text_primary, style.highlight_orange, 0.24)
}
