use super::JobMessageSender;
use super::{AudioLoadResult, JobMessage, RecordingWaveformLoadResult, SearchResult};
use crate::app::controller::state::runtime::WavLoadResult;
use crate::gui::repaint::SharedRepaintSignal;
use std::{
    sync::{Arc, mpsc::Receiver},
    thread,
};

/// Inputs required to wire result-channel forwarding threads.
pub(super) struct JobForwarderSpawnConfig {
    pub(super) message_tx: JobMessageSender,
    pub(super) repaint_signal: Arc<SharedRepaintSignal>,
    pub(super) wav_job_rx: Receiver<WavLoadResult>,
    pub(super) audio_job_rx: Receiver<AudioLoadResult>,
    pub(super) recording_waveform_job_rx: Receiver<RecordingWaveformLoadResult>,
    pub(super) search_job_rx: Receiver<SearchResult>,
}

/// Join handles for job result forwarding threads to shut them down deterministically.
pub(super) struct JobForwarderHandles {
    wav: thread::JoinHandle<()>,
    audio: thread::JoinHandle<()>,
    recording_waveform: thread::JoinHandle<()>,
    search: thread::JoinHandle<()>,
}

impl JobForwarderHandles {
    /// Spawn forwarding threads for background result channels.
    pub(super) fn spawn(config: JobForwarderSpawnConfig) -> Self {
        let JobForwarderSpawnConfig {
            message_tx,
            repaint_signal,
            wav_job_rx,
            audio_job_rx,
            recording_waveform_job_rx,
            search_job_rx,
        } = config;
        let wav = spawn_result_forwarder(
            message_tx.clone(),
            repaint_signal.clone(),
            wav_job_rx,
            JobMessage::WavLoaded,
        );
        let audio = spawn_result_forwarder(
            message_tx.clone(),
            repaint_signal.clone(),
            audio_job_rx,
            JobMessage::AudioLoaded,
        );
        let recording_waveform = spawn_result_forwarder(
            message_tx.clone(),
            repaint_signal.clone(),
            recording_waveform_job_rx,
            JobMessage::RecordingWaveformLoaded,
        );
        let search = spawn_result_forwarder(
            message_tx,
            repaint_signal,
            search_job_rx,
            JobMessage::BrowserSearchFinished,
        );
        Self {
            wav,
            audio,
            recording_waveform,
            search,
        }
    }

    /// Join all forwarding threads.
    pub(super) fn join(self) {
        let _ = self.wav.join();
        let _ = self.audio.join();
        let _ = self.recording_waveform.join();
        let _ = self.search.join();
    }
}

/// Inputs required to forward progress/event messages until a terminal event.
pub(super) struct ProgressForwarderConfig<T> {
    pub(super) message_tx: JobMessageSender,
    pub(super) repaint_signal: Arc<SharedRepaintSignal>,
    pub(super) rx: Receiver<T>,
    pub(super) wrap: fn(T) -> JobMessage,
    pub(super) is_finished: fn(&T) -> bool,
}

/// Spawn a forwarding thread that relays messages until a terminal message arrives.
pub(super) fn spawn_progress_forwarder<T: Send + 'static>(config: ProgressForwarderConfig<T>) {
    thread::spawn(move || {
        let ProgressForwarderConfig {
            message_tx,
            repaint_signal,
            rx,
            wrap,
            is_finished,
        } = config;
        while let Ok(message) = rx.recv() {
            let should_break = is_finished(&message);
            let _ = message_tx.send(wrap(message));
            repaint_signal.request_repaint();
            if should_break {
                break;
            }
        }
    });
}

/// Spawn a perpetual forwarding thread for completion/result channels.
fn spawn_result_forwarder<T: Send + 'static>(
    message_tx: JobMessageSender,
    repaint_signal: Arc<SharedRepaintSignal>,
    rx: Receiver<T>,
    wrap: fn(T) -> JobMessage,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(message) = rx.recv() {
            let _ = message_tx.send(wrap(message));
            repaint_signal.request_repaint();
        }
    })
}
