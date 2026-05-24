use super::*;
use crate::app_core::native_shell::runtime_contract::NormalizedRangeModel;

#[path = "fades/curve.rs"]
mod curve;
#[path = "fades/handles.rs"]
mod handles;
#[path = "fades/model.rs"]
mod model;
#[path = "fades/shading.rs"]
mod shading;

use self::{handles::emit_fade_handles, model::EditFadePositions, shading::emit_fade_shading};

pub(super) use self::model::{
    EditFadeOverlayGeometry, EditFadeSelection, EditFadeSide, EditFadeTime,
};

/// Width in logical pixels for edit-fade drag handles.
pub(super) const EDIT_FADE_HANDLE_WIDTH: f32 = 3.0;
/// Width/height in logical pixels for square edit-fade grab tabs.
pub(super) const EDIT_FADE_HANDLE_TAB_SIZE: f32 = 10.0;

/// Emit edit-fade shading and draggable handle markers for the active edit selection.
pub(super) fn emit_edit_fade_overlays(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    geometry: EditFadeOverlayGeometry,
    selection: EditFadeSelection,
    accent_blue: Rgba8,
) {
    if selection.is_empty() || geometry.waveform_plot.width() <= 0.0 {
        return;
    }

    let positions = EditFadePositions::resolve(selection, geometry);
    emit_fade_shading(primitives, style, geometry, positions, accent_blue);
    emit_fade_handles(primitives, style, geometry, positions, accent_blue);
}
