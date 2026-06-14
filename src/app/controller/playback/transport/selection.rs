mod bpm;
mod drag;
mod edit;
mod retarget;
mod snapping;

#[cfg(test)]
use super::*;

pub(crate) use bpm::scaled_selection_bpm;
pub(crate) use drag::{
    cancel_click_armed_selection_drag, clear_selection, finish_selection_drag,
    is_selection_dragging, set_selection_range, set_selection_range_with_smart_scale,
    start_selection_drag, start_selection_edge_drag, update_selection_drag,
};
pub(crate) use edit::{
    clear_edit_selection, finish_edit_selection_drag, is_edit_selection_dragging,
    set_edit_selection_range, start_edit_selection_drag, update_edit_selection_drag,
};

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
