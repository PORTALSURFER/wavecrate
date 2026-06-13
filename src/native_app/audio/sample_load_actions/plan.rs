use radiant::prelude as ui;
use std::time::{Duration, Instant};

use crate::native_app::{
    app::{GuiMessage, NativeAppState, emit_gui_action, sample_path_label},
    audio::sample_load_actions::{
        foreground_sample_load_priority, log_sample_load_timing,
        types::{SampleLoadRequest, SampleLoadStrategy},
        worker::SampleLoadWorker,
    },
};

impl NativeAppState {
    pub(super) fn schedule_deferred_sample_load(
        &mut self,
        path: String,
        autoplay: bool,
        check_cache: bool,
        delay: Duration,
        input_method: &'static str,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        tracing::info!(
            target: "wavecrate::debug::sample_load",
            event = "browser.sample_load.deferred_scheduled",
            path = %path,
            input_method,
            cache_state = "uncached",
            autoplay,
            delay_ms = delay.as_secs_f64() * 1000.0,
            "Sample load scheduled"
        );
        context.after_latest(
            &mut self.background.deferred_sample_load_task,
            delay,
            |ticket| GuiMessage::DeferredSampleLoad {
                ticket,
                path,
                autoplay,
                check_cache,
                scheduled_at: Instant::now(),
            },
        );
    }

    pub(in crate::native_app) fn start_deferred_sample_load(
        &mut self,
        ticket: ui::TaskTicket,
        path: String,
        autoplay: bool,
        check_cache: bool,
        scheduled_at: Instant,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        log_sample_load_timing(
            "browser.sample_load.deferred_wait",
            path.as_str(),
            started_at.saturating_duration_since(scheduled_at),
            true,
        );
        if !self.background.deferred_sample_load_task.finish(ticket)
            || self.library.folder_browser.selected_file_id() != Some(path.as_str())
        {
            self.audio.pending_sample_playback = None;
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&sample_path_label(path.as_str())),
                "load_deferred_stale",
                started_at,
                None,
            );
            return;
        }
        let strategy = if check_cache {
            SampleLoadStrategy::PreferPersistedPlaybackCache
        } else {
            SampleLoadStrategy::Decode
        };
        self.start_sample_load(path, autoplay, context, strategy, started_at);
    }

    pub(super) fn prepare_uncached_sample_load(
        &mut self,
        path: &str,
        outcome: &'static str,
        started_at: Instant,
    ) {
        self.stop_current_sample_playback_for_load();
        self.ui.status.sample = format!("Loading {}", sample_path_label(path));
        let label = sample_path_label(path);
        self.waveform.load.label = Some(label.clone());
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&label),
            outcome,
            started_at,
            None,
        );
    }

    pub(super) fn start_sample_load(
        &mut self,
        path: String,
        autoplay: bool,
        context: &mut ui::UpdateContext<GuiMessage>,
        strategy: SampleLoadStrategy,
        started_at: Instant,
    ) {
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&sample_path_label(path.as_str())),
            "load_queued",
            started_at,
            None,
        );
        self.start_sample_load_with_priority(
            path,
            autoplay,
            context,
            foreground_sample_load_priority(),
            strategy,
        );
    }

    pub(super) fn start_sample_load_with_priority(
        &mut self,
        path: String,
        autoplay: bool,
        context: &mut ui::UpdateContext<GuiMessage>,
        priority: ui::TaskPriority,
        strategy: SampleLoadStrategy,
    ) {
        let sender = self.background.worker_sender.clone();
        let request = SampleLoadRequest::new(path, autoplay, priority, strategy);
        let load = match request.priority() {
            ui::TaskPriority::Interactive => context.business().interactive("gui-sample-load"),
            ui::TaskPriority::Background => context.business().background("gui-sample-load"),
            ui::TaskPriority::Idle => context.business().idle("gui-sample-load"),
        }
        .cancellable()
        .latest(&mut self.background.sample_load_task);
        let ticket = load.ticket();
        self.background.sample_load_cancel = Some(load.run(
            move |worker_context| {
                SampleLoadWorker::new(request, sender).run(ticket, worker_context)
            },
            GuiMessage::SampleLoadFinished,
        ));
    }
}
