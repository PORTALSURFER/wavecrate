use radiant::prelude as ui;
use radiant::widgets::PointerModifiers;
use std::{
    collections::hash_map::Entry,
    path::{Path, PathBuf},
    sync::{OnceLock, mpsc},
    time::Instant,
};

use super::waveform::cached_waveform_file_exists;
use super::{
    GuiAppState, GuiMessage, SampleLoadResult, WaveformCacheEntry, WaveformState, emit_gui_action,
    sample_path_label,
};

const SAMPLE_LOAD_PROGRESS_MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
const SAMPLE_LOAD_PROGRESS_MIN_DELTA: f32 = 0.01;
const WAVEFORM_MEMORY_CACHE_MAX_FILES: usize = 48;
const WAVEFORM_MEMORY_CACHE_MAX_BYTES: usize = 256 * 1024 * 1024;
type DeferredDropJob = Box<dyn FnOnce() + Send + 'static>;
static DEFERRED_DROP_SENDER: OnceLock<Option<mpsc::Sender<DeferredDropJob>>> = OnceLock::new();

pub(super) struct NormalizedWaveformReload<'a> {
    pub(super) path: &'a Path,
    pub(super) playback: Option<WaveformPlaybackResume>,
}

pub(super) struct WaveformPlaybackResume {
    pub(super) start_ratio: f32,
    pub(super) span: Option<(f32, f32)>,
}

impl GuiAppState {
    pub(super) fn reload_normalized_waveform(
        &mut self,
        reload: NormalizedWaveformReload<'_>,
    ) -> Result<(), String> {
        self.replace_waveform_deferred(WaveformState::load_path(reload.path.to_path_buf())?);
        self.folder_browser
            .select_file(reload.path.display().to_string());
        if let Some(playback) = reload.playback {
            let (_, previous_end) = playback.span.unwrap_or((0.0, 1.0));
            let start = playback.start_ratio.clamp(0.0, 1.0);
            let end = previous_end.max(start).clamp(start, 1.0);
            self.start_playback_current_span(start, end)?;
        }
        Ok(())
    }

    pub(super) fn select_sample(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let previous_selection = self.folder_browser.selected_file_id().map(str::to_owned);
        self.folder_browser
            .focus_file_preserving_selection(path.clone());
        if self.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.selected_metadata_tag = None;
        }
        self.load_sample(path, context);
    }

    pub(super) fn select_sample_with_modifiers(
        &mut self,
        path: String,
        modifiers: PointerModifiers,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let previous_selection = self.folder_browser.selected_file_id().map(str::to_owned);
        self.folder_browser
            .select_file_with_modifiers(path.clone(), modifiers);
        if self.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.selected_metadata_tag = None;
        }
        self.load_sample(path, context);
    }

    pub(super) fn load_sample(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.load_sample_with_autoplay(path, context, true);
    }

    pub(super) fn load_sample_without_autoplay(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.load_sample_with_autoplay(path, context, false);
    }

    fn load_sample_with_autoplay(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
        autoplay: bool,
    ) {
        let started_at = Instant::now();
        self.cancel_inflight_sample_load();
        if let Some(waveform) = self.cached_waveform_state(Path::new(&path)) {
            self.finish_cached_sample_load(waveform, autoplay, started_at);
            return;
        }
        if self.waveform.is_playing() {
            if let Some(player) = self.audio_player.as_mut() {
                player.stop();
            }
            self.waveform.stop_playback();
            self.current_playback_span = None;
        }
        self.sample_status = format!("Loading {}", sample_path_label(path.as_str()));
        let label = sample_path_label(path.as_str());
        self.waveform_loading_label = Some(label.clone());
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&label),
            "load_queued",
            started_at,
            None,
        );
        let ticket = self.sample_load_task.begin();
        let token = ui::CancellationToken::new();
        self.sample_load_cancel = Some(token.clone());
        let sender = self.worker_sender.clone();
        context.spawn_cancellable(
            "gui-sample-load",
            token,
            move |token| {
                if token.is_cancelled() {
                    return ui::TaskCompletion {
                        ticket,
                        output: SampleLoadResult {
                            path,
                            result: Err(String::from("cancelled")),
                            autoplay,
                        },
                    };
                }
                let progress_reporter =
                    std::cell::RefCell::new(SampleLoadProgressReporter::new(sender, ticket));
                let result = WaveformState::load_path_with_progress_and_cancel(
                    PathBuf::from(&path),
                    |progress| {
                        progress_reporter.borrow_mut().report(progress);
                    },
                    || token.is_cancelled(),
                );
                ui::TaskCompletion {
                    ticket,
                    output: SampleLoadResult {
                        path,
                        result,
                        autoplay,
                    },
                }
            },
            GuiMessage::SampleLoadFinished,
        );
    }

    pub(super) fn finish_sample_load(&mut self, load: ui::TaskCompletion<SampleLoadResult>) {
        let started_at = Instant::now();
        let ticket = load.ticket;
        let load = load.output;
        let label = sample_path_label(load.path.as_str());
        if !self.sample_load_task.finish(ticket) {
            emit_gui_action(
                "browser.sample_load.finish",
                Some("browser"),
                Some(&label),
                "stale",
                started_at,
                None,
            );
            return;
        }
        self.waveform_loading_label = None;
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        self.sample_load_cancel = None;
        match load.result {
            Ok(waveform) => {
                let file_name = waveform.file_name();
                self.remember_waveform(&waveform);
                self.replace_waveform_deferred(waveform);
                if !load.autoplay {
                    self.sample_status = format!("Loaded {file_name}");
                    emit_gui_action(
                        "browser.sample_load.finish",
                        Some("browser"),
                        Some(&file_name),
                        "loaded",
                        started_at,
                        None,
                    );
                    return;
                }
                match self.start_playback_current_span(0.0, 1.0) {
                    Ok(()) => {
                        self.sample_status = format!("Playing {file_name}");
                        emit_gui_action(
                            "browser.sample_load.finish",
                            Some("browser"),
                            Some(&file_name),
                            "playing",
                            started_at,
                            None,
                        );
                    }
                    Err(err) => {
                        self.sample_status =
                            format!("Loaded {file_name} | playback unavailable: {err}");
                        emit_gui_action(
                            "browser.sample_load.finish",
                            Some("browser"),
                            Some(&file_name),
                            "loaded_playback_error",
                            started_at,
                            Some(&err),
                        );
                    }
                }
            }
            Err(err) => {
                self.sample_status = format!("Could not load sample: {err}");
                emit_gui_action(
                    "browser.sample_load.finish",
                    Some("browser"),
                    Some(&label),
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    fn finish_cached_sample_load(
        &mut self,
        waveform: WaveformState,
        autoplay: bool,
        started_at: Instant,
    ) {
        if self.waveform.is_playing() {
            if let Some(player) = self.audio_player.as_mut() {
                player.stop();
            }
            self.waveform.stop_playback();
            self.current_playback_span = None;
        }
        let file_name = waveform.file_name();
        self.cancel_inflight_sample_load();
        self.waveform_loading_label = None;
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        self.replace_waveform_deferred(waveform);
        if !autoplay {
            self.sample_status = format!("Loaded {file_name}");
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&file_name),
                "cache_loaded",
                started_at,
                None,
            );
            return;
        }
        match self.start_playback_current_span(0.0, 1.0) {
            Ok(()) => {
                self.sample_status = format!("Playing {file_name}");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&file_name),
                    "cache_playing",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.sample_status = format!("Loaded {file_name} | playback unavailable: {err}");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&file_name),
                    "cache_loaded_playback_error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    fn cached_waveform_state(&mut self, path: &Path) -> Option<WaveformState> {
        let path = path.to_path_buf();
        let signature = sample_file_signature(&path)?;
        let entry = self.waveform_cache.get(&path)?;
        if entry.signature != signature {
            self.remove_waveform_cache_path(&path);
            self.cached_sample_paths.remove(&path.display().to_string());
            return None;
        }
        let file = std::sync::Arc::clone(&entry.file);
        self.touch_waveform_cache_path(path.clone());
        Some(WaveformState::from_cached_file(file))
    }

    fn remember_waveform(&mut self, waveform: &WaveformState) {
        if !waveform.has_loaded_sample() {
            return;
        }
        let path = waveform.path();
        let Some(signature) = sample_file_signature(&path) else {
            return;
        };
        let entry = WaveformCacheEntry {
            byte_len: waveform.audio_bytes().len()
                + waveform
                    .playback_samples()
                    .map(|samples| samples.len() * std::mem::size_of::<f32>())
                    .unwrap_or(0),
            file: waveform.file(),
            signature,
        };
        match self.waveform_cache.entry(path.clone()) {
            Entry::Occupied(mut occupied) => {
                let old_entry = std::mem::replace(occupied.get_mut(), entry);
                self.waveform_cache_bytes = self
                    .waveform_cache_bytes
                    .saturating_sub(old_entry.byte_len)
                    .saturating_add(occupied.get().byte_len);
                defer_large_drop(old_entry);
            }
            Entry::Vacant(vacant) => {
                self.waveform_cache_bytes =
                    self.waveform_cache_bytes.saturating_add(entry.byte_len);
                vacant.insert(entry);
            }
        }
        self.cached_sample_paths.insert(path.display().to_string());
        self.touch_waveform_cache_path(path);
        self.enforce_waveform_cache_limit();
    }

    fn touch_waveform_cache_path(&mut self, path: PathBuf) {
        self.waveform_cache_order.retain(|cached| cached != &path);
        self.waveform_cache_order.push_back(path);
    }

    fn enforce_waveform_cache_limit(&mut self) {
        while self.waveform_cache_order.len() > WAVEFORM_MEMORY_CACHE_MAX_FILES
            || (self.waveform_cache_bytes > WAVEFORM_MEMORY_CACHE_MAX_BYTES
                && self.waveform_cache_order.len() > 1)
        {
            let Some(path) = self.waveform_cache_order.pop_front() else {
                break;
            };
            if self.remove_waveform_cache_path(&path) {
                self.remove_cached_sample_path_if_not_persisted(&path);
            }
        }
    }

    fn remove_waveform_cache_path(&mut self, path: &Path) -> bool {
        let Some(entry) = self.waveform_cache.remove(path) else {
            return false;
        };
        self.waveform_cache_bytes = self.waveform_cache_bytes.saturating_sub(entry.byte_len);
        defer_large_drop(entry);
        true
    }

    pub(super) fn refresh_persisted_waveform_cache_indicators(&mut self) {
        let audio_files = self
            .folder_browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        for file_id in audio_files {
            let path = PathBuf::from(&file_id);
            if self.waveform_cache.contains_key(&path) || cached_waveform_file_exists(&path) {
                self.cached_sample_paths.insert(file_id);
            } else {
                self.cached_sample_paths.remove(&file_id);
            }
        }
    }

    fn remove_cached_sample_path_if_not_persisted(&mut self, path: &Path) {
        if !cached_waveform_file_exists(path) {
            self.cached_sample_paths.remove(&path.display().to_string());
        }
    }

    fn cancel_inflight_sample_load(&mut self) {
        if let Some(token) = self.sample_load_cancel.take() {
            token.cancel();
        }
        self.sample_load_task.cancel();
    }

    fn replace_waveform_deferred(&mut self, waveform: WaveformState) {
        let previous = std::mem::replace(&mut self.waveform, waveform);
        defer_large_drop(previous);
    }
}

fn sample_file_signature(path: &Path) -> Option<super::SampleFileSignature> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified_ns = metadata
        .modified()
        .ok()?
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .ok()?
        .as_nanos()
        .try_into()
        .ok()?;
    Some(super::SampleFileSignature {
        size_bytes: metadata.len(),
        modified_ns,
    })
}

fn defer_large_drop<T: Send + 'static>(value: T) {
    let job: DeferredDropJob = Box::new(move || drop(value));
    let Some(sender) = deferred_drop_sender() else {
        job();
        return;
    };
    if let Err(err) = sender.send(job) {
        (err.0)();
    }
}

fn deferred_drop_sender() -> Option<&'static mpsc::Sender<DeferredDropJob>> {
    DEFERRED_DROP_SENDER
        .get_or_init(|| {
            let (sender, receiver) = mpsc::channel::<DeferredDropJob>();
            match std::thread::Builder::new()
                .name(String::from("wavecrate-deferred-drop"))
                .spawn(move || {
                    while let Ok(job) = receiver.recv() {
                        job();
                    }
                }) {
                Ok(_) => Some(sender),
                Err(err) => {
                    tracing::warn!("Failed to spawn deferred waveform drop worker: {err}");
                    None
                }
            }
        })
        .as_ref()
}

#[cfg(test)]
fn deferred_drop_sender_initialized_for_tests() -> bool {
    DEFERRED_DROP_SENDER.get().is_some()
}

#[cfg(test)]
mod deferred_drop_tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::{Duration, Instant};

    #[test]
    fn deferred_drop_uses_reusable_worker() {
        let dropped = Arc::new(AtomicBool::new(false));
        struct Probe(Arc<AtomicBool>);
        impl Drop for Probe {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }

        defer_large_drop(Probe(Arc::clone(&dropped)));

        let deadline = Instant::now() + Duration::from_secs(2);
        while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(dropped.load(Ordering::Acquire));
        assert!(deferred_drop_sender_initialized_for_tests());
    }
}

struct SampleLoadProgressReporter {
    sender: std::sync::mpsc::Sender<GuiMessage>,
    ticket: ui::TaskTicket,
    last_sent_at: Option<Instant>,
    last_progress: f32,
}

impl SampleLoadProgressReporter {
    fn new(sender: std::sync::mpsc::Sender<GuiMessage>, ticket: ui::TaskTicket) -> Self {
        Self {
            sender,
            ticket,
            last_sent_at: None,
            last_progress: 0.0,
        }
    }

    fn report(&mut self, progress: f32) {
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
