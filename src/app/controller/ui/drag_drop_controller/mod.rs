//! Deprecated compatibility-controller drag/drop logic.
//!
//! The shipping Wavecrate desktop GUI enters through `src/native_app.rs`; folder
//! browser drag/drop bugs in the current UI should start in
//! `src/native_app/folder_browser/**`. Keep this module for compatibility-model
//! behavior and legacy/ui-projection test coverage unless a task explicitly
//! targets the `src/app/**` controller layer.

mod actions;
mod delegates;
mod drag_effects;
mod drag_state;
mod drag_transitions;
mod label_formatting;
mod path_resolution;

pub(crate) use actions::DragDropActions;
pub(crate) use drag_state::DragDropController;

use super::*;
use crate::app::controller::library::wav_io::file_metadata;
use crate::app::state::{DragPayload, DragTarget};
use tracing::{debug, info};
