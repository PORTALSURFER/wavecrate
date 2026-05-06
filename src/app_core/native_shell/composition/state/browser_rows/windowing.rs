//! Browser-row viewport, scrollbar, and rendered-window helpers.

use super::*;

#[path = "windowing/hit_testing.rs"]
mod hit_testing;
#[path = "windowing/projection.rs"]
mod projection;
#[path = "windowing/scrollbars.rs"]
mod scrollbars;
#[path = "windowing/viewport.rs"]
mod viewport;

pub(in crate::app_core::native_shell::composition::state) use self::{
    hit_testing::*, projection::*, scrollbars::*, viewport::*,
};
