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
    let output = crate::gui::paint::text_field_paint(crate::gui::paint::TextFieldPaint {
        geometry: crate::gui::paint::TextFieldPaintGeometry {
            field_rect,
            text_rect,
        },
        content: crate::gui::paint::TextFieldPaintContent {
            text: visual.text.clone(),
            caret_offset: visual.caret_offset,
            selection_offsets: visual.selection_offsets,
            font_size: sizing.font_meta,
        },
        colors: crate::gui::paint::TextFieldPaintColors {
            fill_color,
            border_color,
            selection_color,
            caret_color,
            text_color: style.text_primary,
        },
        stroke: crate::gui::paint::TextFieldPaintStroke {
            stroke_width: sizing.border_width,
        },
    });

    for primitive in output.primitives {
        emit_primitive(primitives, primitive);
    }
    if let Some(text_run) = output.text_run {
        emit_text(text_runs, text_run);
    }
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
            blend_color(style.accent_warning, style.accent_danger, 0.6)
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
