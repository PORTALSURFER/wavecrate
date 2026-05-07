use super::*;
use crate::gui::types::Rect;
use radiant::prelude as ui;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BrowserToolbarSurfaceWidths {
    pub(super) horizontal_padding: f32,
    pub(super) filter_side: f32,
    pub(super) action_side: f32,
    pub(super) search_width: f32,
    pub(super) tag_width: f32,
    pub(super) activity_width: f32,
    pub(super) sort_width: f32,
    pub(super) gap: f32,
    pub(super) filter_gap: f32,
    pub(super) filter_group_gap: f32,
}

pub(super) fn build_toolbar_children(
    content: &BrowserToolbarSurfaceContent,
    search_height: f32,
    chip_label_height: f32,
    widths: BrowserToolbarSurfaceWidths,
) -> Vec<ui::ViewNode<()>> {
    let mut spacer_id = TOOLBAR_SORT_ID + 1;
    let mut children = Vec::new();
    for index in 0..BROWSER_RATING_FILTER_COUNT {
        if index > 0 {
            children.push(spacer_child(spacer_id, widths.filter_gap));
            spacer_id += 1;
        }
        children.push(
            ui::passive_toggle(chip_label(index), false)
                .id(TOOLBAR_RATING_BASE_ID + index as u64)
                .size(widths.filter_side, widths.filter_side)
                .width(widths.filter_side)
                .height(widths.filter_side),
        );
    }
    children.push(spacer_child(spacer_id, widths.filter_group_gap));
    spacer_id += 1;
    for index in 0..BROWSER_PLAYBACK_AGE_FILTER_COUNT {
        if index > 0 {
            children.push(spacer_child(spacer_id, widths.filter_gap));
            spacer_id += 1;
        }
        children.push(
            ui::passive_toggle(playback_chip_label(index), false)
                .id(TOOLBAR_PLAYBACK_BASE_ID + index as u64)
                .size(widths.filter_side, widths.filter_side)
                .width(widths.filter_side)
                .height(widths.filter_side),
        );
    }
    children.push(spacer_child(spacer_id, widths.filter_group_gap));
    spacer_id += 1;
    children.push(
        ui::passive_toggle("Marked", false)
            .id(TOOLBAR_MARKED_ID)
            .size(widths.filter_side, widths.filter_side)
            .width(widths.filter_side)
            .height(widths.filter_side),
    );
    children.push(spacer_child(spacer_id, widths.filter_gap));
    spacer_id += 1;
    children.push(
        ui::passive_toggle("Derived label", false)
            .id(TOOLBAR_DERIVED_LABEL_ID)
            .size(widths.filter_side, widths.filter_side)
            .width(widths.filter_side)
            .height(widths.filter_side),
    );
    children.push(spacer_child(spacer_id, widths.gap));
    spacer_id += 1;
    children.push(
        ui::passive_button("Random")
            .id(TOOLBAR_RANDOM_ID)
            .size(widths.action_side, widths.action_side)
            .width(widths.action_side)
            .height(widths.action_side),
    );
    children.push(spacer_child(spacer_id, widths.gap));
    spacer_id += 1;
    children.push(
        ui::passive_button("Cleanup")
            .id(TOOLBAR_CLEANUP_ID)
            .size(widths.action_side, widths.action_side)
            .width(widths.action_side)
            .height(widths.action_side),
    );
    children.push(spacer_child(spacer_id, widths.gap));
    spacer_id += 1;
    children.push(
        ui::passive_text_input(&content.search_value, &content.search_placeholder)
            .id(TOOLBAR_SEARCH_ID)
            .size(widths.search_width, search_height)
            .width(widths.search_width)
            .height(search_height),
    );
    children.push(spacer_child(spacer_id, widths.gap));
    spacer_id += 1;
    children.push(
        ui::passive_button("Tags")
            .id(TOOLBAR_TAGS_ID)
            .size(widths.tag_width, search_height)
            .width(widths.tag_width)
            .height(search_height),
    );
    if widths.activity_width > 0.0 {
        children.push(spacer_child(spacer_id, widths.gap));
        spacer_id += 1;
    }
    children.push(
        ui::text(&content.activity_label)
            .id(TOOLBAR_ACTIVITY_ID)
            .size(widths.activity_width, chip_label_height)
            .baseline((chip_label_height * 0.75).max(0.0))
            .width(widths.activity_width)
            .height(chip_label_height),
    );
    if widths.sort_width > 0.0 {
        children.push(spacer_child(spacer_id, widths.gap));
    }
    children.push(
        ui::text(&content.sort_label)
            .id(TOOLBAR_SORT_ID)
            .size(widths.sort_width, chip_label_height)
            .baseline((chip_label_height * 0.75).max(0.0))
            .width(widths.sort_width)
            .height(chip_label_height),
    );
    for index in 0..BROWSER_TRIAGE_CHIP_COUNT {
        children.push(
            ui::text("")
                .id(TOOLBAR_TRIAGE_BASE_ID + index as u64)
                .size(1.0, chip_label_height)
                .baseline((chip_label_height * 0.75).max(0.0))
                .width(0.0)
                .height(chip_label_height),
        );
    }
    children.push(
        ui::canvas()
            .id(TOOLBAR_TRIAGE_BASE_ID + BROWSER_TRIAGE_CHIP_COUNT as u64)
            .fill(),
    );
    children
}

pub(super) fn browser_toolbar_surface_widths(
    toolbar: Rect,
    sizing: SizingTokens,
) -> BrowserToolbarSurfaceWidths {
    let gap = sizing.action_button_gap.max(1.0);
    let filter_gap = sizing.border_width.max(1.0) + 1.0;
    let filter_group_gap = filter_gap + sizing.border_width.max(1.0) + 2.0;
    let horizontal_padding = sizing.text_inset_x.max(0.0);
    let available = (toolbar.width() - (horizontal_padding * 2.0)).max(0.0);
    if available <= 1.0 || toolbar.height() <= 0.0 {
        return BrowserToolbarSurfaceWidths {
            horizontal_padding,
            filter_side: 0.0,
            action_side: 0.0,
            search_width: 0.0,
            tag_width: 0.0,
            activity_width: 0.0,
            sort_width: 0.0,
            gap,
            filter_gap,
            filter_group_gap,
        };
    }
    let desired_search_width = ((toolbar.width() * sizing.browser_search_field_ratio)
        .max(sizing.browser_search_field_min_width))
    .min(
        (available * sizing.browser_search_field_ratio).max(sizing.browser_search_field_min_width),
    );
    let action_side = (toolbar.height() - (sizing.text_inset_y * 0.4))
        .floor()
        .clamp(14.0, 24.0)
        .min((available - gap).max(0.0));
    let action_cluster_width = if action_side > 0.0 {
        (action_side * 2.0) + gap
    } else {
        0.0
    };
    let min_search_width = sizing.browser_search_field_min_width.min(available);
    let filter_side = 0.0;
    let filter_total_width = 0.0;
    let derived_label_width = 0.0;
    let remaining_after_filters =
        (available - filter_total_width - derived_label_width - action_cluster_width - (gap * 2.0))
            .max(0.0);
    BrowserToolbarSurfaceWidths {
        horizontal_padding,
        filter_side,
        action_side,
        search_width: desired_search_width
            .min(remaining_after_filters.max(min_search_width))
            .max(0.0),
        tag_width: 0.0,
        activity_width: 0.0,
        sort_width: 0.0,
        gap,
        filter_gap,
        filter_group_gap,
    }
}

pub(super) fn browser_sort_label(model: &AppModel) -> String {
    let sort_label = if model.browser_chrome.sort_order_label.is_empty() {
        model.browser.sort_label.as_deref().unwrap_or("List order")
    } else {
        model.browser_chrome.sort_order_label.as_str()
    };
    if model.browser_chrome.sort_prefix_label.is_empty() {
        String::from(sort_label)
    } else {
        format!("{}: {}", model.browser_chrome.sort_prefix_label, sort_label)
    }
}

fn chip_label(index: usize) -> &'static str {
    match index {
        0 => "-3",
        1 => "-2",
        2 => "-1",
        3 => "0",
        4 => "1",
        5 => "2",
        6 => "3",
        7 => "4",
        _ => "",
    }
}

fn playback_chip_label(index: usize) -> &'static str {
    match index {
        0 => "Never",
        1 => "Month",
        2 => "Week",
        _ => "",
    }
}

fn spacer_child(id: u64, width: f32) -> ui::ViewNode<()> {
    ui::spacer()
        .id(id)
        .size(width.max(1.0), 1.0)
        .width(width.max(0.0))
        .height(1.0)
}
