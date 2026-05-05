//! Geometry and hit-testing helpers for the native-shell options panel.

use super::actions::{
    audio_overview_button_defs, legacy_options_panel_button_defs, options_panel_title,
    picker_action, picker_options,
};
use super::*;

pub(super) fn status_right_text_rect(
    segment: Rect,
    sizing: SizingTokens,
    button_rect: Option<Rect>,
) -> Rect {
    let text_segment = if let Some(button_rect) = button_rect {
        let max_x = (button_rect.min.x - sizing.text_inset_x.max(3.0)).max(segment.min.x);
        Rect::from_min_max(segment.min, Point::new(max_x, segment.max.y))
    } else {
        segment
    };
    compute_status_text_line_rect(text_segment, sizing, sizing.font_status)
}

pub(super) fn options_panel_layout(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Option<OptionsPanelLayout> {
    if !model.options_panel.visible {
        return None;
    }
    let sizing = style.sizing;
    let panel_padding = sizing.overlay_padding.max(10.0);
    let title_height = sizing.overlay_button_height.max(22.0);
    let detail_height = if model.paired_device_panel().detail_label().is_some() {
        sizing.overlay_button_height.max(20.0)
    } else {
        0.0
    };
    let button_height = sizing.overlay_button_height.max(22.0);
    let button_gap = sizing.action_button_gap.max(4.0);
    let button_width = 268.0_f32.min((layout.content.width() - panel_padding * 2.0).max(180.0));
    let panel_width = button_width + (panel_padding * 2.0);
    let buttons = build_options_panel_buttons(model);
    let detail_gap = if detail_height > 0.0 { button_gap } else { 0.0 };
    let panel_height = panel_padding
        + title_height
        + detail_gap
        + detail_height
        + button_gap
        + (button_height * buttons.len() as f32)
        + (button_gap * buttons.len().saturating_sub(1) as f32)
        + panel_padding;
    let inset = sizing.panel_inset.max(6.0);
    let max_x = layout.top_bar.max.x - inset;
    let min_x = (max_x - panel_width).max(layout.content.min.x + inset);
    let min_y = layout.top_bar.max.y + inset;
    let max_y = (min_y + panel_height).min(layout.status_bar.min.y - inset);
    let min_y = (max_y - panel_height).max(layout.top_bar.max.y + inset);
    let panel_rect = Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(min_x + panel_width, max_y),
    );
    let title_rect = Rect::from_min_max(
        Point::new(
            panel_rect.min.x + panel_padding,
            panel_rect.min.y + panel_padding,
        ),
        Point::new(
            panel_rect.max.x - panel_padding,
            panel_rect.min.y + panel_padding + title_height,
        ),
    );
    let detail_rect = if detail_height > 0.0 {
        Some(Rect::from_min_max(
            Point::new(title_rect.min.x, title_rect.max.y + detail_gap),
            Point::new(
                title_rect.max.x,
                title_rect.max.y + detail_gap + detail_height,
            ),
        ))
    } else {
        None
    };
    let button_x = panel_rect.min.x + panel_padding;
    let mut button_y = detail_rect
        .map(|rect| rect.max.y + button_gap)
        .unwrap_or(title_rect.max.y + button_gap);
    let mut layout_buttons = Vec::with_capacity(buttons.len());
    for (text, action, active) in buttons {
        let rect = Rect::from_min_max(
            Point::new(button_x, button_y),
            Point::new(button_x + button_width, button_y + button_height),
        );
        layout_buttons.push(OptionsPanelButton {
            rect,
            text,
            action,
            active,
        });
        button_y += button_height + button_gap;
    }
    Some(OptionsPanelLayout {
        panel_rect,
        title_rect,
        detail_rect,
        title: options_panel_title(model),
        buttons: layout_buttons,
    })
}

pub(super) fn options_panel_contains_point(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> bool {
    options_panel_layout(layout, style, model).is_some_and(|panel| panel.panel_rect.contains(point))
}

pub(super) fn options_panel_action_at_point(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let panel = options_panel_layout(layout, style, model)?;
    panel
        .buttons
        .into_iter()
        .find(|button| button.rect.contains(point))
        .map(|button| button.action)
}

fn build_options_panel_buttons(model: &AppModel) -> Vec<(String, UiAction, bool)> {
    if let Some(target) = model.paired_device_panel().active_picker() {
        let mut buttons = Vec::new();
        buttons.push((String::from("Back"), UiAction::ShowOptionsOverview, false));
        buttons.extend(picker_options(model, target).iter().map(|item| {
            (
                item.label.clone(),
                picker_action(&item.value),
                item.selected,
            )
        }));
        return buttons;
    }

    let mut buttons = audio_overview_button_defs(model)
        .into_iter()
        .map(|(text, action)| (text, action, false))
        .collect::<Vec<_>>();
    buttons.extend(
        legacy_options_panel_button_defs(model)
            .into_iter()
            .map(|(text, action)| (text, action, false)),
    );
    buttons
}
