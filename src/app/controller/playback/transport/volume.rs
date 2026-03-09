use super::*;
use std::time::{Duration, Instant};

/// Debounce window before persisting live volume slider updates.
const VOLUME_PERSIST_DEBOUNCE: Duration = Duration::from_millis(120);

/// Apply volume immediately and mark the setting dirty for deferred persistence.
pub(crate) fn set_volume_live(controller: &mut AppController, volume: f32) {
    let previous_milli = volume_to_milli(controller.ui.volume);
    controller.apply_volume(volume);
    let current_milli = volume_to_milli(controller.ui.volume);
    if previous_milli == current_milli {
        return;
    }
    if controller.runtime.last_persisted_volume_milli == Some(current_milli) {
        controller.runtime.volume_persist_dirty = false;
        controller.runtime.volume_persist_deadline = None;
        return;
    }
    controller.runtime.volume_persist_dirty = true;
    controller.runtime.volume_persist_deadline = Some(Instant::now() + VOLUME_PERSIST_DEBOUNCE);
}

/// Persist a dirty volume setting immediately.
pub(crate) fn commit_volume_setting(controller: &mut AppController) {
    if !controller.runtime.volume_persist_dirty {
        return;
    }
    let current_milli = volume_to_milli(controller.ui.volume);
    if controller.runtime.last_persisted_volume_milli == Some(current_milli) {
        controller.runtime.volume_persist_dirty = false;
        controller.runtime.volume_persist_deadline = None;
        return;
    }
    if let Err(err) = controller.persist_config("Failed to save volume") {
        controller.set_status(err, StatusTone::Error);
        controller.runtime.volume_persist_deadline = Some(Instant::now() + VOLUME_PERSIST_DEBOUNCE);
        return;
    }
    controller.runtime.volume_persist_dirty = false;
    controller.runtime.volume_persist_deadline = None;
    controller.runtime.last_persisted_volume_milli = Some(current_milli);
}

/// Flush deferred volume persistence once the debounce deadline elapses.
pub(crate) fn flush_pending_volume_setting(controller: &mut AppController) {
    if !controller.runtime.volume_persist_dirty {
        return;
    }
    let Some(deadline) = controller.runtime.volume_persist_deadline else {
        return;
    };
    if Instant::now() >= deadline {
        commit_volume_setting(controller);
    }
}

/// Convert normalized volume into stable milli units for equality checks.
fn volume_to_milli(volume: f32) -> u16 {
    (volume.clamp(0.0, 1.0) * 1000.0).round() as u16
}
