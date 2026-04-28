//! Static-frame segment routing and waveform overlay emit helpers.

use super::*;

mod fades;
mod header;
mod overlay;
mod routing;
mod scrollbar;
mod selection;
mod slices;
mod surface;
mod trail;

use self::{
    fades::emit_edit_fade_overlays,
    selection::{
        emit_hovered_edit_resize_edge, emit_hovered_selection_resize_edge,
        emit_selection_drag_handle, emit_selection_shift_handle, emit_waveform_loop_bar,
    },
    slices::emit_waveform_slice_previews,
};
pub(in crate::gui::native_shell::state) use self::{
    header::push_waveform_header_overlay,
    overlay::push_waveform_playhead_overlay,
    routing::{static_segment_for_primitive, static_segment_for_text, static_segment_matches},
    scrollbar::{waveform_scrollbar_center_for_pointer, waveform_scrollbar_layout},
    surface::push_waveform_image,
    trail::{PlayheadTrailLine, playhead_marker_rect},
};
