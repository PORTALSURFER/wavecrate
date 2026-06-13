use radiant::prelude as ui;
use std::{
    cell::RefCell,
    path::PathBuf,
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use crate::native_app::{
    app::{GuiMessage, SampleLoadResult, SamplePlaybackReady, WaveformState},
    audio::sample_load_actions::{
        log_loaded_sample_metadata, log_sample_load_timing,
        types::{SampleLoadRequest, SampleLoadStrategy},
    },
};

const SAMPLE_LOAD_PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(50);
const SAMPLE_LOAD_PROGRESS_MIN_DELTA: f32 = 0.01;

pub(super) struct SampleLoadWorker {
    request: SampleLoadRequest,
    sender: Sender<GuiMessage>,
}

impl SampleLoadWorker {
    pub(super) fn new(request: SampleLoadRequest, sender: Sender<GuiMessage>) -> Self {
        Self { request, sender }
    }

    pub(super) fn run(
        self,
        ticket: ui::TaskTicket,
        context: ui::BusinessWorkContext,
    ) -> SampleLoadResult {
        log_sample_load_timing(
            "browser.sample_load.worker.queue_wait",
            self.request.path(),
            self.request.queue_wait(Instant::now()),
            true,
        );
        if context.is_cancelled() {
            let autoplay = self.request.autoplay();
            return SampleLoadResult {
                path: self.request.into_path(),
                result: Err(String::from("cancelled")),
                autoplay,
            };
        }

        let progress_reporter = Self::progress_reporter(ticket, &self.sender);
        let result = self.load(ticket, &context, &progress_reporter);
        let autoplay = self.request.autoplay();
        SampleLoadResult {
            path: self.request.path().to_owned(),
            result,
            autoplay,
        }
    }

    fn load(
        &self,
        ticket: ui::TaskTicket,
        context: &ui::BusinessWorkContext,
        progress_reporter: &RefCell<ui::ThrottledProgressReporter<impl FnMut(f32)>>,
    ) -> Result<WaveformState, String> {
        match self.request.strategy() {
            SampleLoadStrategy::PersistedPlaybackCacheOnly => {
                self.load_persisted_playback_cache("browser.sample_load.worker.persisted_cache")
            }
            SampleLoadStrategy::PreferPersistedPlaybackCache => {
                let result = self.load_persisted_playback_cache(
                    "browser.sample_load.worker.persisted_cache_probe",
                );
                if result.is_ok() {
                    result
                } else {
                    self.load_decoded_sample(ticket, context, progress_reporter)
                }
            }
            SampleLoadStrategy::Decode => {
                self.load_decoded_sample(ticket, context, progress_reporter)
            }
        }
    }

    fn load_persisted_playback_cache(&self, event: &'static str) -> Result<WaveformState, String> {
        let phase_started_at = Instant::now();
        let result =
            WaveformState::load_persisted_playback_cache(PathBuf::from(self.request.path()));
        log_sample_load_timing(event, self.request.path(), phase_started_at.elapsed(), true);
        log_loaded_sample_metadata(self.request.path(), &result, "persisted_playback_cache");
        result
    }

    fn load_decoded_sample(
        &self,
        ticket: ui::TaskTicket,
        context: &ui::BusinessWorkContext,
        progress_reporter: &RefCell<ui::ThrottledProgressReporter<impl FnMut(f32)>>,
    ) -> Result<WaveformState, String> {
        let phase_started_at = Instant::now();
        let ready_sender = self.sender.clone();
        let ready_path = self.request.path().to_owned();
        let autoplay = self.request.autoplay();
        let result = WaveformState::load_path_with_progress_cancel_and_playback_ready(
            PathBuf::from(self.request.path()),
            |progress| {
                progress_reporter.borrow_mut().report(progress);
            },
            || context.is_cancelled(),
            |audio| {
                if autoplay && !context.is_cancelled() {
                    let _ =
                        ready_sender.send(GuiMessage::SamplePlaybackReady(ui::TaskCompletion {
                            ticket,
                            output: SamplePlaybackReady {
                                path: ready_path.clone(),
                                audio,
                                autoplay,
                            },
                        }));
                }
            },
        );
        log_sample_load_timing(
            "browser.sample_load.worker.decode_waveform",
            self.request.path(),
            phase_started_at.elapsed(),
            true,
        );
        log_loaded_sample_metadata(self.request.path(), &result, "uncached_decode");
        result
    }

    fn progress_reporter(
        ticket: ui::TaskTicket,
        sender: &Sender<GuiMessage>,
    ) -> RefCell<ui::ThrottledProgressReporter<impl FnMut(f32)>> {
        let progress_gate = ui::ProgressUpdateGate::new(
            SAMPLE_LOAD_PROGRESS_MIN_INTERVAL,
            SAMPLE_LOAD_PROGRESS_MIN_DELTA,
        )
        .with_max_fraction(0.995);
        let progress_sender = sender.clone();
        RefCell::new(ui::ThrottledProgressReporter::new(
            progress_gate,
            move |progress| {
                let _ = progress_sender.send(GuiMessage::SampleLoadProgress(ticket, progress));
            },
        ))
    }
}
