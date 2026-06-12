use std::time::{Duration, Instant};

use crate::native_app::app::{NativeAppState, emit_gui_action};

pub(in crate::native_app) const VOLUME_PERSIST_DEBOUNCE: Duration = Duration::from_millis(250);

impl NativeAppState {
    pub(in crate::native_app) fn set_volume(&mut self, volume: f32) {
        let started_at = Instant::now();
        let previous = volume_milli(self.audio.volume);
        self.audio.volume = volume.clamp(0.0, 1.0);
        if let Some(player) = self.audio.player.as_mut() {
            player.set_volume(self.audio.volume);
        }
        if volume_milli(self.audio.volume) == previous {
            return;
        }
        self.audio.volume_persist_deadline = Some(started_at + VOLUME_PERSIST_DEBOUNCE);
    }

    pub(in crate::native_app) fn flush_pending_volume_persist(&mut self) {
        let Some(deadline) = self.audio.volume_persist_deadline else {
            return;
        };
        if Instant::now() < deadline {
            return;
        }
        let started_at = Instant::now();
        self.persist_user_configuration("playback.volume.persist", started_at);
        if self.audio.volume_persist_deadline.is_none() {
            emit_gui_action(
                "playback.volume.set",
                Some("transport"),
                None,
                "success",
                started_at,
                None,
            );
        }
    }
}

fn volume_milli(volume: f32) -> u16 {
    (volume.clamp(0.0, 1.0) * 1000.0).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_milli_clamps_and_rounds_to_persist_threshold() {
        assert_eq!(volume_milli(-1.0), 0);
        assert_eq!(volume_milli(0.0004), 0);
        assert_eq!(volume_milli(0.0005), 1);
        assert_eq!(volume_milli(0.5), 500);
        assert_eq!(volume_milli(2.0), 1000);
    }
}
