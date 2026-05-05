//! Generic waveform-toolbar surface projection for the native-shell compat path.
//!
//! This keeps the compact waveform control strip on the same public
//! `radiant::layout`, `radiant::runtime`, and `radiant::widgets` hosting
//! pattern used by the earlier chrome migrations while waveform plot rendering,
//! overlays, and edit geometry stay on the compatibility renderer.

use super::style::SizingTokens;
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::NodeId,
    layout::{
        Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, OverflowPolicy,
        SizeModeCross, SizeModeMain, SlotParams, layout_tree,
    },
    runtime::{SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{ButtonWidget, TextInputWidget, ToggleWidget, WidgetSizing, WidgetSpec},
};

const WAVEFORM_TOOLBAR_BASE_ID: u64 = 1320;

/// Public widget primitive used for one waveform-toolbar control slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaveformToolbarSurfaceItemKind {
    /// Momentary action such as transport, compare, or cleanup.
    Button,
    /// Stateful toggle such as loop, grid, or normalize.
    Toggle,
    /// BPM text-entry affordance hosted as a text input.
    TextInput,
}

/// User-facing control metadata projected into the generic waveform toolbar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WaveformToolbarSurfaceItem {
    /// Stable control label used by tests and host-side mapping.
    pub label: String,
    /// Control family that chooses the public widget primitive.
    pub kind: WaveformToolbarSurfaceItemKind,
    /// Displayed field value for text-input content.
    pub value: Option<String>,
    /// Whether the control is currently interactable.
    pub enabled: bool,
    /// Whether the control is visually active/on.
    pub active: bool,
}

/// Ordered waveform-toolbar content projected into the generic surface.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WaveformToolbarSurfaceContent {
    /// Toolbar items in left-to-right logical order.
    pub items: Vec<WaveformToolbarSurfaceItem>,
}

/// Resolved widget bounds for the generic waveform-toolbar surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct WaveformToolbarSurfaceLayout {
    /// Control bounds in the same order as [`WaveformToolbarSurfaceContent::items`].
    ///
    /// Leading items that do not fit in compact widths resolve to empty rects.
    pub item_rects: Vec<Rect>,
}

/// Resolve the generic waveform-toolbar surface layout inside the waveform header band.
pub(crate) fn resolve_waveform_toolbar_surface_layout(
    header_rect: Rect,
    sizing: SizingTokens,
    content: &WaveformToolbarSurfaceContent,
) -> WaveformToolbarSurfaceLayout {
    if content.items.is_empty() || header_rect.width() <= 0.0 || header_rect.height() <= 0.0 {
        return WaveformToolbarSurfaceLayout::default();
    }
    let empty = Rect::from_min_max(header_rect.min, header_rect.min);
    let legacy_rects = legacy_toolbar_rects(header_rect, sizing, content);
    let mut item_rects = vec![empty; content.items.len()];
    let visible_start = content.items.len().saturating_sub(legacy_rects.len());
    if legacy_rects.is_empty() {
        return WaveformToolbarSurfaceLayout { item_rects };
    }
    let surface = build_waveform_toolbar_surface(header_rect, content, &legacy_rects);
    let output = layout_tree(&surface.layout_node(), header_rect);
    for item_index in visible_start..content.items.len() {
        let id = waveform_toolbar_widget_id(item_index);
        item_rects[item_index] =
            clamp_rect_to_bounds(rect_for(&output.rects, id, empty), header_rect);
    }
    WaveformToolbarSurfaceLayout { item_rects }
}

fn build_waveform_toolbar_surface(
    header_rect: Rect,
    content: &WaveformToolbarSurfaceContent,
    legacy_rects: &[Rect],
) -> UiSurface<()> {
    let visible_start = content.items.len().saturating_sub(legacy_rects.len());
    let visible_items = &content.items[visible_start..];
    UiSurface::new(SurfaceNode::container(
        WAVEFORM_TOOLBAR_BASE_ID,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 0.0,
            align_cross: CrossAlign::Start,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        visible_items
            .iter()
            .zip(legacy_rects.iter().copied())
            .enumerate()
            .map(|(visible_index, (item, rect))| {
                SurfaceChild::new(
                    slot_for_rect(
                        header_rect,
                        legacy_rects,
                        rect,
                        visible_index,
                        visible_items.len(),
                    ),
                    widget_for_item(visible_start + visible_index, item, rect),
                )
            })
            .collect(),
    ))
}

fn slot_for_rect(
    header_rect: Rect,
    legacy_rects: &[Rect],
    rect: Rect,
    visible_index: usize,
    visible_len: usize,
) -> SlotParams {
    let previous_max_x = if visible_index == 0 {
        header_rect.min.x
    } else {
        legacy_rects[visible_index - 1].max.x
    };
    let right_gap = if visible_index + 1 == visible_len {
        (header_rect.max.x - rect.max.x).max(0.0)
    } else {
        0.0
    };
    SlotParams {
        size_main: SizeModeMain::Fixed(rect.width().max(1.0)),
        size_cross: SizeModeCross::Fixed(rect.height().max(1.0)),
        constraints: Constraints::new(
            rect.width().max(1.0),
            rect.width().max(1.0),
            rect.height().max(1.0),
            rect.height().max(1.0),
        ),
        margin: Insets {
            left: (rect.min.x - previous_max_x).max(0.0),
            right: right_gap,
            top: (rect.min.y - header_rect.min.y).max(0.0),
            bottom: (header_rect.max.y - rect.max.y).max(0.0),
        },
        align_cross_override: Some(CrossAlign::Start),
        allow_fixed_compress: false,
    }
}

fn widget_for_item(
    item_index: usize,
    item: &WaveformToolbarSurfaceItem,
    rect: Rect,
) -> SurfaceNode<()> {
    let id = waveform_toolbar_widget_id(item_index);
    let size = WidgetSizing::fixed(Vector2::new(rect.width().max(1.0), rect.height().max(1.0)));
    let widget = match item.kind {
        WaveformToolbarSurfaceItemKind::Button => {
            let mut widget = ButtonWidget::new(id, &item.label, size);
            widget.common.state.disabled = !item.enabled;
            widget.common.state.active = item.active;
            WidgetSpec::Button(widget)
        }
        WaveformToolbarSurfaceItemKind::Toggle => {
            let mut widget = ToggleWidget::new(id, &item.label, size);
            widget.common.state.disabled = !item.enabled;
            widget.common.state.active = item.active;
            widget.state.checked = item.active;
            WidgetSpec::Toggle(widget)
        }
        WaveformToolbarSurfaceItemKind::TextInput => {
            let mut widget = TextInputWidget::new(id, item.value.clone().unwrap_or_default(), size);
            widget.common.state.disabled = !item.enabled;
            widget.common.state.active = item.active;
            widget.common.state.read_only = true;
            widget.props.placeholder = Some(item.label.clone());
            WidgetSpec::TextInput(widget)
        }
    };
    SurfaceNode::widget(widget, WidgetMessageMapper::None)
}

fn legacy_toolbar_rects(
    header_rect: Rect,
    sizing: SizingTokens,
    content: &WaveformToolbarSurfaceContent,
) -> Vec<Rect> {
    let labels: Vec<String> = content
        .items
        .iter()
        .map(waveform_toolbar_layout_label)
        .collect();
    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    let cluster = waveform_toolbar_cluster_rect(header_rect);
    super::layout_adapter::compute_update_action_button_rects(
        header_rect,
        cluster,
        sizing,
        &label_refs,
    )
}

fn waveform_toolbar_cluster_rect(header_rect: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(
            header_rect.min.x + (header_rect.width() * 0.32),
            header_rect.min.y,
        ),
        header_rect.max,
    )
}

fn waveform_toolbar_layout_label(item: &WaveformToolbarSurfaceItem) -> String {
    if item.kind == WaveformToolbarSurfaceItemKind::TextInput {
        return item.value.clone().unwrap_or_else(|| String::from("120.0"));
    }
    String::from("Mono")
}

fn waveform_toolbar_widget_id(index: usize) -> NodeId {
    WAVEFORM_TOOLBAR_BASE_ID + index as u64 + 1
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gui::native_shell::style::StyleTokens, widgets::WidgetKind};

    fn sample_content() -> WaveformToolbarSurfaceContent {
        WaveformToolbarSurfaceContent {
            items: vec![
                WaveformToolbarSurfaceItem {
                    label: String::from("Channel"),
                    kind: WaveformToolbarSurfaceItemKind::Toggle,
                    value: None,
                    enabled: true,
                    active: false,
                },
                WaveformToolbarSurfaceItem {
                    label: String::from("Norm"),
                    kind: WaveformToolbarSurfaceItemKind::Toggle,
                    value: None,
                    enabled: true,
                    active: true,
                },
                WaveformToolbarSurfaceItem {
                    label: String::from("BPM Value"),
                    kind: WaveformToolbarSurfaceItemKind::TextInput,
                    value: Some(String::from("128.0")),
                    enabled: true,
                    active: false,
                },
                WaveformToolbarSurfaceItem {
                    label: String::from("Loop"),
                    kind: WaveformToolbarSurfaceItemKind::Toggle,
                    value: None,
                    enabled: true,
                    active: true,
                },
                WaveformToolbarSurfaceItem {
                    label: String::from("Compare"),
                    kind: WaveformToolbarSurfaceItemKind::Button,
                    value: None,
                    enabled: true,
                    active: false,
                },
                WaveformToolbarSurfaceItem {
                    label: String::from("Play"),
                    kind: WaveformToolbarSurfaceItemKind::Button,
                    value: None,
                    enabled: true,
                    active: false,
                },
            ],
        }
    }

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn waveform_toolbar_surface_uses_public_toggle_button_and_text_input_widgets() {
        let header_rect = Rect::from_min_max(Point::new(220.0, 32.0), Point::new(1260.0, 64.0));
        let content = sample_content();
        let surface = build_waveform_toolbar_surface(
            header_rect,
            &content,
            &legacy_toolbar_rects(
                header_rect,
                StyleTokens::for_viewport_width(1280.0).sizing,
                &content,
            ),
        );
        assert_eq!(
            surface
                .find_widget(waveform_toolbar_widget_id(0))
                .expect("channel toggle")
                .widget()
                .kind(),
            WidgetKind::Toggle
        );
        assert_eq!(
            surface
                .find_widget(waveform_toolbar_widget_id(2))
                .expect("bpm value input")
                .widget()
                .kind(),
            WidgetKind::TextInput
        );
        assert_eq!(
            surface
                .find_widget(waveform_toolbar_widget_id(4))
                .expect("compare button")
                .widget()
                .kind(),
            WidgetKind::Button
        );
    }

    #[test]
    fn waveform_toolbar_surface_layout_preserves_control_order_inside_header() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let header_rect = Rect::from_min_max(Point::new(220.0, 32.0), Point::new(1260.0, 64.0));
        let layout =
            resolve_waveform_toolbar_surface_layout(header_rect, style.sizing, &sample_content());
        for rect in layout
            .item_rects
            .iter()
            .copied()
            .filter(|rect| rect.width() > 1.0)
        {
            assert_inside(header_rect, rect);
        }
        assert!(layout.item_rects[0].max.x <= layout.item_rects[1].min.x);
        assert!(layout.item_rects[1].max.x <= layout.item_rects[2].min.x);
        assert!(layout.item_rects[2].max.x <= layout.item_rects[3].min.x);
        assert!(layout.item_rects[3].max.x <= layout.item_rects[4].min.x);
        assert!(layout.item_rects[4].max.x <= layout.item_rects[5].min.x);
    }
}
