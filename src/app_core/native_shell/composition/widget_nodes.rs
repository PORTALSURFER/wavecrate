use crate::{
    gui::types::Vector2,
    runtime::SurfaceNode,
    widgets::{
        ButtonWidget, CanvasWidget, TextInputWidget, TextWidget, ToggleWidget, WidgetSizing,
    },
};

pub(super) fn button_node(id: u64, label: &str, width: f32, height: f32) -> SurfaceNode<()> {
    button_node_with(id, label, width, height, |_| {})
}

pub(super) fn button_node_with(
    id: u64,
    label: &str,
    width: f32,
    height: f32,
    configure: impl FnOnce(&mut ButtonWidget),
) -> SurfaceNode<()> {
    let mut widget = ButtonWidget::new(id, label, fixed_size(width, height));
    configure(&mut widget);
    SurfaceNode::static_widget(widget)
}

pub(super) fn toggle_node(id: u64, label: &str, width: f32, height: f32) -> SurfaceNode<()> {
    toggle_node_with(id, label, width, height, |_| {})
}

pub(super) fn toggle_square_node(id: u64, label: &str, side: f32) -> SurfaceNode<()> {
    toggle_node(id, label, side, side)
}

pub(super) fn toggle_node_with(
    id: u64,
    label: &str,
    width: f32,
    height: f32,
    configure: impl FnOnce(&mut ToggleWidget),
) -> SurfaceNode<()> {
    let mut widget = ToggleWidget::new(id, label, fixed_size(width, height));
    configure(&mut widget);
    SurfaceNode::static_widget(widget)
}

pub(super) fn text_input_node(
    id: u64,
    value: &str,
    placeholder: Option<&str>,
    width: f32,
    height: f32,
) -> SurfaceNode<()> {
    text_input_node_with(id, value, width, height, |widget| {
        widget.props.placeholder = placeholder
            .filter(|placeholder| !placeholder.is_empty())
            .map(str::to_string);
    })
}

pub(super) fn text_input_node_with(
    id: u64,
    value: &str,
    width: f32,
    height: f32,
    configure: impl FnOnce(&mut TextInputWidget),
) -> SurfaceNode<()> {
    let mut widget = TextInputWidget::new(id, value, fixed_size(width, height));
    configure(&mut widget);
    SurfaceNode::static_widget(widget)
}

pub(super) fn text_node(
    id: u64,
    text: &str,
    width: f32,
    height: f32,
    font_size: f32,
) -> SurfaceNode<()> {
    SurfaceNode::static_widget(TextWidget::new(
        id,
        text,
        fixed_size(width, height).with_baseline((font_size * 0.75).max(0.0)),
    ))
}

pub(super) fn canvas_node(id: u64, width: f32, height: f32) -> SurfaceNode<()> {
    SurfaceNode::static_widget(CanvasWidget::new(id, fixed_size(width, height)))
}

pub(super) fn spacer_node(id: u64) -> SurfaceNode<()> {
    canvas_node(id, 1.0, 1.0)
}

fn fixed_size(width: f32, height: f32) -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(width.max(1.0), height.max(1.0)))
}
