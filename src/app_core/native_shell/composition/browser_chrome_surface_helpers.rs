use super::*;
use crate::app_core::native_shell::composition::widget_nodes::{
    canvas_node, text_input_node, text_node, toggle_square_node,
};
use crate::{
    gui::types::Rect,
    layout::{CrossAlign, Insets, SizeModeCross, SizeModeMain, SlotParams},
    runtime::{SurfaceChild, SurfaceNode},
};

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
) -> Vec<SurfaceChild<()>> {
    let mut spacer_id = TOOLBAR_SORT_ID + 1;
    let mut children = Vec::new();
    for index in 0..BROWSER_RATING_FILTER_COUNT {
        if index > 0 {
            children.push(spacer_child(spacer_id, widths.filter_gap));
            spacer_id += 1;
        }
        children.push(SurfaceChild::new(
            fixed_slot(widths.filter_side, widths.filter_side),
            toggle_widget(
                TOOLBAR_RATING_BASE_ID + index as u64,
                chip_label(index),
                widths.filter_side,
            ),
        ));
    }
    children.push(spacer_child(spacer_id, widths.filter_group_gap));
    spacer_id += 1;
    for index in 0..BROWSER_PLAYBACK_AGE_FILTER_COUNT {
        if index > 0 {
            children.push(spacer_child(spacer_id, widths.filter_gap));
            spacer_id += 1;
        }
        children.push(SurfaceChild::new(
            fixed_slot(widths.filter_side, widths.filter_side),
            toggle_widget(
                TOOLBAR_PLAYBACK_BASE_ID + index as u64,
                playback_chip_label(index),
                widths.filter_side,
            ),
        ));
    }
    children.push(spacer_child(spacer_id, widths.filter_group_gap));
    spacer_id += 1;
    children.push(SurfaceChild::new(
        fixed_slot(widths.filter_side, widths.filter_side),
        toggle_widget(TOOLBAR_MARKED_ID, "Marked", widths.filter_side),
    ));
    children.push(spacer_child(spacer_id, widths.filter_gap));
    spacer_id += 1;
    children.push(SurfaceChild::new(
        fixed_slot(widths.filter_side, widths.filter_side),
        toggle_widget(
            TOOLBAR_DERIVED_LABEL_ID,
            "Derived label",
            widths.filter_side,
        ),
    ));
    children.push(spacer_child(spacer_id, widths.gap));
    spacer_id += 1;
    children.push(SurfaceChild::new(
        fixed_slot(widths.action_side, widths.action_side),
        button_widget(
            TOOLBAR_RANDOM_ID,
            "Random",
            widths.action_side,
            widths.action_side,
        ),
    ));
    children.push(spacer_child(spacer_id, widths.gap));
    spacer_id += 1;
    children.push(SurfaceChild::new(
        fixed_slot(widths.action_side, widths.action_side),
        button_widget(
            TOOLBAR_CLEANUP_ID,
            "Cleanup",
            widths.action_side,
            widths.action_side,
        ),
    ));
    children.push(spacer_child(spacer_id, widths.gap));
    spacer_id += 1;
    children.push(SurfaceChild::new(
        fixed_slot(widths.search_width, search_height),
        text_input_widget(
            TOOLBAR_SEARCH_ID,
            &content.search_value,
            &content.search_placeholder,
            widths.search_width,
            search_height,
        ),
    ));
    children.push(spacer_child(spacer_id, widths.gap));
    spacer_id += 1;
    children.push(SurfaceChild::new(
        fixed_slot(widths.tag_width, search_height),
        button_widget(TOOLBAR_TAGS_ID, "Tags", widths.tag_width, search_height),
    ));
    if widths.activity_width > 0.0 {
        children.push(spacer_child(spacer_id, widths.gap));
        spacer_id += 1;
    }
    children.push(SurfaceChild::new(
        fixed_slot(widths.activity_width, chip_label_height),
        text_widget(
            TOOLBAR_ACTIVITY_ID,
            &content.activity_label,
            widths.activity_width,
            chip_label_height,
            chip_label_height,
        ),
    ));
    if widths.sort_width > 0.0 {
        children.push(spacer_child(spacer_id, widths.gap));
    }
    children.push(SurfaceChild::new(
        fixed_slot(widths.sort_width, chip_label_height),
        text_widget(
            TOOLBAR_SORT_ID,
            &content.sort_label,
            widths.sort_width,
            chip_label_height,
            chip_label_height,
        ),
    ));
    for index in 0..BROWSER_TRIAGE_CHIP_COUNT {
        children.push(SurfaceChild::new(
            fixed_slot(0.0, chip_label_height),
            text_widget(
                TOOLBAR_TRIAGE_BASE_ID + index as u64,
                "",
                0.0,
                chip_label_height,
                chip_label_height,
            ),
        ));
    }
    children.push(SurfaceChild::new(
        SlotParams::fill(),
        canvas_node(
            TOOLBAR_TRIAGE_BASE_ID + BROWSER_TRIAGE_CHIP_COUNT as u64,
            1.0,
            1.0,
        ),
    ));
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

fn toggle_widget(id: u64, label: &str, side: f32) -> SurfaceNode<()> {
    toggle_square_node(id, label, side)
}

fn text_input_widget(
    id: u64,
    value: &str,
    placeholder: &str,
    width: f32,
    height: f32,
) -> SurfaceNode<()> {
    text_input_node(id, value, Some(placeholder), width, height)
}

fn text_widget(id: u64, text: &str, width: f32, height: f32, font_size: f32) -> SurfaceNode<()> {
    text_node(id, text, width, height, font_size)
}

fn fixed_slot(width: f32, height: f32) -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Fixed(width.max(0.0)),
        size_cross: SizeModeCross::Fixed(height.max(0.0)),
        constraints: Constraints::new(
            width.max(0.0),
            width.max(0.0),
            height.max(0.0),
            height.max(0.0),
        ),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Center),
        allow_fixed_compress: false,
    }
}

fn spacer_child(id: u64, width: f32) -> SurfaceChild<()> {
    SurfaceChild::new(fixed_slot(width.max(0.0), 1.0), canvas_node(id, width, 1.0))
}
