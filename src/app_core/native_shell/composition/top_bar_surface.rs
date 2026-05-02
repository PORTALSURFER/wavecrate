//! Generic top-bar surface projection for the native-shell compatibility layer.
//!
//! This mirrors the status-bar pilot pattern for the bounded top-bar chrome:
//! generic Radiant containers and widgets own the compact title, volume,
//! options, and update-action composition, while the compatibility shell still
//! paints those resolved rects with the existing native theme.

use super::style::SizingTokens;
use crate::{
    app::{AppModel, UiAction, UpdateStatusModel},
    gui::types::{Point, Rect, Vector2},
    layout::{
        Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, MainAlign, OverflowPolicy,
        SizeModeCross, SizeModeMain, SlotParams, layout_tree,
    },
    runtime::{SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{ButtonWidget, CanvasWidget, TextWidget, WidgetSizing, WidgetSpec},
};

const TOP_ROOT_ID: u64 = 1040;
const TOP_ROW_ID: u64 = 1041;
const TOP_TITLE_CLUSTER_ID: u64 = 1042;
const TOP_ACTION_CLUSTER_ID: u64 = 1043;
const TOP_TITLE_ROW_ID: u64 = 1044;
const TOP_VOLUME_METER_ID: u64 = 1045;
const TOP_VOLUME_VALUE_ID: u64 = 1046;
const TOP_VOLUME_LABEL_ID: u64 = 1047;
const TOP_TITLE_TEXT_ID: u64 = 1048;
const TOP_ACTION_ROW_ID: u64 = 1049;
const TOP_ACTION_SPACER_ID: u64 = 1050;
const TOP_OPTIONS_BUTTON_ID: u64 = 1051;
const TOP_UPDATE_BUTTON_BASE_ID: u64 = 1060;

/// User-facing content projected into the generic top-bar surface.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TopBarSurfaceContent {
    /// Product title projected into the compact chrome band.
    pub title: String,
    /// Formatted master-volume value.
    pub volume_value: String,
    /// Short volume label paired with the meter.
    pub volume_label: String,
    /// Compact audio-engine chip label shown on the options button.
    pub options_label: String,
    /// Bounded update actions projected into the action cluster.
    pub update_actions: Vec<TopBarUpdateActionSpec>,
}

/// One projected update action hosted inside the generic top-bar surface.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TopBarUpdateActionSpec {
    /// Stable automation slug used for semantic node ids.
    pub node_slug: &'static str,
    /// User-facing action label.
    pub label: &'static str,
    /// Native action emitted when the button activates.
    pub action: UiAction,
    /// Whether the action is currently interactive.
    pub enabled: bool,
}

/// Resolved button geometry for one projected update action.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TopBarUpdateButtonLayout {
    /// Action metadata associated with the rendered button.
    pub spec: TopBarUpdateActionSpec,
    /// Resolved button bounds.
    pub rect: Rect,
}

/// Resolved layout rectangles for the generic top-bar surface.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TopBarSurfaceLayout {
    /// Left host cluster that contains the volume affordance and title copy.
    pub title_cluster: Rect,
    /// Right host cluster that contains update actions and the options button.
    pub action_cluster: Rect,
    /// Title text widget bounds.
    pub title_text_rect: Rect,
    /// Volume meter canvas bounds.
    pub volume_meter_rect: Rect,
    /// Volume value text bounds.
    pub volume_value_rect: Rect,
    /// Volume label text bounds.
    pub volume_label_rect: Rect,
    /// Options button bounds, when enough room remains.
    pub options_button_rect: Option<Rect>,
    /// Visible update action buttons resolved inside the action cluster.
    pub update_buttons: Vec<TopBarUpdateButtonLayout>,
}

/// Build user-facing top-bar surface content from the projected app model.
pub(crate) fn top_bar_surface_content(model: &AppModel) -> TopBarSurfaceContent {
    TopBarSurfaceContent {
        title: model.title.clone(),
        volume_value: format!("{:.2}", model.volume.clamp(0.0, 1.0)),
        volume_label: String::from("Vol"),
        options_label: model.paired_device_panel().status_label().to_string(),
        update_actions: top_bar_update_action_specs(model),
    }
}

/// Build the update-action descriptors projected into the top-bar chrome.
pub(crate) fn top_bar_update_action_specs(model: &AppModel) -> Vec<TopBarUpdateActionSpec> {
    match model.update.status {
        UpdateStatusModel::Idle => vec![TopBarUpdateActionSpec {
            node_slug: "check",
            label: "Check",
            action: UiAction::CheckForUpdates,
            enabled: true,
        }],
        UpdateStatusModel::Checking => Vec::new(),
        UpdateStatusModel::Available => {
            let mut buttons = Vec::new();
            if model.update.available_url.is_some() {
                buttons.push(TopBarUpdateActionSpec {
                    node_slug: "open",
                    label: "Open",
                    action: UiAction::OpenUpdateLink,
                    enabled: true,
                });
                buttons.push(TopBarUpdateActionSpec {
                    node_slug: "install",
                    label: "Install",
                    action: UiAction::InstallUpdate,
                    enabled: true,
                });
            }
            buttons.push(TopBarUpdateActionSpec {
                node_slug: "dismiss",
                label: "Dismiss",
                action: UiAction::DismissUpdate,
                enabled: true,
            });
            buttons
        }
        UpdateStatusModel::Error => vec![TopBarUpdateActionSpec {
            node_slug: "check",
            label: "Retry",
            action: UiAction::CheckForUpdates,
            enabled: true,
        }],
    }
}

/// Resolve the generic top-bar surface layout inside one shell top-bar rect.
pub(crate) fn resolve_top_bar_surface_layout(
    top_bar: Rect,
    sizing: SizingTokens,
    content: &TopBarSurfaceContent,
) -> TopBarSurfaceLayout {
    let surface = build_top_bar_surface(content, sizing, top_bar.width());
    let output = layout_tree(&surface.layout_node(), top_bar);
    let empty = Rect::from_min_max(top_bar.min, top_bar.min);
    let title_cluster = clamp_rect_to_bounds(
        rect_for(&output.rects, TOP_TITLE_CLUSTER_ID, empty),
        top_bar,
    );
    let action_cluster = clamp_rect_to_bounds(
        rect_for(&output.rects, TOP_ACTION_CLUSTER_ID, empty),
        top_bar,
    );
    let options_button_rect = rect_for(&output.rects, TOP_OPTIONS_BUTTON_ID, empty);
    let update_buttons = content
        .update_actions
        .iter()
        .enumerate()
        .filter_map(|(index, spec)| {
            let rect = clamp_rect_to_bounds(
                rect_for(
                    &output.rects,
                    TOP_UPDATE_BUTTON_BASE_ID + index as u64,
                    empty,
                ),
                top_bar,
            );
            (rect.width() > 0.0 && rect.height() > 0.0).then(|| TopBarUpdateButtonLayout {
                spec: spec.clone(),
                rect,
            })
        })
        .collect();
    TopBarSurfaceLayout {
        title_cluster,
        action_cluster,
        title_text_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, TOP_TITLE_TEXT_ID, empty),
            top_bar,
        ),
        volume_meter_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, TOP_VOLUME_METER_ID, empty),
            top_bar,
        ),
        volume_value_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, TOP_VOLUME_VALUE_ID, empty),
            top_bar,
        ),
        volume_label_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, TOP_VOLUME_LABEL_ID, empty),
            top_bar,
        ),
        options_button_rect: (options_button_rect.width() > 0.0
            && options_button_rect.height() > 0.0)
            .then_some(clamp_rect_to_bounds(options_button_rect, top_bar)),
        update_buttons,
    }
}

/// Resolve the compact options-button rect without projecting model-specific copy.
pub(crate) fn top_bar_options_button_rect(top_bar: Rect, sizing: SizingTokens) -> Option<Rect> {
    resolve_top_bar_surface_layout(
        top_bar,
        sizing,
        &TopBarSurfaceContent {
            title: String::new(),
            volume_value: String::new(),
            volume_label: String::new(),
            options_label: String::new(),
            update_actions: Vec::new(),
        },
    )
    .options_button_rect
}

fn build_top_bar_surface(
    content: &TopBarSurfaceContent,
    sizing: SizingTokens,
    viewport_width: f32,
) -> UiSurface<()> {
    let action_cluster_width = top_bar_action_cluster_width(viewport_width, sizing);
    let title_text_height = sizing.font_title.max(1.0);
    let volume_meter_width = sizing.top_volume_meter_width.max(26.0);
    let volume_meter_height = sizing.top_volume_meter_height.max(3.0);
    let volume_value_width = 44.0;
    let volume_label_width = 28.0;
    let volume_gap = sizing.action_button_gap.max(2.0);
    let button_height = top_bar_button_height(sizing);
    let options_button_width = top_bar_options_button_width(sizing, button_height);
    let update_widths = visible_update_widths(
        &content.update_actions,
        action_cluster_width,
        options_button_width,
        sizing,
    );
    let visible_update_count = update_widths.len();

    UiSurface::new(SurfaceNode::container(
        TOP_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: sizing.panel_inset.max(0.0),
                right: sizing.panel_inset.max(0.0),
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::new(
            SlotParams::fill(),
            SurfaceNode::container(
                TOP_ROW_ID,
                ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing: sizing.top_bar_cluster_gap.max(0.0),
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    SurfaceChild::new(
                        SlotParams::fill(),
                        build_title_cluster(
                            content,
                            sizing,
                            volume_meter_width,
                            volume_meter_height,
                            volume_value_width,
                            volume_label_width,
                            volume_gap,
                            title_text_height,
                        ),
                    ),
                    SurfaceChild::new(
                        fixed_slot(action_cluster_width),
                        build_action_cluster(
                            content,
                            sizing,
                            options_button_width,
                            button_height,
                            &update_widths,
                            visible_update_count,
                        ),
                    ),
                ],
            ),
        )],
    ))
}

fn build_title_cluster(
    content: &TopBarSurfaceContent,
    sizing: SizingTokens,
    meter_width: f32,
    meter_height: f32,
    value_width: f32,
    label_width: f32,
    gap: f32,
    title_height: f32,
) -> SurfaceNode<()> {
    SurfaceNode::container(
        TOP_TITLE_CLUSTER_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: (sizing.text_inset_x + sizing.header_label_gutter).max(0.0),
                right: (sizing.text_inset_x + sizing.header_label_gutter).max(0.0),
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::new(
            SlotParams::fill(),
            SurfaceNode::container(
                TOP_TITLE_ROW_ID,
                ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing: gap,
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Center,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    SurfaceChild::new(
                        fixed_slot_with_cross(meter_width, meter_height),
                        SurfaceNode::widget(
                            WidgetSpec::Canvas(CanvasWidget::new(
                                TOP_VOLUME_METER_ID,
                                WidgetSizing::fixed(Vector2::new(
                                    meter_width.max(1.0),
                                    meter_height.max(1.0),
                                )),
                            )),
                            WidgetMessageMapper::None,
                        ),
                    ),
                    SurfaceChild::new(
                        fixed_slot(value_width),
                        text_widget(
                            TOP_VOLUME_VALUE_ID,
                            &content.volume_value,
                            value_width,
                            sizing.font_meta,
                        ),
                    ),
                    SurfaceChild::new(
                        fixed_slot(label_width),
                        text_widget(
                            TOP_VOLUME_LABEL_ID,
                            &content.volume_label,
                            label_width,
                            sizing.font_meta,
                        ),
                    ),
                    SurfaceChild::new(
                        SlotParams::fill(),
                        text_widget(TOP_TITLE_TEXT_ID, &content.title, 1.0, title_height),
                    ),
                ],
            ),
        )],
    )
}

fn build_action_cluster(
    content: &TopBarSurfaceContent,
    sizing: SizingTokens,
    options_width: f32,
    button_height: f32,
    update_widths: &[f32],
    visible_update_count: usize,
) -> SurfaceNode<()> {
    let mut children = Vec::with_capacity(visible_update_count + 2);
    children.push(SurfaceChild::new(
        SlotParams::fill(),
        spacer_widget(TOP_ACTION_SPACER_ID),
    ));
    let hidden_count = content
        .update_actions
        .len()
        .saturating_sub(visible_update_count);
    for (index, width) in update_widths.iter().enumerate() {
        let spec = &content.update_actions[hidden_count + index];
        children.push(SurfaceChild::new(
            fixed_slot(*width),
            SurfaceNode::widget(
                WidgetSpec::Button(ButtonWidget::new(
                    TOP_UPDATE_BUTTON_BASE_ID + (hidden_count + index) as u64,
                    spec.label,
                    WidgetSizing::fixed(Vector2::new(width.max(1.0), button_height.max(1.0))),
                )),
                WidgetMessageMapper::None,
            ),
        ));
    }
    children.push(SurfaceChild::new(
        fixed_slot(options_width),
        SurfaceNode::widget(
            WidgetSpec::Button(ButtonWidget::new(
                TOP_OPTIONS_BUTTON_ID,
                &content.options_label,
                WidgetSizing::fixed(Vector2::new(options_width.max(1.0), button_height.max(1.0))),
            )),
            WidgetMessageMapper::None,
        ),
    ));
    SurfaceNode::container(
        TOP_ACTION_CLUSTER_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                right: sizing.text_inset_x.max(3.0),
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::new(
            SlotParams::fill(),
            SurfaceNode::container(
                TOP_ACTION_ROW_ID,
                ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing: sizing.action_button_gap.max(1.0),
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Center,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        )],
    )
}

fn top_bar_action_cluster_width(viewport_width: f32, sizing: SizingTokens) -> f32 {
    let inner_width = (viewport_width - (sizing.panel_inset.max(0.0) * 2.0)).max(0.0);
    let desired = ((sizing.action_button_width * 5.0)
        + (sizing.action_button_gap * 4.0)
        + (sizing.text_inset_x * 2.0))
        .clamp(
            sizing.top_bar_action_cluster_min_width,
            sizing.top_bar_action_cluster_max_width,
        );
    desired.min((inner_width - sizing.top_bar_action_cluster_title_reserve_width).max(0.0))
}

fn top_bar_button_height(sizing: SizingTokens) -> f32 {
    (sizing.top_bar_height - (sizing.text_inset_y.max(2.0) * 2.0)).clamp(16.0, 24.0)
}

fn top_bar_options_button_width(sizing: SizingTokens, button_height: f32) -> f32 {
    (button_height * 4.0).clamp(72.0, sizing.top_bar_action_cluster_max_width.max(72.0))
}

fn visible_update_widths(
    actions: &[TopBarUpdateActionSpec],
    action_cluster_width: f32,
    options_width: f32,
    sizing: SizingTokens,
) -> Vec<f32> {
    let available_width =
        (action_cluster_width - options_width - sizing.action_button_gap.max(1.0)).max(0.0);
    let widths: Vec<f32> = actions
        .iter()
        .map(|spec| {
            (spec.label.chars().count() as f32 * (sizing.font_meta * 0.62)
                + (sizing.text_inset_x * 2.0))
                .clamp(42.0, 84.0)
        })
        .collect();
    visible_suffix_widths(&widths, available_width, sizing.action_button_gap.max(1.0))
}

fn visible_suffix_widths(widths: &[f32], available_width: f32, gap: f32) -> Vec<f32> {
    if available_width <= 0.0 || widths.is_empty() {
        return Vec::new();
    }
    let mut used = 0.0;
    let mut reversed = Vec::new();
    for (index, width) in widths.iter().rev().enumerate() {
        let candidate = used + width + if index > 0 { gap } else { 0.0 };
        if candidate >= available_width {
            break;
        }
        reversed.push(*width);
        used = candidate;
    }
    reversed.reverse();
    reversed
}

fn text_widget(id: u64, text: &str, width: f32, font_size: f32) -> SurfaceNode<()> {
    SurfaceNode::widget(
        WidgetSpec::Text(TextWidget::new(
            id,
            text,
            WidgetSizing::fixed(Vector2::new(width.max(1.0), font_size.max(1.0)))
                .with_baseline((font_size * 0.75).max(0.0)),
        )),
        WidgetMessageMapper::None,
    )
}

fn spacer_widget(id: u64) -> SurfaceNode<()> {
    SurfaceNode::widget(
        WidgetSpec::Canvas(CanvasWidget::new(
            id,
            WidgetSizing::fixed(Vector2::new(1.0, 1.0)),
        )),
        WidgetMessageMapper::None,
    )
}

fn fixed_slot(width: f32) -> SlotParams {
    let width = width.max(0.0);
    SlotParams {
        size_main: SizeModeMain::Fixed(width),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(width, width, 0.0, f32::INFINITY),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Stretch),
        allow_fixed_compress: false,
    }
}

fn fixed_slot_with_cross(width: f32, height: f32) -> SlotParams {
    let width = width.max(0.0);
    let height = height.max(0.0);
    SlotParams {
        size_main: SizeModeMain::Fixed(width),
        size_cross: SizeModeCross::Fixed(height),
        constraints: Constraints::new(width, width, height, height),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Center),
        allow_fixed_compress: false,
    }
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gui::native_shell::style::StyleTokens, widgets::WidgetKind};

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    fn content() -> TopBarSurfaceContent {
        TopBarSurfaceContent {
            title: String::from("Sempal"),
            volume_value: String::from("0.75"),
            volume_label: String::from("Vol"),
            options_label: String::from("48 kHz"),
            update_actions: vec![
                TopBarUpdateActionSpec {
                    node_slug: "open",
                    label: "Open",
                    action: UiAction::OpenUpdateLink,
                    enabled: true,
                },
                TopBarUpdateActionSpec {
                    node_slug: "install",
                    label: "Install",
                    action: UiAction::InstallUpdate,
                    enabled: true,
                },
                TopBarUpdateActionSpec {
                    node_slug: "dismiss",
                    label: "Dismiss",
                    action: UiAction::DismissUpdate,
                    enabled: true,
                },
            ],
        }
    }

    #[test]
    fn top_bar_surface_uses_public_text_button_and_canvas_widgets() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let surface = build_top_bar_surface(&content(), style.sizing, 1280.0);
        assert_eq!(
            surface
                .find_widget(TOP_TITLE_TEXT_ID)
                .expect("title")
                .widget()
                .kind(),
            WidgetKind::Text
        );
        assert_eq!(
            surface
                .find_widget(TOP_VOLUME_METER_ID)
                .expect("meter")
                .widget()
                .kind(),
            WidgetKind::Canvas
        );
        assert_eq!(
            surface
                .find_widget(TOP_OPTIONS_BUTTON_ID)
                .expect("options")
                .widget()
                .kind(),
            WidgetKind::Button
        );
    }

    #[test]
    fn top_bar_surface_layout_keeps_clusters_and_controls_inside_band() {
        for viewport_width in [820.0, 1280.0, 2300.0] {
            let style = StyleTokens::for_viewport_width(viewport_width);
            let bar = Rect::from_min_max(
                Point::new(0.0, 0.0),
                Point::new(viewport_width, style.sizing.top_bar_height),
            );
            let layout = resolve_top_bar_surface_layout(bar, style.sizing, &content());
            assert_inside(bar, layout.title_cluster);
            assert_inside(bar, layout.action_cluster);
            assert!(layout.title_cluster.max.x <= layout.action_cluster.min.x);
            assert_inside(layout.title_cluster, layout.volume_meter_rect);
            assert_inside(layout.title_cluster, layout.volume_value_rect);
            assert_inside(layout.title_cluster, layout.volume_label_rect);
            if let Some(options) = layout.options_button_rect {
                assert_inside(layout.action_cluster, options);
                for button in &layout.update_buttons {
                    assert_inside(layout.action_cluster, button.rect);
                    assert!(button.rect.max.x <= options.min.x);
                }
            }
        }
    }

    #[test]
    fn top_bar_surface_preserves_rightmost_options_button() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let bar = Rect::from_min_max(
            Point::new(0.0, 0.0),
            Point::new(1280.0, style.sizing.top_bar_height),
        );
        let layout = resolve_top_bar_surface_layout(bar, style.sizing, &content());
        let options = layout.options_button_rect.expect("options button");
        let rightmost_update = layout
            .update_buttons
            .iter()
            .map(|button| button.rect.max.x)
            .fold(layout.action_cluster.min.x, f32::max);
        assert!(options.max.x <= layout.action_cluster.max.x);
        assert!(rightmost_update <= options.min.x);
    }
}
