use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread,
    time::Duration,
};

use super::{AudioPlayer, EditFadeRange};
use crate::output::ResolvedOutput;

const DEFAULT_PLAYBACK_COMMAND_QUEUE: usize = 8;

/// Monotonic id assigned to playback runtime commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlaybackRequestId(u64);

impl PlaybackRequestId {
    /// Return the numeric id for persistence-free correlation by host apps.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Bounded playback runtime queue configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackRuntimeConfig {
    /// Maximum queued commands before non-blocking submit returns `QueueFull`.
    pub queue_capacity: usize,
}

impl Default for PlaybackRuntimeConfig {
    fn default() -> Self {
        Self {
            queue_capacity: DEFAULT_PLAYBACK_COMMAND_QUEUE,
        }
    }
}

/// Audio source payload and timing metadata for a playback command.
#[derive(Clone, Debug)]
pub enum PlaybackRuntimeSource {
    /// Original encoded bytes with caller-provided timing metadata.
    AudioBytes {
        data: Arc<[u8]>,
        duration: f32,
        sample_rate: u32,
        channels: usize,
    },
    /// Original encoded bytes plus pre-decoded interleaved f32 playback samples.
    DecodedSamples {
        audio_bytes: Arc<[u8]>,
        samples: Arc<[f32]>,
        duration: f32,
        sample_rate: u32,
        channels: usize,
    },
    /// Raw little-endian interleaved f32 file with caller-provided timing metadata.
    InterleavedF32File {
        path: PathBuf,
        sample_count: u64,
        duration: f32,
        sample_rate: u32,
        channels: usize,
    },
}

impl PlaybackRuntimeSource {
    fn apply_to_player(self, player: &mut AudioPlayer) {
        match self {
            Self::AudioBytes {
                data,
                duration,
                sample_rate,
                channels,
            } => player.set_audio_with_metadata(data, duration, sample_rate, channels),
            Self::DecodedSamples {
                audio_bytes,
                samples,
                duration,
                sample_rate,
                channels,
            } => player.set_audio_samples_with_metadata(
                audio_bytes,
                samples,
                duration,
                sample_rate,
                channels,
            ),
            Self::InterleavedF32File {
                path,
                sample_count,
                duration,
                sample_rate,
                channels,
            } => player.set_interleaved_f32_file_with_metadata(
                path,
                sample_count,
                duration,
                sample_rate,
                channels,
            ),
        }
    }
}

/// Span and loop behavior for a playback command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaybackRuntimeMode {
    /// Play the requested normalized span once.
    OneShot { start: f64, end: f64 },
    /// Loop the requested normalized span, starting at a normalized offset.
    Looped { start: f64, end: f64, offset: f64 },
}

impl PlaybackRuntimeMode {
    fn start_player(self, player: &mut AudioPlayer) -> Result<f32, String> {
        match self {
            Self::OneShot { start, end } => {
                player.play_range(start, end, false)?;
                Ok(start.clamp(0.0, 1.0) as f32)
            }
            Self::Looped { start, end, offset } => {
                player.play_looped_range_from(start, end, offset)?;
                Ok(offset.clamp(start.min(end), start.max(end)).clamp(0.0, 1.0) as f32)
            }
        }
    }
}

/// Complete neutral playback-start request.
#[derive(Clone, Debug)]
pub struct PlaybackRuntimeRequest {
    pub source: PlaybackRuntimeSource,
    pub mode: PlaybackRuntimeMode,
    pub volume: f32,
    pub edit_fade: Option<EditFadeRange>,
}

/// Successful playback-start outcome.
#[derive(Clone, Debug)]
pub struct PlaybackRuntimeStarted {
    pub id: PlaybackRequestId,
    pub output: ResolvedOutput,
    pub playback_start: f32,
}

/// Playback state snapshot returned by the audio-control runtime.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlaybackRuntimeProgress {
    pub active: bool,
    pub elapsed: Option<Duration>,
    pub looping: bool,
    pub progress: Option<f32>,
    pub error: Option<String>,
}

/// Reason a playback command was cancelled before execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackRuntimeCancellation {
    /// A newer queued play request replaced this one before it started.
    Superseded,
    /// A stop request cancelled this queued play request before it started.
    Stopped,
    /// The runtime was shut down before this queued play request started.
    Shutdown,
}

/// Events emitted by the playback command runtime.
#[derive(Clone, Debug)]
pub enum PlaybackRuntimeEvent {
    Started(PlaybackRuntimeStarted),
    Failed {
        id: PlaybackRequestId,
        error: String,
    },
    Cancelled {
        id: PlaybackRequestId,
        reason: PlaybackRuntimeCancellation,
    },
    Stopped {
        id: PlaybackRequestId,
    },
    Progress {
        id: PlaybackRequestId,
        progress: PlaybackRuntimeProgress,
    },
}

/// Non-blocking playback-command submission failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaybackRuntimeSubmitError {
    QueueFull,
    Closed,
}

/// Cloneable non-blocking handle for a playback command runtime.
#[derive(Clone)]
pub struct PlaybackRuntimeHandle {
    commands: SyncSender<PlaybackRuntimeCommand>,
    next_id: Arc<AtomicU64>,
}

impl PlaybackRuntimeHandle {
    /// Submit a playback request without blocking the caller.
    pub fn try_play(
        &self,
        request: PlaybackRuntimeRequest,
    ) -> Result<PlaybackRequestId, PlaybackRuntimeSubmitError> {
        let id = self.next_request_id();
        self.commands
            .try_send(PlaybackRuntimeCommand::Play { id, request })
            .map(|()| id)
            .map_err(map_try_send_error)
    }

    /// Submit a stop command without blocking the caller.
    pub fn try_stop(&self) -> Result<PlaybackRequestId, PlaybackRuntimeSubmitError> {
        let id = self.next_request_id();
        self.commands
            .try_send(PlaybackRuntimeCommand::Stop { id })
            .map(|()| id)
            .map_err(map_try_send_error)
    }

    /// Submit a non-blocking playback-progress snapshot request.
    pub fn try_poll_progress(&self) -> Result<PlaybackRequestId, PlaybackRuntimeSubmitError> {
        let id = self.next_request_id();
        self.commands
            .try_send(PlaybackRuntimeCommand::PollProgress { id })
            .map(|()| id)
            .map_err(map_try_send_error)
    }

    /// Submit a non-blocking volume update for current and future playback.
    pub fn try_set_volume(&self, volume: f32) -> Result<(), PlaybackRuntimeSubmitError> {
        self.commands
            .try_send(PlaybackRuntimeCommand::SetVolume { volume })
            .map_err(map_try_send_error)
    }

    /// Request runtime shutdown without blocking the caller.
    pub fn try_shutdown(&self) -> Result<(), PlaybackRuntimeSubmitError> {
        self.commands
            .try_send(PlaybackRuntimeCommand::Shutdown)
            .map_err(map_try_send_error)
    }

    fn next_request_id(&self) -> PlaybackRequestId {
        PlaybackRequestId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }
}

/// Handle plus event receiver for a spawned playback command runtime.
pub struct PlaybackRuntime {
    pub handle: PlaybackRuntimeHandle,
    pub events: Receiver<PlaybackRuntimeEvent>,
}

impl PlaybackRuntime {
    /// Spawn a dedicated audio-control thread that owns `AudioPlayer`.
    pub fn spawn(
        player: AudioPlayer,
        config: PlaybackRuntimeConfig,
    ) -> Result<Self, std::io::Error> {
        spawn_executor(AudioPlayerPlaybackExecutor { player }, config)
    }
}

enum PlaybackRuntimeCommand {
    Play {
        id: PlaybackRequestId,
        request: PlaybackRuntimeRequest,
    },
    Stop {
        id: PlaybackRequestId,
    },
    PollProgress {
        id: PlaybackRequestId,
    },
    SetVolume {
        volume: f32,
    },
    Shutdown,
}

trait PlaybackRuntimeExecutor: Send + 'static {
    fn play(
        &mut self,
        request: PlaybackRuntimeRequest,
    ) -> Result<PlaybackRuntimeStartedData, String>;
    fn stop(&mut self) -> Result<(), String>;
    fn set_volume(&mut self, volume: f32);
    fn progress(&mut self) -> PlaybackRuntimeProgress;
}

struct PlaybackRuntimeStartedData {
    output: ResolvedOutput,
    playback_start: f32,
}

struct AudioPlayerPlaybackExecutor {
    player: AudioPlayer,
}

impl PlaybackRuntimeExecutor for AudioPlayerPlaybackExecutor {
    fn play(
        &mut self,
        request: PlaybackRuntimeRequest,
    ) -> Result<PlaybackRuntimeStartedData, String> {
        self.player.set_volume(request.volume);
        let output = self.player.output_details().clone();
        request.source.apply_to_player(&mut self.player);
        self.player.set_edit_fade_state(request.edit_fade);
        let playback_start = request.mode.start_player(&mut self.player)?;
        Ok(PlaybackRuntimeStartedData {
            output,
            playback_start,
        })
    }

    fn stop(&mut self) -> Result<(), String> {
        self.player.stop();
        Ok(())
    }

    fn set_volume(&mut self, volume: f32) {
        self.player.set_volume(volume);
    }

    fn progress(&mut self) -> PlaybackRuntimeProgress {
        PlaybackRuntimeProgress {
            active: self.player.is_playing(),
            elapsed: self.player.playback_elapsed(),
            looping: self.player.is_looping(),
            progress: self.player.progress(),
            error: self.player.take_error(),
        }
    }
}

fn spawn_executor(
    executor: impl PlaybackRuntimeExecutor,
    config: PlaybackRuntimeConfig,
) -> Result<PlaybackRuntime, std::io::Error> {
    let capacity = config.queue_capacity.max(1);
    let (command_sender, command_receiver) = mpsc::sync_channel(capacity);
    let (event_sender, event_receiver) = mpsc::channel();
    let handle = PlaybackRuntimeHandle {
        commands: command_sender,
        next_id: Arc::new(AtomicU64::new(1)),
    };
    thread::Builder::new()
        .name(String::from("reson-playback-runtime"))
        .spawn(move || run_playback_runtime(executor, command_receiver, event_sender))?;
    Ok(PlaybackRuntime {
        handle,
        events: event_receiver,
    })
}

fn run_playback_runtime(
    mut executor: impl PlaybackRuntimeExecutor,
    commands: Receiver<PlaybackRuntimeCommand>,
    events: mpsc::Sender<PlaybackRuntimeEvent>,
) {
    while let Ok(command) = commands.recv() {
        match coalesce_command(command, &commands, &events) {
            CoalescedCommand::Play { id, request } => {
                let event = match executor.play(request) {
                    Ok(started) => PlaybackRuntimeEvent::Started(PlaybackRuntimeStarted {
                        id,
                        output: started.output,
                        playback_start: started.playback_start,
                    }),
                    Err(error) => PlaybackRuntimeEvent::Failed { id, error },
                };
                let _ = events.send(event);
            }
            CoalescedCommand::Stop { id } => {
                let event = match executor.stop() {
                    Ok(()) => PlaybackRuntimeEvent::Stopped { id },
                    Err(error) => PlaybackRuntimeEvent::Failed { id, error },
                };
                let _ = events.send(event);
            }
            CoalescedCommand::PollProgress { id } => {
                let _ = events.send(PlaybackRuntimeEvent::Progress {
                    id,
                    progress: executor.progress(),
                });
            }
            CoalescedCommand::SetVolume { volume } => {
                executor.set_volume(volume);
            }
            CoalescedCommand::Shutdown => return,
        }
    }
}

enum CoalescedCommand {
    Play {
        id: PlaybackRequestId,
        request: PlaybackRuntimeRequest,
    },
    Stop {
        id: PlaybackRequestId,
    },
    PollProgress {
        id: PlaybackRequestId,
    },
    SetVolume {
        volume: f32,
    },
    Shutdown,
}

fn coalesce_command(
    command: PlaybackRuntimeCommand,
    commands: &Receiver<PlaybackRuntimeCommand>,
    events: &mpsc::Sender<PlaybackRuntimeEvent>,
) -> CoalescedCommand {
    let mut current = command;
    loop {
        match commands.try_recv() {
            Ok(next) => current = supersede(current, next, events),
            Err(TryRecvError::Empty) => return command_to_coalesced(current),
            Err(TryRecvError::Disconnected) => {
                cancel_command(current, PlaybackRuntimeCancellation::Shutdown, events);
                return CoalescedCommand::Shutdown;
            }
        }
    }
}

fn supersede(
    current: PlaybackRuntimeCommand,
    next: PlaybackRuntimeCommand,
    events: &mpsc::Sender<PlaybackRuntimeEvent>,
) -> PlaybackRuntimeCommand {
    match (&current, &next) {
        (PlaybackRuntimeCommand::Play { id, .. }, PlaybackRuntimeCommand::Play { .. }) => {
            send_cancelled(*id, PlaybackRuntimeCancellation::Superseded, events);
        }
        (PlaybackRuntimeCommand::Play { id, .. }, PlaybackRuntimeCommand::Stop { .. }) => {
            send_cancelled(*id, PlaybackRuntimeCancellation::Stopped, events);
        }
        (PlaybackRuntimeCommand::Play { id, .. }, PlaybackRuntimeCommand::Shutdown) => {
            send_cancelled(*id, PlaybackRuntimeCancellation::Shutdown, events);
        }
        _ => {}
    }
    next
}

fn command_to_coalesced(command: PlaybackRuntimeCommand) -> CoalescedCommand {
    match command {
        PlaybackRuntimeCommand::Play { id, request } => CoalescedCommand::Play { id, request },
        PlaybackRuntimeCommand::Stop { id } => CoalescedCommand::Stop { id },
        PlaybackRuntimeCommand::PollProgress { id } => CoalescedCommand::PollProgress { id },
        PlaybackRuntimeCommand::SetVolume { volume } => CoalescedCommand::SetVolume { volume },
        PlaybackRuntimeCommand::Shutdown => CoalescedCommand::Shutdown,
    }
}

fn cancel_command(
    command: PlaybackRuntimeCommand,
    reason: PlaybackRuntimeCancellation,
    events: &mpsc::Sender<PlaybackRuntimeEvent>,
) {
    if let PlaybackRuntimeCommand::Play { id, .. } = command {
        send_cancelled(id, reason, events);
    }
}

fn send_cancelled(
    id: PlaybackRequestId,
    reason: PlaybackRuntimeCancellation,
    events: &mpsc::Sender<PlaybackRuntimeEvent>,
) {
    let _ = events.send(PlaybackRuntimeEvent::Cancelled { id, reason });
}

fn map_try_send_error(error: TrySendError<PlaybackRuntimeCommand>) -> PlaybackRuntimeSubmitError {
    match error {
        TrySendError::Full(_) => PlaybackRuntimeSubmitError::QueueFull,
        TrySendError::Disconnected(_) => PlaybackRuntimeSubmitError::Closed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    enum FakeOutcome {
        Started(f32),
        Failed(&'static str),
    }

    struct FakeExecutor {
        outcomes: Vec<FakeOutcome>,
        played: Arc<Mutex<Vec<PlaybackRuntimeMode>>>,
        stopped: Arc<Mutex<usize>>,
    }

    impl FakeExecutor {
        fn new(outcomes: Vec<FakeOutcome>) -> Self {
            Self {
                outcomes,
                played: Arc::new(Mutex::new(Vec::new())),
                stopped: Arc::new(Mutex::new(0)),
            }
        }

        fn played(&self) -> Arc<Mutex<Vec<PlaybackRuntimeMode>>> {
            Arc::clone(&self.played)
        }
    }

    impl PlaybackRuntimeExecutor for FakeExecutor {
        fn play(
            &mut self,
            request: PlaybackRuntimeRequest,
        ) -> Result<PlaybackRuntimeStartedData, String> {
            self.played.lock().expect("played lock").push(request.mode);
            match self.outcomes.pop().unwrap_or(FakeOutcome::Started(0.0)) {
                FakeOutcome::Started(playback_start) => Ok(PlaybackRuntimeStartedData {
                    output: ResolvedOutput::default(),
                    playback_start,
                }),
                FakeOutcome::Failed(error) => Err(String::from(error)),
            }
        }

        fn stop(&mut self) -> Result<(), String> {
            *self.stopped.lock().expect("stopped lock") += 1;
            Ok(())
        }

        fn set_volume(&mut self, _volume: f32) {}

        fn progress(&mut self) -> PlaybackRuntimeProgress {
            PlaybackRuntimeProgress {
                active: true,
                elapsed: Some(Duration::from_millis(10)),
                looping: false,
                progress: Some(0.25),
                error: None,
            }
        }
    }

    #[test]
    fn playback_runtime_coalesces_queued_play_requests_to_latest() {
        let executor = FakeExecutor::new(vec![FakeOutcome::Started(0.5)]);
        let played = executor.played();
        let runtime = spawn_executor(executor, PlaybackRuntimeConfig { queue_capacity: 4 })
            .expect("spawn runtime");

        let first = runtime.handle.try_play(test_request(0.1)).expect("first");
        let second = runtime.handle.try_play(test_request(0.5)).expect("second");

        let cancelled = runtime.events.recv().expect("cancelled");
        assert!(matches!(
            cancelled,
            PlaybackRuntimeEvent::Cancelled {
                id,
                reason: PlaybackRuntimeCancellation::Superseded
            } if id == first
        ));
        let started = runtime.events.recv().expect("started");
        assert!(matches!(
            started,
            PlaybackRuntimeEvent::Started(PlaybackRuntimeStarted { id, playback_start, .. })
                if id == second && playback_start == 0.5
        ));
        assert_eq!(
            played.lock().expect("played").as_slice(),
            &[PlaybackRuntimeMode::OneShot {
                start: 0.5,
                end: 1.0
            }]
        );
    }

    #[test]
    fn playback_runtime_reports_playback_failures() {
        let runtime = spawn_executor(
            FakeExecutor::new(vec![FakeOutcome::Failed("no output")]),
            PlaybackRuntimeConfig::default(),
        )
        .expect("spawn runtime");

        let id = runtime.handle.try_play(test_request(0.0)).expect("submit");
        let event = runtime.events.recv().expect("event");

        assert!(matches!(
            event,
            PlaybackRuntimeEvent::Failed { id: event_id, error }
                if event_id == id && error == "no output"
        ));
    }

    #[test]
    fn playback_runtime_submit_is_bounded_and_non_blocking() {
        let (sender, _receiver) = mpsc::sync_channel(1);
        let handle = PlaybackRuntimeHandle {
            commands: sender,
            next_id: Arc::new(AtomicU64::new(1)),
        };

        assert!(handle.try_play(test_request(0.0)).is_ok());
        assert_eq!(
            handle.try_play(test_request(0.25)),
            Err(PlaybackRuntimeSubmitError::QueueFull)
        );
    }

    #[test]
    fn playback_runtime_reports_progress_snapshots() {
        let runtime = spawn_executor(FakeExecutor::new(vec![]), PlaybackRuntimeConfig::default())
            .expect("spawn runtime");

        let id = runtime.handle.try_poll_progress().expect("poll");
        let event = runtime.events.recv().expect("event");

        assert!(matches!(
            event,
            PlaybackRuntimeEvent::Progress {
                id: event_id,
                progress
            } if event_id == id && progress.active && progress.progress == Some(0.25)
        ));
    }

    fn test_request(start: f64) -> PlaybackRuntimeRequest {
        PlaybackRuntimeRequest {
            source: PlaybackRuntimeSource::AudioBytes {
                data: Arc::<[u8]>::from([]),
                duration: 1.0,
                sample_rate: 44_100,
                channels: 1,
            },
            mode: PlaybackRuntimeMode::OneShot { start, end: 1.0 },
            volume: 1.0,
            edit_fade: None,
        }
    }
}
