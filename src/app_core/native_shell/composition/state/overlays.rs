//! Overlay geometry helpers and focused overlay paint builders for the native shell state.

use super::*;

#[path = "overlays/drag.rs"]
mod drag;
#[path = "overlays/geometry.rs"]
mod geometry;
#[path = "overlays/progress.rs"]
mod progress;
#[path = "overlays/prompt.rs"]
mod prompt;

pub(in crate::gui::native_shell::state) use self::{drag::*, geometry::*, progress::*, prompt::*};
