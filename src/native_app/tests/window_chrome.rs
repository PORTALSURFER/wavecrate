use super::*;
use crate::native_app::app_chrome::waveform_panel::waveform_loading_visual;
use radiant::runtime::{NativeFileDrop, RuntimeBridge, SurfaceRuntime};
use std::{cell::RefCell, rc::Rc};
use winit::{dpi::PhysicalPosition, event::MouseButton};

fn waveform_rect(runtime: &NativeRuntimeForTests) -> Rect {
    *runtime
        .layout()
        .rects
        .get(&crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
        .expect("full app shell should lay out waveform widget")
}

fn assert_ratio_near(actual: Option<f32>, expected: f32) {
    let actual = actual.expect("expected waveform ratio");
    assert!(
        (actual - expected).abs() <= f32::EPSILON * 8.0,
        "expected {expected}, got {actual}"
    );
}

struct NativePointerShellHarness {
    runtime: NativeRuntimeForTests,
    last_cursor: Option<Point>,
    dpi_scale: radiant::theme::DpiScale,
}

impl NativePointerShellHarness {
    fn new(state: NativeAppState) -> Self {
        Self {
            runtime: native_runtime_for_tests(state, Vector2::new(900.0, 620.0)),
            last_cursor: None,
            dpi_scale: radiant::theme::DpiScale::ONE,
        }
    }

    fn runtime(&self) -> &NativeRuntimeForTests {
        &self.runtime
    }

    fn cursor_moved_logical(&mut self, point: Point) -> Option<u64> {
        self.last_cursor = Some(point);
        self.runtime.dispatch_event(Event::pointer_move(point))
    }

    fn cursor_moved_physical(&mut self, position: PhysicalPosition<f64>) -> Option<u64> {
        let point = Point::new(
            self.dpi_scale.physical_to_logical(position.x as f32),
            self.dpi_scale.physical_to_logical(position.y as f32),
        );
        self.cursor_moved_logical(point)
    }

    fn mouse_pressed(&mut self, button: MouseButton) -> Option<u64> {
        let position = self.last_cursor?;
        self.runtime.dispatch_event(Event::pointer_press(
            position,
            pointer_button(button)?,
            Default::default(),
        ))
    }

    fn mouse_released(&mut self, button: MouseButton) -> Option<u64> {
        let position = self.last_cursor?;
        self.runtime.dispatch_event(Event::pointer_release(
            position,
            pointer_button(button)?,
            Default::default(),
        ))
    }
}

fn pointer_button(button: MouseButton) -> Option<PointerButton> {
    Some(match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Auxiliary,
        _ => return None,
    })
}

mod app_bridge;
mod audio_settings;
mod browser_labels;
mod hit_targets;
mod loading_visual;
mod modal_blocking;
mod primary_waveform;
mod secondary_waveform;
