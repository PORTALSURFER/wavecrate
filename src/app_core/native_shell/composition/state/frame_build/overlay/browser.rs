use super::*;

pub(super) fn render_browser_tab_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    let sizing = style.sizing;
    let tabs = resolve_browser_tabs_surface_layout(
        layout.browser_tabs,
        sizing,
        &browser_tabs_surface_content(model),
    );
    let (samples_fill, map_fill, samples_text_color, map_text_color) = if !model.map.active {
        (
            blend_color(
                style.surface_overlay,
                style.bg_tertiary,
                style.state_selected_blend + 0.1,
            ),
            style.surface_base,
            blend_color(
                style.accent_mint,
                style.text_primary,
                style.state_selected_blend,
            ),
            style.text_muted,
        )
    } else {
        (
            style.surface_base,
            blend_color(
                style.surface_overlay,
                style.bg_tertiary,
                style.state_selected_blend + 0.1,
            ),
            style.text_muted,
            blend_color(
                style.accent_mint,
                style.text_primary,
                style.state_selected_blend,
            ),
        )
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: tabs.items,
            color: samples_fill,
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: tabs.map,
            color: map_fill,
        }),
    );
    push_border(primitives, tabs.items, style.border, sizing.border_width);
    push_border(
        primitives,
        tabs.map,
        blend_color(style.accent_mint, style.text_primary, 0.42),
        sizing.border_width,
    );
    let tabs_text_layout = compute_browser_tabs_text_layout(tabs.items, tabs.map, sizing);
    let samples_text = format!(
        "{} ({})",
        model.browser_chrome.items_tab_label,
        model
            .columns
            .get(1)
            .map(|column| column.item_count)
            .unwrap_or(0)
    );
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(
                &samples_text,
                tabs_text_layout.items_label.width().max(40.0),
                sizing.font_header,
            ),
            position: tabs_text_layout.items_label.min,
            font_size: sizing.font_header,
            color: samples_text_color,
            max_width: Some(tabs_text_layout.items_label.width().max(40.0)),
            align: TextAlign::Left,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from(model.browser_chrome.map_tab_label.as_str()),
            position: tabs_text_layout.map_label.min,
            font_size: sizing.font_header,
            color: map_text_color,
            max_width: Some(tabs_text_layout.map_label.width().max(40.0)),
            align: TextAlign::Left,
        },
    );
}

pub(super) fn render_source_context_menu(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    source_context_menu: Option<SourceContextMenuState>,
) {
    let Some((menu_panel, menu_buttons)) =
        source_context_menu_spec(layout, style, model, source_context_menu)
    else {
        return;
    };
    let sizing = style.sizing;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: menu_panel,
            color: style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        menu_panel,
        style.border_emphasis,
        sizing.border_width,
    );
    for button in menu_buttons {
        let label_rect = compute_action_button_text_rect(button.rect, sizing);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: if button.enabled {
                    blend_color(style.surface_base, style.bg_secondary, 0.36)
                } else {
                    style.control_disabled_fill
                },
            }),
        );
        push_border(
            primitives,
            button.rect,
            if button.enabled {
                style.border
            } else {
                style.grid_soft
            },
            sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: button.label.to_string(),
                position: label_rect.min,
                font_size: sizing.font_meta,
                color: if button.enabled {
                    button.text_color
                } else {
                    style.text_muted
                },
                max_width: Some(label_rect.width().max(16.0)),
                align: TextAlign::Center,
            },
        );
    }
}

pub(super) fn render_browser_context_menu(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    browser_context_menu: Option<BrowserContextMenuState>,
) {
    let Some((menu_panel, menu_buttons)) =
        browser_context_menu_spec(layout, style, model, browser_context_menu)
    else {
        return;
    };
    let sizing = style.sizing;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: menu_panel,
            color: style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        menu_panel,
        style.border_emphasis,
        sizing.border_width,
    );
    for button in menu_buttons {
        let label_rect = compute_action_button_text_rect(button.rect, sizing);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: if button.enabled {
                    blend_color(style.surface_base, style.bg_secondary, 0.36)
                } else {
                    style.control_disabled_fill
                },
            }),
        );
        push_border(
            primitives,
            button.rect,
            if button.enabled {
                style.border
            } else {
                style.grid_soft
            },
            sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: button.label.to_string(),
                position: label_rect.min,
                font_size: sizing.font_meta,
                color: if button.enabled {
                    button.text_color
                } else {
                    style.text_muted
                },
                max_width: Some(label_rect.width().max(16.0)),
                align: TextAlign::Center,
            },
        );
    }
}
