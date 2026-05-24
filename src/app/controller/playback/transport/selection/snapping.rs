use super::*;

const TRANSIENT_SNAP_RADIUS: f32 = 0.01;

pub(super) fn snap_to_transient(controller: &AppController, position: f32) -> Option<f32> {
    if !controller.ui.waveform.transient_markers_enabled
        || !controller.ui.waveform.transient_snap_enabled
    {
        return None;
    }
    let mut closest = None;
    let mut best_distance = TRANSIENT_SNAP_RADIUS;
    for marker in controller.ui.waveform.transients.iter().copied() {
        let distance = (marker - position).abs();
        if distance <= best_distance {
            best_distance = distance;
            closest = Some(marker);
        }
    }
    closest
}
