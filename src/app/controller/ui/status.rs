//! UI status message helpers owned by the controller.

use super::*;

impl AppController {
    pub(crate) fn set_status(&mut self, text: impl Into<String>, tone: StatusTone) {
        let text = text.into();
        let status_changed = self.ui.status.text != text || self.ui.status.status_tone != tone;
        self.ui.status.text = text.clone();
        self.ui.status.status_tone = tone;
        if status_changed {
            self.mark_status_projection_revision_dirty();
        }
        let entry = format!("[{}] {}", status_prefix(tone), text);
        if self.ui.status.log.last().is_some_and(|last| last == &entry) {
            return;
        }
        self.ui.status.log.push(entry);
        if self.ui.status.log.len() > STATUS_LOG_LIMIT {
            let overflow = self.ui.status.log.len() - STATUS_LOG_LIMIT;
            self.ui.status.log.drain(0..overflow);
        }
        log_status_entry(tone, self.ui.status.log.last().expect("just pushed"));
    }

    pub(crate) fn set_error_status(&mut self, text: impl Into<String>) {
        self.set_status(text, StatusTone::Error);
    }

    pub(crate) fn set_status_message(&mut self, message: StatusMessage) {
        let (text, tone) = message.into_text_and_tone();
        self.set_status(text, tone);
    }
}

fn log_status_entry(tone: StatusTone, entry: &str) {
    match tone {
        StatusTone::Warning => tracing::warn!("{entry}"),
        StatusTone::Error => tracing::error!("{entry}"),
        StatusTone::Info | StatusTone::Busy | StatusTone::Idle => tracing::info!("{entry}"),
    }
}

/// Return the status badge prefix text for a status tone.
fn status_prefix(tone: StatusTone) -> &'static str {
    match tone {
        StatusTone::Idle => "Idle",
        StatusTone::Busy => "Working",
        StatusTone::Info => "Info",
        StatusTone::Warning => "Warning",
        StatusTone::Error => "Error",
    }
}
