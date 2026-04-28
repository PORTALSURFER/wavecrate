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

pub(in crate::gui::native_shell::state) use self::{
    hit_testing::*, projection::*, scrollbars::*, viewport::*,
};
