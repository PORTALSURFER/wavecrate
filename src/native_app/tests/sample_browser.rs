use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    prelude::IntoView,
    runtime::{Event, SurfaceFrame},
    widgets::{PointerButton, PointerModifiers, Widget, WidgetInput},
};
use std::fs;

use super::{native_app_state_with_temp_sample, native_runtime_for_tests};

const FOLDER_DROP_TARGET_FILL: Rgba8 = Rgba8::new(255, 130, 78, 220);

fn sample_hit_target(
    selected: bool,
    drag_active: bool,
    drag_source: bool,
    cached: bool,
) -> crate::native_app::test_support::SampleFileHitTarget {
    crate::native_app::test_support::SampleFileHitTarget::new(
        String::from("sample.wav"),
        selected,
        drag_active,
        drag_source,
        cached,
    )
}

fn text_center(frame: &SurfaceFrame, label: &str) -> Point {
    frame
        .paint_plan
        .text_runs()
        .find(|text| text.text.as_str() == label)
        .map(|text| text.rect.center())
        .unwrap_or_else(|| panic!("{label} should paint"))
}

mod column_headers;
mod column_reorder;
mod drag_drop;
mod row_activation;
mod rows;
mod similarity;
