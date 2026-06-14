use radiant::prelude as ui;
use std::{
    cell::RefCell,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::native_app::{
    app::{SampleLoadResult, SamplePlaybackReady, WaveformState},
    audio::sample_load_actions::{
        log_loaded_sample_metadata, log_sample_load_timing,
        types::{SampleLoadRequest, SampleLoadStrategy},
    },
};

const SAMPLE_LOAD_PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(50);
const SAMPLE_LOAD_PROGRESS_MIN_DELTA: f32 = 0.01;

pub(super) enum SampleLoadWorkerEvent {
    Progress(f32),
    PlaybackReady(SamplePlaybackReady),
}

pub(super) struct SampleLoadWorker {
    request: SampleLoadRequest,
}

impl SampleLoadWorker {
    pub(super) fn new(request: SampleLoadRequest) -> Self {
        Self { request }
    }

    pub(super) fn run(
        self,
        context: radiant::runtime::BusinessWorkContext,
        events: ui::BusinessEventSink<SampleLoadWorkerEvent>,
    ) -> SampleLoadResult {
        log_sample_load_timing(
            "browser.sample_load.worker.queue_wait",
            self.request.path(),
            self.request.queue_wait(Instant::now()),
            true,
        );
        if context.check_cancelled().is_err() {
            let autoplay = self.request.autoplay();
            return SampleLoadResult {
                path: self.request.into_path(),
                result: Err(String::from("cancelled")),
                autoplay,
            };
        }

        let progress_reporter = Self::progress_reporter(events.clone());
        let result = self.load(&context, &events, &progress_reporter);
        let autoplay = self.request.autoplay();
        SampleLoadResult {
            path: self.request.path().to_owned(),
            result,
            autoplay,
        }
    }

    fn load(
        &self,
        context: &radiant::runtime::BusinessWorkContext,
        events: &ui::BusinessEventSink<SampleLoadWorkerEvent>,
        progress_reporter: &RefCell<ui::ThrottledProgressReporter<impl FnMut(f32)>>,
    ) -> Result<WaveformState, String> {
        match self.request.strategy() {
            SampleLoadStrategy::Decode => {
                self.load_decoded_sample(context, events, progress_reporter)
            }
        }
    }

    fn load_decoded_sample(
        &self,
        context: &radiant::runtime::BusinessWorkContext,
        events: &ui::BusinessEventSink<SampleLoadWorkerEvent>,
        progress_reporter: &RefCell<ui::ThrottledProgressReporter<impl FnMut(f32)>>,
    ) -> Result<WaveformState, String> {
        let phase_started_at = Instant::now();
        let ready_events = events.clone();
        let ready_path = self.request.path().to_owned();
        let autoplay = self.request.autoplay();
        let result = WaveformState::load_path_for_foreground_audition(
            PathBuf::from(self.request.path()),
            |progress| {
                let _ = context.yield_if_elapsed(Duration::from_millis(8));
                progress_reporter.borrow_mut().report(progress);
            },
            || context.is_cancelled(),
            |audio| {
                if autoplay && !context.is_cancelled() {
                    let _ = ready_events.emit(SampleLoadWorkerEvent::PlaybackReady(
                        SamplePlaybackReady {
                            path: ready_path.clone(),
                            audio,
                            autoplay,
                        },
                    ));
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
        events: ui::BusinessEventSink<SampleLoadWorkerEvent>,
    ) -> RefCell<ui::ThrottledProgressReporter<impl FnMut(f32)>> {
        let progress_gate = ui::ProgressUpdateGate::new(
            SAMPLE_LOAD_PROGRESS_MIN_INTERVAL,
            SAMPLE_LOAD_PROGRESS_MIN_DELTA,
        )
        .with_max_fraction(0.995);
        RefCell::new(ui::ThrottledProgressReporter::new(
            progress_gate,
            move |progress| {
                let _ = events.emit(SampleLoadWorkerEvent::Progress(progress));
            },
        ))
    }
}
