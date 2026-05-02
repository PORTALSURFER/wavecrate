//! Compatibility shell snapshot capture used by host-owned GUI fixtures.

use super::AppModel;
use crate::gui::{
    native_shell::{NativeShellState, ShellLayout, ShellLayoutRuntime, StyleTokens},
    paint::{PaintFrame as NativeViewFrame, Primitive, TextAlign},
    snapshot::{
        SnapshotColor, SnapshotPoint, SnapshotPrimitive, SnapshotRect, SnapshotTextAlign,
        SnapshotTextRun, VisualSnapshot,
    },
    types::{Point, Rect, Rgba8, Vector2},
};

/// Compatibility alias for generic visual snapshots captured from the legacy shell.
pub type NativeShellShotSnapshot = VisualSnapshot;

/// Capture a deterministic native-shell visual snapshot without launching a window.
pub fn capture_native_shell_shot_snapshot(
    name: impl Into<String>,
    viewport: [f32; 2],
    model: &AppModel,
) -> NativeShellShotSnapshot {
    let viewport = Vector2::new(viewport[0].max(1.0), viewport[1].max(1.0));
    let style = StyleTokens::for_viewport_width(viewport.x);
    let mut runtime = ShellLayoutRuntime::default();
    let layout = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);
    let mut state = NativeShellState::new();
    state.sync_from_model(model);
    let mut frame = NativeViewFrame {
        clear_color: style.clear_color,
        primitives: Vec::new(),
        text_runs: Vec::new(),
    };
    state.build_frame_with_style_into_static(&layout, &style, model, &mut frame);
    snapshot_from_frame(name.into(), &layout, &frame)
}

fn snapshot_from_frame(
    name: String,
    layout: &ShellLayout,
    frame: &NativeViewFrame,
) -> NativeShellShotSnapshot {
    let viewport_width =
        u32::try_from(layout.root.rect.width().round().max(1.0) as i64).unwrap_or(1);
    let viewport_height =
        u32::try_from(layout.root.rect.height().round().max(1.0) as i64).unwrap_or(1);
    let primitives = frame.primitives.iter().map(snap_primitive).collect();
    let text_runs = frame
        .text_runs
        .iter()
        .map(|run| SnapshotTextRun {
            text: run.text.clone(),
            position: snap_point(run.position),
            font_size: quantize(run.font_size),
            color: snap_color(run.color),
            max_width: run.max_width.map(quantize),
            align: snap_align(run.align),
        })
        .collect();

    VisualSnapshot {
        name,
        viewport_width,
        viewport_height,
        clear_color: snap_color(frame.clear_color),
        primitive_count: frame.primitives.len(),
        text_run_count: frame.text_runs.len(),
        primitives,
        text_runs,
    }
}

fn snap_primitive(primitive: &Primitive) -> SnapshotPrimitive {
    match primitive {
        Primitive::Rect(fill_rect) => SnapshotPrimitive::Rect {
            rect: snap_rect(fill_rect.rect),
            color: snap_color(fill_rect.color),
        },
        Primitive::Circle(fill_circle) => SnapshotPrimitive::Circle {
            center: snap_point(fill_circle.center),
            radius: quantize(fill_circle.radius),
            color: snap_color(fill_circle.color),
        },
        Primitive::LinearGradient(fill_gradient) => SnapshotPrimitive::LinearGradient {
            rect: snap_rect(fill_gradient.rect),
            start: snap_point(fill_gradient.start),
            end: snap_point(fill_gradient.end),
            start_color: snap_color(fill_gradient.start_color),
            end_color: snap_color(fill_gradient.end_color),
        },
        Primitive::Image(draw_image) => SnapshotPrimitive::Image {
            rect: snap_rect(draw_image.rect),
            width: u32::try_from(draw_image.image.width).unwrap_or(0),
            height: u32::try_from(draw_image.image.height).unwrap_or(0),
            pixels: draw_image.image.pixels.as_ref().to_vec(),
        },
    }
}

fn quantize(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

fn snap_color(color: Rgba8) -> SnapshotColor {
    SnapshotColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn snap_point(point: Point) -> SnapshotPoint {
    SnapshotPoint {
        x: quantize(point.x),
        y: quantize(point.y),
    }
}

fn snap_rect(rect: Rect) -> SnapshotRect {
    SnapshotRect {
        x: quantize(rect.min.x),
        y: quantize(rect.min.y),
        width: quantize(rect.width()),
        height: quantize(rect.height()),
    }
}

fn snap_align(align: TextAlign) -> SnapshotTextAlign {
    match align {
        TextAlign::Left => SnapshotTextAlign::Left,
        TextAlign::Center => SnapshotTextAlign::Center,
        TextAlign::Right => SnapshotTextAlign::Right,
    }
}
