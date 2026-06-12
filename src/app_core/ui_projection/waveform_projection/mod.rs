//! Waveform panel and waveform chrome projection helpers.

use super::*;

mod channel_view;
mod chrome;
mod fade_overlay;
mod image;
mod panel;
mod selection;
mod units;

pub(super) use channel_view::project_waveform_channel_view_model;
pub(crate) use chrome::project_waveform_chrome_model;
pub(super) use chrome::waveform_transport_hint;
pub(super) use fade_overlay::project_waveform_edit_fade_overlay_model;
pub(crate) use image::effective_waveform_image_signature;
pub(crate) use panel::project_waveform_model;
#[cfg(test)]
pub(super) use panel::resolve_projected_playhead_ratio;
pub(super) use panel::{project_waveform_target_label, projected_playhead_ratio};
pub(super) use selection::{
    project_waveform_edit_selection_milli, project_waveform_slice_previews,
};
pub(super) use units::{
    normalized_to_micros, normalized_to_milli, normalized64_to_micros, normalized64_to_milli,
    normalized64_to_nanos,
};
