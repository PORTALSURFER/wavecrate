//! UI status message helpers owned by the controller.

use super::*;
use crate::app::state::StatusScope;

impl AppController {
    pub(crate) fn set_status(&mut self, text: impl Into<String>, tone: StatusTone) {
        self.set_scoped_status(text, tone, StatusScope::Passive, StatusHold::Ongoing);
    }

    pub(crate) fn set_background_status(&mut self, text: impl Into<String>, tone: StatusTone) {
        self.set_scoped_status(
            text,
            tone,
            StatusScope::BackgroundMaintenance,
            StatusHold::Ongoing,
        );
    }

    pub(crate) fn set_file_op_status(&mut self, text: impl Into<String>, tone: StatusTone) {
        self.set_scoped_status(text, tone, StatusScope::FileOp, StatusHold::Ongoing);
    }

    pub(crate) fn complete_file_op_status(&mut self, text: impl Into<String>, tone: StatusTone) {
        self.set_scoped_status(text, tone, StatusScope::FileOp, StatusHold::Release);
    }

    fn set_scoped_status(
        &mut self,
        text: impl Into<String>,
        tone: StatusTone,
        scope: StatusScope,
        hold: StatusHold,
    ) {
        let text = text.into();
        if self.status_scope_can_replace_visible(scope, tone) {
            let status_changed = self.ui.status.text != text
                || self.ui.status.status_tone != tone
                || self.ui.status.visible_scope != scope;
            self.ui.status.text = text.clone();
            self.ui.status.status_tone = tone;
            self.ui.status.visible_scope = match hold {
                StatusHold::Ongoing => scope,
                StatusHold::Release => StatusScope::Passive,
            };
            if status_changed {
                self.mark_status_projection_revision_dirty();
            }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusHold {
    Ongoing,
    Release,
}

impl AppController {
    fn status_scope_can_replace_visible(&self, scope: StatusScope, tone: StatusTone) -> bool {
        tone == StatusTone::Error
            || self.ui.status.status_tone == StatusTone::Idle
            || scope >= self.ui.status.visible_scope
    }
}

fn log_status_entry(tone: StatusTone, entry: &str) {
    match tone {
        StatusTone::Warning => tracing::warn!("{entry}"),
        StatusTone::Error => tracing::error!("{entry}"),
        StatusTone::Info | StatusTone::Busy | StatusTone::Idle => tracing::info!("{entry}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_file_op_status_defers_lower_priority_background_info_but_logs_it() {
        let (mut controller, _source) = crate::app::controller::test_support::dummy_controller();

        controller.set_file_op_status("Auto renaming 1 sample(s)...", StatusTone::Busy);
        controller.set_background_status(
            "Quick sync complete: 0 added, 1 updated, 0 missing",
            StatusTone::Info,
        );
        controller.set_status("Cached 1 wav files", StatusTone::Info);

        assert_eq!(controller.ui.status.text, "Auto renaming 1 sample(s)...");
        assert_eq!(controller.ui.status.status_tone, StatusTone::Busy);
        assert_eq!(controller.ui.status.visible_scope, StatusScope::FileOp);
        assert!(
            controller
                .ui
                .status
                .log
                .iter()
                .any(|entry| entry.contains("Quick sync complete"))
        );
        assert!(
            controller
                .ui
                .status
                .log
                .iter()
                .any(|entry| entry.contains("Cached 1 wav files"))
        );
    }

    #[test]
    fn file_op_completion_replaces_active_status_and_releases_ownership() {
        let (mut controller, _source) = crate::app::controller::test_support::dummy_controller();

        controller.set_file_op_status("Auto renaming 1 sample(s)...", StatusTone::Busy);
        controller.complete_file_op_status(
            "Auto Rename: renamed 1, skipped 0, failed 0",
            StatusTone::Info,
        );

        assert_eq!(
            controller.ui.status.text,
            "Auto Rename: renamed 1, skipped 0, failed 0"
        );
        assert_eq!(controller.ui.status.status_tone, StatusTone::Info);
        assert_eq!(controller.ui.status.visible_scope, StatusScope::Passive);
    }

    #[test]
    fn error_status_preempts_active_file_op() {
        let (mut controller, _source) = crate::app::controller::test_support::dummy_controller();

        controller.set_file_op_status("Auto renaming 1 sample(s)...", StatusTone::Busy);
        controller.set_status("File operation failed: disk full", StatusTone::Error);

        assert_eq!(
            controller.ui.status.text,
            "File operation failed: disk full"
        );
        assert_eq!(controller.ui.status.status_tone, StatusTone::Error);
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
