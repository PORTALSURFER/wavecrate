use super::*;
use crate::app::controller::jobs::{ConfigPersistJob, ConfigPersistResult, JobMessage};
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
    let request_id = controller.runtime.jobs.next_config_persist_request_id();
    let volume = controller.ui.volume.clamp(0.0, 1.0);
    controller.runtime.pending_config_persist =
        Some(crate::app::controller::state::runtime::PendingConfigPersist {
            request_id,
            volume,
            queued_at: Instant::now(),
        });
    controller.runtime.jobs.spawn_one_shot_job(
        true,
        move || {
            let started_at = Instant::now();
                let result = crate::sample_sources::config::load_or_default()
                .map_err(|err| err.to_string())
                .and_then(|mut config| {
                    config.core.volume = volume;
                    crate::sample_sources::config::save(&config).map_err(|err| err.to_string())
                });
            ConfigPersistResult {
                request_id,
                job: ConfigPersistJob::SaveVolume { request_id, volume },
                elapsed: started_at.elapsed(),
                result,
            }
        },
        JobMessage::ConfigPersistFinished,
    );
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
