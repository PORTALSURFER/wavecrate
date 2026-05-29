use radiant::prelude as ui;
use std::time::Instant;

use crate::gui_app::GuiMessage;

const SAMPLE_LOAD_PROGRESS_MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
const SAMPLE_LOAD_PROGRESS_MIN_DELTA: f32 = 0.01;

pub(super) struct SampleLoadProgressReporter {
    sender: std::sync::mpsc::Sender<GuiMessage>,
    ticket: ui::TaskTicket,
    last_sent_at: Option<Instant>,
    last_progress: f32,
}

impl SampleLoadProgressReporter {
    pub(super) fn new(sender: std::sync::mpsc::Sender<GuiMessage>, ticket: ui::TaskTicket) -> Self {
        Self {
            sender,
            ticket,
            last_sent_at: None,
            last_progress: 0.0,
        }
    }

    pub(super) fn report(&mut self, progress: f32) {
        self.report_at(progress, Instant::now());
    }

    fn report_at(&mut self, progress: f32, now: Instant) {
        let progress = progress.clamp(0.0, 0.995);
        if !self.should_send(progress, now) {
            return;
        }
        self.last_sent_at = Some(now);
        self.last_progress = progress;
        let _ = self
            .sender
            .send(GuiMessage::SampleLoadProgress(self.ticket, progress));
    }

    fn should_send(&self, progress: f32, now: Instant) -> bool {
        if progress >= 0.995 {
            return true;
        }
        let Some(last_sent_at) = self.last_sent_at else {
            return true;
        };
        if progress <= self.last_progress {
            return false;
        }
        now.duration_since(last_sent_at) >= SAMPLE_LOAD_PROGRESS_MIN_INTERVAL
            && progress - self.last_progress >= SAMPLE_LOAD_PROGRESS_MIN_DELTA
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, time::Duration};

    #[test]
    fn sample_load_progress_reporter_coalesces_tight_progress_loop() {
        let (sender, receiver) = mpsc::channel();
        let ticket = ui::LatestTask::new().begin();
        let mut reporter = SampleLoadProgressReporter::new(sender, ticket);
        let start = Instant::now();

        reporter.report_at(0.001, start);
        reporter.report_at(0.002, start + Duration::from_millis(1));
        reporter.report_at(0.003, start + Duration::from_millis(2));
        reporter.report_at(0.012, start + Duration::from_millis(3));
        reporter.report_at(0.014, start + Duration::from_millis(60));

        let messages = receiver.try_iter().collect::<Vec<_>>();
        assert_eq!(
            messages.len(),
            2,
            "tight progress callbacks should be coalesced so drag hover events are not starved"
        );
        assert!(matches!(
            messages.last(),
            Some(GuiMessage::SampleLoadProgress(_, progress)) if (*progress - 0.014).abs() < f32::EPSILON
        ));
    }
}
