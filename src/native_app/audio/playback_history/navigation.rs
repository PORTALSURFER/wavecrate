use std::{path::Path, time::Instant};

use radiant::prelude as ui;

use crate::native_app::app::{
    GuiMessage, NativeAppState, PendingSamplePlayback, emit_gui_action, sample_path_label,
};

const PLAYBACK_NAVIGATION_HISTORY_LIMIT: usize = 50;

#[derive(Clone, Debug, PartialEq)]
struct PlaybackHistoryEntry {
    path: String,
    start_ratio: f32,
    end_ratio: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::native_app) struct PlaybackNavigationHistory {
    entries: Vec<PlaybackHistoryEntry>,
    cursor: Option<usize>,
}

impl PlaybackHistoryEntry {
    fn new(path: String, start_ratio: f32, end_ratio: f32) -> Self {
        let start_ratio = normalized_history_ratio(start_ratio);
        let end_ratio = normalized_history_ratio(end_ratio);
        let (start_ratio, end_ratio) = if end_ratio < start_ratio {
            (end_ratio, start_ratio)
        } else {
            (start_ratio, end_ratio)
        };
        Self {
            path,
            start_ratio,
            end_ratio,
        }
    }
}

impl PlaybackNavigationHistory {
    pub(in crate::native_app) fn record(&mut self, path: String, start_ratio: f32, end_ratio: f32) {
        let entry = PlaybackHistoryEntry::new(path, start_ratio, end_ratio);
        if let Some(cursor) = self.cursor
            && cursor + 1 < self.entries.len()
        {
            self.entries.truncate(cursor + 1);
        }
        if self.entries.last() == Some(&entry) {
            self.cursor = self.entries.len().checked_sub(1);
            return;
        }
        self.entries.push(entry);
        if self.entries.len() > PLAYBACK_NAVIGATION_HISTORY_LIMIT {
            let overflow = self.entries.len() - PLAYBACK_NAVIGATION_HISTORY_LIMIT;
            self.entries.drain(0..overflow);
        }
        self.cursor = self.entries.len().checked_sub(1);
    }

    fn back(&mut self) -> Option<PlaybackHistoryEntry> {
        if self.entries.len() < 2 {
            return None;
        }
        let current = self.cursor.unwrap_or(self.entries.len() - 1);
        if current == 0 {
            self.cursor = Some(0);
            return None;
        }
        let previous = current - 1;
        self.cursor = Some(previous);
        self.entries.get(previous).cloned()
    }

    fn forward(&mut self) -> Option<PlaybackHistoryEntry> {
        let current = self.cursor?;
        let next = current + 1;
        if next >= self.entries.len() {
            return None;
        }
        self.cursor = Some(next);
        self.entries.get(next).cloned()
    }

    #[cfg(test)]
    pub(in crate::native_app) fn len(&self) -> usize {
        self.entries.len()
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn record_current_playback_history(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
    ) {
        if !self.waveform.current.has_loaded_sample() {
            return;
        }
        self.audio.playback_history.record(
            self.waveform.current.path().display().to_string(),
            start_ratio,
            end_ratio,
        );
    }

    pub(in crate::native_app) fn play_previous_playback_history(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let Some(entry) = self.audio.playback_history.back() else {
            self.ui.status.sample = String::from("No earlier playback history");
            emit_gui_action(
                "playback.history.previous",
                Some("transport"),
                None,
                "empty",
                started_at,
                None,
            );
            return;
        };
        self.play_playback_history_entry(entry, "playback.history.previous", started_at, context);
    }

    pub(in crate::native_app) fn play_next_playback_history(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let Some(entry) = self.audio.playback_history.forward() else {
            self.ui.status.sample = String::from("No later playback history");
            emit_gui_action(
                "playback.history.next",
                Some("transport"),
                None,
                "empty",
                started_at,
                None,
            );
            return;
        };
        self.play_playback_history_entry(entry, "playback.history.next", started_at, context);
    }

    fn play_playback_history_entry(
        &mut self,
        entry: PlaybackHistoryEntry,
        action: &'static str,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let label = sample_path_label(entry.path.as_str());
        if self.waveform.current.has_loaded_sample()
            && self.waveform.current.path() == Path::new(&entry.path)
        {
            self.start_loaded_playback_history_entry(entry, &label, action, started_at, context);
            return;
        }

        self.audio.pending_sample_playback = Some(PendingSamplePlayback::ReplayHistory {
            start: entry.start_ratio,
            end: entry.end_ratio,
        });
        if self
            .library
            .folder_browser
            .focus_file_across_sources(Path::new(&entry.path))
        {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
        }
        self.load_sample_without_autoplay(entry.path, context);
        self.ui.status.sample = format!("Loading {label} from playback history");
        emit_gui_action(
            action,
            Some("transport"),
            Some(&label),
            "load_queued",
            started_at,
            None,
        );
    }

    fn start_loaded_playback_history_entry(
        &mut self,
        entry: PlaybackHistoryEntry,
        label: &str,
        action: &'static str,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match self.start_playback_fixed_span_without_history(entry.start_ratio, entry.end_ratio) {
            Ok(()) => {
                self.record_sample_last_played(entry.path, context);
                self.ui.status.sample = format!("Playing {label} from history");
                emit_gui_action(
                    action,
                    Some("transport"),
                    Some(label),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.ui.status.sample = format!("Playback unavailable: {err}");
                emit_gui_action(
                    action,
                    Some("transport"),
                    Some(label),
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }
}

fn normalized_history_ratio(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{PlaybackHistoryEntry, PlaybackNavigationHistory};

    #[test]
    fn history_steps_back_and_forward_without_recording_replays() {
        let mut history = PlaybackNavigationHistory::default();
        history.record(String::from("kick.wav"), 0.0, 1.0);
        history.record(String::from("snare.wav"), 0.2, 0.5);

        assert_eq!(history.back(), Some(entry("kick.wav", 0.0, 1.0)));
        assert_eq!(history.forward(), Some(entry("snare.wav", 0.2, 0.5)));
        assert_eq!(history.forward(), None);
    }

    #[test]
    fn recording_after_back_discards_forward_history() {
        let mut history = PlaybackNavigationHistory::default();
        history.record(String::from("a.wav"), 0.0, 1.0);
        history.record(String::from("b.wav"), 0.0, 1.0);
        history.record(String::from("c.wav"), 0.0, 1.0);

        assert_eq!(history.back(), Some(entry("b.wav", 0.0, 1.0)));
        history.record(String::from("d.wav"), 0.25, 0.75);

        assert_eq!(history.forward(), None);
        assert_eq!(history.back(), Some(entry("b.wav", 0.0, 1.0)));
    }

    #[test]
    fn history_keeps_last_fifty_entries() {
        let mut history = PlaybackNavigationHistory::default();
        for index in 0..55 {
            history.record(format!("sample-{index}.wav"), 0.0, 1.0);
        }

        assert_eq!(history.len(), 50);
        for _ in 0..49 {
            let _ = history.back();
        }
        assert_eq!(history.back(), None);
        assert_eq!(history.forward(), Some(entry("sample-6.wav", 0.0, 1.0)));
    }

    #[test]
    fn duplicate_contiguous_entries_are_collapsed() {
        let mut history = PlaybackNavigationHistory::default();
        history.record(String::from("loop.wav"), 0.1, 0.9);
        history.record(String::from("loop.wav"), 0.1, 0.9);

        assert_eq!(history.len(), 1);
        assert_eq!(history.back(), None);
    }

    fn entry(path: impl Into<String>, start: f32, end: f32) -> PlaybackHistoryEntry {
        PlaybackHistoryEntry::new(path.into(), start, end)
    }
}
