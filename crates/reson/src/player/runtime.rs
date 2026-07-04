use std::{
    collections::VecDeque,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    ops::Range,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, SendError, Sender, TryRecvError},
    },
    thread,
    time::Duration,
};

use super::{AudioPlayer, EditFadeRange, PlaybackMetronomeConfig};
use crate::output::ResolvedOutput;

const DEFAULT_PLAYBACK_COMMAND_QUEUE: usize = 8;
const F32_SAMPLE_BYTES: usize = std::mem::size_of::<f32>();
const NORMALIZED_GAIN_READ_SAMPLES: usize = 4096;

/// Monotonic id assigned to playback runtime commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlaybackRequestId(u64);

impl PlaybackRequestId {
    /// Return the numeric id for persistence-free correlation by host apps.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Playback runtime queue configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackRuntimeConfig {
    /// Deprecated compatibility field. Playback commands are coalesced on the
    /// runtime thread instead of being rejected by a small bounded inbox.
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
    /// Encoded audio file path with caller-provided timing metadata.
    AudioFile {
        path: PathBuf,
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
            Self::AudioFile {
                path,
                duration,
                sample_rate,
                channels,
            } => player.set_audio_file_with_metadata(path, duration, sample_rate, channels),
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
    fn start_player(
        self,
        player: &mut AudioPlayer,
        metronome: Option<PlaybackMetronomeConfig>,
    ) -> Result<f32, String> {
        match self {
            Self::OneShot { start, end } => {
                player.play_range_with_metronome(start, end, false, metronome)?;
                Ok(start.clamp(0.0, 1.0) as f32)
            }
            Self::Looped { start, end, offset } => {
                player.play_looped_range_from_with_metronome(start, end, offset, metronome)?;
                Ok(offset.clamp(start.min(end), start.max(end)).clamp(0.0, 1.0) as f32)
            }
        }
    }
}

/// In-place span-bound update for already running playback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackRuntimeSpanUpdate {
    pub start: f64,
    pub end: f64,
    pub offset: f64,
    pub seek_to_offset: bool,
    pub looped: bool,
    pub playback_gain: f32,
    pub playback_gain_normalization: Option<PlaybackRuntimeGainNormalization>,
    pub metronome: Option<PlaybackMetronomeConfig>,
}

/// Runtime-owned gain normalization for one normalized playback span.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackRuntimeGainNormalization {
    pub start: f32,
    pub end: f32,
}

impl PlaybackRuntimeGainNormalization {
    pub fn new(start: f32, end: f32) -> Self {
        Self { start, end }
    }
}

/// Complete neutral playback-start request.
#[derive(Clone, Debug)]
pub struct PlaybackRuntimeRequest {
    pub source: PlaybackRuntimeSource,
    pub mode: PlaybackRuntimeMode,
    pub volume: f32,
    pub playback_gain: f32,
    pub playback_gain_normalization: Option<PlaybackRuntimeGainNormalization>,
    pub edit_fade: Option<EditFadeRange>,
    pub metronome: Option<PlaybackMetronomeConfig>,
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

/// Cloneable handle for submitting playback commands to the runtime thread.
#[derive(Clone)]
pub struct PlaybackRuntimeHandle {
    commands: Sender<PlaybackRuntimeCommand>,
    next_id: Arc<AtomicU64>,
}

impl PlaybackRuntimeHandle {
    /// Submit a playback request without waiting for the runtime thread.
    pub fn try_play(
        &self,
        request: PlaybackRuntimeRequest,
    ) -> Result<PlaybackRequestId, PlaybackRuntimeSubmitError> {
        let id = self.next_request_id();
        self.commands
            .send(PlaybackRuntimeCommand::Play { id, request })
            .map(|()| id)
            .map_err(map_send_error)
    }

    /// Submit a stop command without waiting for the runtime thread.
    pub fn try_stop(&self) -> Result<PlaybackRequestId, PlaybackRuntimeSubmitError> {
        let id = self.next_request_id();
        self.commands
            .send(PlaybackRuntimeCommand::Stop { id })
            .map(|()| id)
            .map_err(map_send_error)
    }

    /// Submit a playback-progress snapshot request without waiting for the runtime thread.
    pub fn try_poll_progress(&self) -> Result<PlaybackRequestId, PlaybackRuntimeSubmitError> {
        let id = self.next_request_id();
        self.commands
            .send(PlaybackRuntimeCommand::PollProgress { id })
            .map(|()| id)
            .map_err(map_send_error)
    }

    /// Submit a volume update for current and future playback without waiting for the runtime thread.
    pub fn try_set_volume(&self, volume: f32) -> Result<(), PlaybackRuntimeSubmitError> {
        self.commands
            .send(PlaybackRuntimeCommand::SetVolume { volume })
            .map_err(map_send_error)
    }

    /// Submit a playback-gain update for current and future playback without waiting for the runtime thread.
    pub fn try_set_playback_gain(&self, gain: f32) -> Result<(), PlaybackRuntimeSubmitError> {
        self.try_set_playback_gain_with_normalization(gain, None)
    }

    /// Submit a non-blocking playback-gain update with runtime-owned normalization.
    pub fn try_set_playback_gain_with_normalization(
        &self,
        gain: f32,
        normalization: Option<PlaybackRuntimeGainNormalization>,
    ) -> Result<(), PlaybackRuntimeSubmitError> {
        self.commands
            .send(PlaybackRuntimeCommand::SetPlaybackGain {
                gain,
                normalization,
            })
            .map_err(map_send_error)
    }

    /// Submit an in-place span-retarget request without waiting for the runtime thread.
    pub fn try_retarget_span(
        &self,
        update: PlaybackRuntimeSpanUpdate,
    ) -> Result<PlaybackRequestId, PlaybackRuntimeSubmitError> {
        let id = self.next_request_id();
        self.commands
            .send(PlaybackRuntimeCommand::RetargetSpan { id, update })
            .map(|()| id)
            .map_err(map_send_error)
    }

    /// Request runtime shutdown without waiting for the runtime thread.
    pub fn try_shutdown(&self) -> Result<(), PlaybackRuntimeSubmitError> {
        self.commands
            .send(PlaybackRuntimeCommand::Shutdown)
            .map_err(map_send_error)
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
    RetargetSpan {
        id: PlaybackRequestId,
        update: PlaybackRuntimeSpanUpdate,
    },
    PollProgress {
        id: PlaybackRequestId,
    },
    SetVolume {
        volume: f32,
    },
    SetPlaybackGain {
        gain: f32,
        normalization: Option<PlaybackRuntimeGainNormalization>,
    },
    Shutdown,
}

trait PlaybackRuntimeExecutor: Send + 'static {
    fn play(
        &mut self,
        request: PlaybackRuntimeRequest,
    ) -> Result<PlaybackRuntimeStartedData, String>;
    fn stop(&mut self) -> Result<(), String>;
    fn retarget_span(&mut self, update: PlaybackRuntimeSpanUpdate) -> Result<f32, String>;
    fn set_volume(&mut self, volume: f32);
    fn set_playback_gain(
        &mut self,
        gain: f32,
        normalization: Option<PlaybackRuntimeGainNormalization>,
    );
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
        self.player
            .set_playback_gain(runtime_playback_gain_for_source(
                request.playback_gain,
                request.playback_gain_normalization,
                &request.source,
            ));
        let output = self.player.output_details().clone();
        request.source.apply_to_player(&mut self.player);
        self.player.set_edit_fade_state(request.edit_fade);
        let playback_start = request
            .mode
            .start_player(&mut self.player, request.metronome)?;
        Ok(PlaybackRuntimeStartedData {
            output,
            playback_start,
        })
    }

    fn stop(&mut self) -> Result<(), String> {
        self.player.stop();
        Ok(())
    }

    fn retarget_span(&mut self, update: PlaybackRuntimeSpanUpdate) -> Result<f32, String> {
        self.player
            .set_playback_gain(runtime_playback_gain_for_player(
                update.playback_gain,
                update.playback_gain_normalization,
                &self.player,
            ));
        if update.looped {
            self.player.retarget_looped_range_with_metronome(
                update.start,
                update.end,
                update.offset,
                update.seek_to_offset,
                update.metronome,
            )?;
        } else {
            self.player.retarget_one_shot_range_with_metronome(
                update.start,
                update.end,
                update.offset,
                update.seek_to_offset,
                update.metronome,
            )?;
        }
        Ok(update
            .offset
            .clamp(update.start.min(update.end), update.start.max(update.end))
            .clamp(0.0, 1.0) as f32)
    }

    fn set_volume(&mut self, volume: f32) {
        self.player.set_volume(volume);
    }

    fn set_playback_gain(
        &mut self,
        gain: f32,
        normalization: Option<PlaybackRuntimeGainNormalization>,
    ) {
        self.player
            .set_playback_gain(runtime_playback_gain_for_player(
                gain,
                normalization,
                &self.player,
            ));
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
    let _ = config;
    let (command_sender, command_receiver) = mpsc::channel();
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
    let mut pending = VecDeque::new();
    while let Some(command) = next_runtime_command(&commands, &mut pending) {
        match coalesce_command(command, &commands, &events, &mut pending) {
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
            CoalescedCommand::RetargetSpan { id, update } => {
                let event = match executor.retarget_span(update) {
                    Ok(_) => PlaybackRuntimeEvent::Progress {
                        id,
                        progress: executor.progress(),
                    },
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
            CoalescedCommand::SetPlaybackGain {
                gain,
                normalization,
            } => {
                executor.set_playback_gain(gain, normalization);
            }
            CoalescedCommand::Shutdown => return,
        }
    }
}

fn next_runtime_command(
    commands: &Receiver<PlaybackRuntimeCommand>,
    pending: &mut VecDeque<PlaybackRuntimeCommand>,
) -> Option<PlaybackRuntimeCommand> {
    pending.pop_front().or_else(|| commands.recv().ok())
}

enum CoalescedCommand {
    Play {
        id: PlaybackRequestId,
        request: PlaybackRuntimeRequest,
    },
    Stop {
        id: PlaybackRequestId,
    },
    RetargetSpan {
        id: PlaybackRequestId,
        update: PlaybackRuntimeSpanUpdate,
    },
    PollProgress {
        id: PlaybackRequestId,
    },
    SetVolume {
        volume: f32,
    },
    SetPlaybackGain {
        gain: f32,
        normalization: Option<PlaybackRuntimeGainNormalization>,
    },
    Shutdown,
}

fn coalesce_command(
    command: PlaybackRuntimeCommand,
    commands: &Receiver<PlaybackRuntimeCommand>,
    events: &mpsc::Sender<PlaybackRuntimeEvent>,
    pending: &mut VecDeque<PlaybackRuntimeCommand>,
) -> CoalescedCommand {
    loop {
        match commands.try_recv() {
            Ok(next) => pending.push_back(next),
            Err(TryRecvError::Empty) => {
                return command_to_coalesced(coalesce_pending_command(command, pending, events));
            }
            Err(TryRecvError::Disconnected) => {
                cancel_command(command, PlaybackRuntimeCancellation::Shutdown, events);
                cancel_pending_commands(pending, PlaybackRuntimeCancellation::Shutdown, events);
                return CoalescedCommand::Shutdown;
            }
        }
    }
}

fn coalesce_pending_command(
    command: PlaybackRuntimeCommand,
    pending: &mut VecDeque<PlaybackRuntimeCommand>,
    events: &mpsc::Sender<PlaybackRuntimeEvent>,
) -> PlaybackRuntimeCommand {
    match command {
        current @ PlaybackRuntimeCommand::Play { .. } => {
            coalesce_play_command(current, pending, events)
        }
        current @ PlaybackRuntimeCommand::PollProgress { .. } => {
            coalesce_repeated_progress_command(current, pending)
        }
        current @ PlaybackRuntimeCommand::RetargetSpan { .. } => {
            coalesce_span_retarget_command(current, pending)
        }
        current @ PlaybackRuntimeCommand::SetVolume { .. } => {
            coalesce_repeated_volume_command(current, pending)
        }
        current @ PlaybackRuntimeCommand::SetPlaybackGain { .. } => {
            coalesce_repeated_playback_gain_command(current, pending)
        }
        PlaybackRuntimeCommand::Shutdown => {
            cancel_pending_commands(pending, PlaybackRuntimeCancellation::Shutdown, events);
            PlaybackRuntimeCommand::Shutdown
        }
        // Stop is an ordering barrier: dropping it lets an old loop keep running
        // while the host app is still loading the replacement sample.
        current @ PlaybackRuntimeCommand::Stop { .. } => current,
    }
}

fn coalesce_play_command(
    mut current: PlaybackRuntimeCommand,
    pending: &mut VecDeque<PlaybackRuntimeCommand>,
    events: &mpsc::Sender<PlaybackRuntimeEvent>,
) -> PlaybackRuntimeCommand {
    loop {
        match pending.front() {
            Some(PlaybackRuntimeCommand::Play { .. }) => {
                let next = pending.pop_front().expect("pending play command");
                cancel_command(current, PlaybackRuntimeCancellation::Superseded, events);
                current = next;
            }
            Some(PlaybackRuntimeCommand::Stop { .. }) => {
                let stop = pending.pop_front().expect("pending stop command");
                cancel_command(current, PlaybackRuntimeCancellation::Stopped, events);
                return stop;
            }
            Some(PlaybackRuntimeCommand::Shutdown) => {
                let shutdown = pending.pop_front().expect("pending shutdown command");
                cancel_command(current, PlaybackRuntimeCancellation::Shutdown, events);
                cancel_pending_commands(pending, PlaybackRuntimeCancellation::Shutdown, events);
                return shutdown;
            }
            Some(PlaybackRuntimeCommand::PollProgress { .. }) => {
                let _ = pending.pop_front();
            }
            Some(PlaybackRuntimeCommand::RetargetSpan { .. })
            | Some(PlaybackRuntimeCommand::SetVolume { .. })
            | Some(PlaybackRuntimeCommand::SetPlaybackGain { .. })
            | None => return current,
        }
    }
}

fn coalesce_span_retarget_command(
    mut current: PlaybackRuntimeCommand,
    pending: &mut VecDeque<PlaybackRuntimeCommand>,
) -> PlaybackRuntimeCommand {
    loop {
        match pending.front() {
            Some(PlaybackRuntimeCommand::RetargetSpan { .. }) => {
                current = pending.pop_front().expect("pending span retarget");
            }
            Some(PlaybackRuntimeCommand::Play { .. }) => {
                return pending.pop_front().expect("pending play command");
            }
            Some(PlaybackRuntimeCommand::Stop { .. }) => {
                return pending.pop_front().expect("pending stop command");
            }
            Some(PlaybackRuntimeCommand::Shutdown) => {
                return pending.pop_front().expect("pending shutdown command");
            }
            Some(PlaybackRuntimeCommand::PollProgress { .. }) => {
                let _ = pending.pop_front();
            }
            Some(PlaybackRuntimeCommand::SetVolume { .. }) | None => return current,
            Some(PlaybackRuntimeCommand::SetPlaybackGain { .. }) => return current,
        }
    }
}

fn coalesce_repeated_progress_command(
    mut current: PlaybackRuntimeCommand,
    pending: &mut VecDeque<PlaybackRuntimeCommand>,
) -> PlaybackRuntimeCommand {
    while matches!(
        pending.front(),
        Some(PlaybackRuntimeCommand::PollProgress { .. })
    ) {
        current = pending.pop_front().expect("pending progress command");
    }
    current
}

fn coalesce_repeated_volume_command(
    mut current: PlaybackRuntimeCommand,
    pending: &mut VecDeque<PlaybackRuntimeCommand>,
) -> PlaybackRuntimeCommand {
    while matches!(
        pending.front(),
        Some(PlaybackRuntimeCommand::SetVolume { .. })
    ) {
        current = pending.pop_front().expect("pending volume command");
    }
    current
}

fn coalesce_repeated_playback_gain_command(
    mut current: PlaybackRuntimeCommand,
    pending: &mut VecDeque<PlaybackRuntimeCommand>,
) -> PlaybackRuntimeCommand {
    while matches!(
        pending.front(),
        Some(PlaybackRuntimeCommand::SetPlaybackGain { .. })
    ) {
        current = pending.pop_front().expect("pending playback gain command");
    }
    current
}

fn command_to_coalesced(command: PlaybackRuntimeCommand) -> CoalescedCommand {
    match command {
        PlaybackRuntimeCommand::Play { id, request } => CoalescedCommand::Play { id, request },
        PlaybackRuntimeCommand::Stop { id } => CoalescedCommand::Stop { id },
        PlaybackRuntimeCommand::RetargetSpan { id, update } => {
            CoalescedCommand::RetargetSpan { id, update }
        }
        PlaybackRuntimeCommand::PollProgress { id } => CoalescedCommand::PollProgress { id },
        PlaybackRuntimeCommand::SetVolume { volume } => CoalescedCommand::SetVolume { volume },
        PlaybackRuntimeCommand::SetPlaybackGain {
            gain,
            normalization,
        } => CoalescedCommand::SetPlaybackGain {
            gain,
            normalization,
        },
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

fn cancel_pending_commands(
    pending: &mut VecDeque<PlaybackRuntimeCommand>,
    reason: PlaybackRuntimeCancellation,
    events: &mpsc::Sender<PlaybackRuntimeEvent>,
) {
    while let Some(command) = pending.pop_front() {
        cancel_command(command, reason, events);
    }
}

fn send_cancelled(
    id: PlaybackRequestId,
    reason: PlaybackRuntimeCancellation,
    events: &mpsc::Sender<PlaybackRuntimeEvent>,
) {
    let _ = events.send(PlaybackRuntimeEvent::Cancelled { id, reason });
}

fn map_send_error(_error: SendError<PlaybackRuntimeCommand>) -> PlaybackRuntimeSubmitError {
    PlaybackRuntimeSubmitError::Closed
}

fn runtime_playback_gain_for_source(
    base_gain: f32,
    normalization: Option<PlaybackRuntimeGainNormalization>,
    source: &PlaybackRuntimeSource,
) -> f32 {
    let Some(normalization) = normalization else {
        return sanitize_playback_gain(base_gain);
    };
    let normalized = normalized_gain_for_runtime_source(source, normalization).unwrap_or(1.0);
    sanitize_playback_gain(base_gain * normalized)
}

fn runtime_playback_gain_for_player(
    base_gain: f32,
    normalization: Option<PlaybackRuntimeGainNormalization>,
    player: &AudioPlayer,
) -> f32 {
    let Some(normalization) = normalization else {
        return sanitize_playback_gain(base_gain);
    };
    let normalized = normalized_gain_for_player(player, normalization).unwrap_or(1.0);
    sanitize_playback_gain(base_gain * normalized)
}

fn sanitize_playback_gain(gain: f32) -> f32 {
    if gain.is_finite() && gain > 0.0 {
        gain
    } else {
        1.0
    }
}

fn normalized_gain_for_runtime_source(
    source: &PlaybackRuntimeSource,
    normalization: PlaybackRuntimeGainNormalization,
) -> Option<f32> {
    match source {
        PlaybackRuntimeSource::DecodedSamples {
            samples, channels, ..
        } => normalized_gain_for_interleaved_span(
            samples,
            *channels,
            normalization.start,
            normalization.end,
        ),
        PlaybackRuntimeSource::InterleavedF32File {
            path,
            sample_count,
            channels,
            ..
        } => normalized_gain_for_interleaved_f32_file_span(
            path,
            (*sample_count).try_into().ok()?,
            *channels,
            normalization.start,
            normalization.end,
        ),
        PlaybackRuntimeSource::AudioBytes { .. } | PlaybackRuntimeSource::AudioFile { .. } => None,
    }
}

fn normalized_gain_for_player(
    player: &AudioPlayer,
    normalization: PlaybackRuntimeGainNormalization,
) -> Option<f32> {
    let channels = usize::from(player.track_channels.unwrap_or(1)).max(1);
    if let Some(samples) = player.playback_samples.as_ref() {
        return normalized_gain_for_interleaved_span(
            samples,
            channels,
            normalization.start,
            normalization.end,
        );
    }
    match player.current_audio.as_ref()? {
        super::AudioPlaybackSource::InterleavedF32File { path, sample_count } => {
            normalized_gain_for_interleaved_f32_file_span(
                path,
                (*sample_count).try_into().ok()?,
                channels,
                normalization.start,
                normalization.end,
            )
        }
        super::AudioPlaybackSource::Bytes(_) | super::AudioPlaybackSource::File(_) => None,
    }
}

fn normalized_gain_for_interleaved_span(
    samples: &[f32],
    channels: usize,
    start: f32,
    end: f32,
) -> Option<f32> {
    let bounds = interleaved_span_sample_bounds(samples.len(), channels, start, end)?;
    let peak = samples[bounds]
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    Some(normalized_gain_from_peak(peak))
}

fn normalized_gain_for_interleaved_f32_file_span(
    path: &Path,
    sample_count: usize,
    channels: usize,
    start: f32,
    end: f32,
) -> Option<f32> {
    let bounds = interleaved_span_sample_bounds(sample_count, channels, start, end)?;
    let mut reader = BufReader::new(File::open(path).ok()?);
    let byte_offset = bounds.start.checked_mul(F32_SAMPLE_BYTES)? as u64;
    reader.seek(SeekFrom::Start(byte_offset)).ok()?;

    let mut remaining = bounds.end.saturating_sub(bounds.start);
    let mut bytes = vec![0_u8; NORMALIZED_GAIN_READ_SAMPLES * F32_SAMPLE_BYTES];
    let mut peak = 0.0_f32;
    while remaining > 0 {
        let samples_to_read = remaining.min(NORMALIZED_GAIN_READ_SAMPLES);
        let byte_len = samples_to_read * F32_SAMPLE_BYTES;
        reader.read_exact(&mut bytes[..byte_len]).ok()?;
        for sample in bytes[..byte_len].chunks_exact(F32_SAMPLE_BYTES) {
            peak = peak.max(f32::from_le_bytes(sample.try_into().ok()?).abs());
        }
        remaining -= samples_to_read;
    }
    Some(normalized_gain_from_peak(peak))
}

fn interleaved_span_sample_bounds(
    sample_count: usize,
    channels: usize,
    start: f32,
    end: f32,
) -> Option<Range<usize>> {
    if sample_count == 0 || !start.is_finite() || !end.is_finite() {
        return None;
    }
    let channels = channels.max(1);
    let total_frames = sample_count / channels;
    if total_frames == 0 {
        return None;
    }
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let start_frame = (start.clamp(0.0, 1.0) * total_frames as f32).floor() as usize;
    let mut end_frame = (end.clamp(0.0, 1.0) * total_frames as f32).ceil() as usize;
    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(total_frames);
    }
    let start_idx = start_frame.saturating_mul(channels);
    let end_idx = end_frame.saturating_mul(channels).min(sample_count);
    (start_idx < end_idx).then_some(start_idx..end_idx)
}

fn normalized_gain_from_peak(peak: f32) -> f32 {
    if peak.is_finite() && peak > f32::EPSILON {
        1.0 / peak
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    enum FakeOutcome {
        Started(f32),
        Failed(&'static str),
    }

    struct FakeExecutor {
        outcomes: Vec<FakeOutcome>,
        played: Arc<Mutex<Vec<PlaybackRuntimeMode>>>,
        retargeted: Arc<Mutex<Vec<PlaybackRuntimeSpanUpdate>>>,
        stopped: Arc<Mutex<usize>>,
    }

    impl FakeExecutor {
        fn new(outcomes: Vec<FakeOutcome>) -> Self {
            Self {
                outcomes,
                played: Arc::new(Mutex::new(Vec::new())),
                retargeted: Arc::new(Mutex::new(Vec::new())),
                stopped: Arc::new(Mutex::new(0)),
            }
        }

        fn played(&self) -> Arc<Mutex<Vec<PlaybackRuntimeMode>>> {
            Arc::clone(&self.played)
        }

        fn retargeted(&self) -> Arc<Mutex<Vec<PlaybackRuntimeSpanUpdate>>> {
            Arc::clone(&self.retargeted)
        }
    }

    struct BlockingExecutor {
        entered_tx: mpsc::Sender<()>,
        release_rx: mpsc::Receiver<()>,
        blocked_once: bool,
        played: Arc<Mutex<Vec<PlaybackRuntimeMode>>>,
    }

    impl BlockingExecutor {
        fn new(entered_tx: mpsc::Sender<()>, release_rx: mpsc::Receiver<()>) -> Self {
            Self {
                entered_tx,
                release_rx,
                blocked_once: false,
                played: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn played(&self) -> Arc<Mutex<Vec<PlaybackRuntimeMode>>> {
            Arc::clone(&self.played)
        }
    }

    impl PlaybackRuntimeExecutor for BlockingExecutor {
        fn play(
            &mut self,
            request: PlaybackRuntimeRequest,
        ) -> Result<PlaybackRuntimeStartedData, String> {
            self.played.lock().expect("played lock").push(request.mode);
            if !self.blocked_once {
                self.blocked_once = true;
                let _ = self.entered_tx.send(());
                self.release_rx.recv().expect("release blocking executor");
            }
            Ok(PlaybackRuntimeStartedData {
                output: ResolvedOutput::default(),
                playback_start: match request.mode {
                    PlaybackRuntimeMode::OneShot { start, .. } => start as f32,
                    PlaybackRuntimeMode::Looped { offset, .. } => offset as f32,
                },
            })
        }

        fn stop(&mut self) -> Result<(), String> {
            Ok(())
        }

        fn retarget_span(&mut self, _update: PlaybackRuntimeSpanUpdate) -> Result<f32, String> {
            Ok(0.0)
        }

        fn set_volume(&mut self, _volume: f32) {}

        fn set_playback_gain(
            &mut self,
            _gain: f32,
            _normalization: Option<PlaybackRuntimeGainNormalization>,
        ) {
        }

        fn progress(&mut self) -> PlaybackRuntimeProgress {
            PlaybackRuntimeProgress::default()
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

        fn retarget_span(&mut self, update: PlaybackRuntimeSpanUpdate) -> Result<f32, String> {
            self.retargeted
                .lock()
                .expect("retargeted lock")
                .push(update);
            Ok(update.offset as f32)
        }

        fn set_volume(&mut self, _volume: f32) {}

        fn set_playback_gain(
            &mut self,
            _gain: f32,
            _normalization: Option<PlaybackRuntimeGainNormalization>,
        ) {
        }

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
    fn coalescing_keeps_stop_before_progress_poll() {
        let stop_id = PlaybackRequestId(1);
        let poll_id = PlaybackRequestId(2);
        let (coalesced, pending, events) = coalesce_for_test(
            PlaybackRuntimeCommand::Stop { id: stop_id },
            vec![PlaybackRuntimeCommand::PollProgress { id: poll_id }],
        );

        assert!(matches!(
            coalesced,
            CoalescedCommand::Stop { id } if id == stop_id
        ));
        assert!(matches!(
            pending.as_slice(),
            [PlaybackRuntimeCommand::PollProgress { id }] if *id == poll_id
        ));
        assert!(events.is_empty());
    }

    #[test]
    fn coalescing_keeps_stop_before_following_play() {
        let stop_id = PlaybackRequestId(1);
        let play_id = PlaybackRequestId(2);
        let (coalesced, pending, events) = coalesce_for_test(
            PlaybackRuntimeCommand::Stop { id: stop_id },
            vec![play_command(play_id, 0.25)],
        );

        assert!(matches!(
            coalesced,
            CoalescedCommand::Stop { id } if id == stop_id
        ));
        assert!(matches!(
            pending.as_slice(),
            [PlaybackRuntimeCommand::Play { id, .. }] if *id == play_id
        ));
        assert!(events.is_empty());
    }

    #[test]
    fn coalescing_play_then_stop_keeps_later_play_pending() {
        let first_play = PlaybackRequestId(1);
        let stop_id = PlaybackRequestId(2);
        let second_play = PlaybackRequestId(3);
        let (coalesced, pending, events) = coalesce_for_test(
            play_command(first_play, 0.0),
            vec![
                PlaybackRuntimeCommand::Stop { id: stop_id },
                play_command(second_play, 0.5),
            ],
        );

        assert!(matches!(
            coalesced,
            CoalescedCommand::Stop { id } if id == stop_id
        ));
        assert!(matches!(
            pending.as_slice(),
            [PlaybackRuntimeCommand::Play { id, .. }] if *id == second_play
        ));
        assert!(matches!(
            events.as_slice(),
            [PlaybackRuntimeEvent::Cancelled { id, reason }]
                if *id == first_play && *reason == PlaybackRuntimeCancellation::Stopped
        ));
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
    fn playback_runtime_accepts_rapid_play_burst_while_executor_is_busy() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let executor = BlockingExecutor::new(entered_tx, release_rx);
        let played = executor.played();
        let runtime =
            spawn_executor(executor, PlaybackRuntimeConfig::default()).expect("spawn runtime");

        let first = runtime.handle.try_play(test_request(0.0)).expect("first");
        entered_rx.recv().expect("executor entered first play");
        let mut latest = first;
        for index in 1..64 {
            latest = runtime
                .handle
                .try_play(test_request(index as f64 / 100.0))
                .expect("rapid play burst should not be rejected");
        }
        release_tx.send(()).expect("release executor");

        assert!(matches!(
            runtime.events.recv().expect("first started"),
            PlaybackRuntimeEvent::Started(PlaybackRuntimeStarted { id, .. }) if id == first
        ));
        let mut saw_latest = false;
        while let Ok(event) = runtime.events.recv_timeout(Duration::from_secs(1)) {
            match event {
                PlaybackRuntimeEvent::Started(PlaybackRuntimeStarted { id, .. })
                    if id == latest =>
                {
                    saw_latest = true;
                    break;
                }
                PlaybackRuntimeEvent::Cancelled {
                    reason: PlaybackRuntimeCancellation::Superseded,
                    ..
                } => {}
                other => panic!("unexpected playback runtime event: {other:?}"),
            }
        }

        assert!(saw_latest, "latest rapid play request should start");
        assert_eq!(
            played.lock().expect("played").as_slice(),
            &[
                PlaybackRuntimeMode::OneShot {
                    start: 0.0,
                    end: 1.0
                },
                PlaybackRuntimeMode::OneShot {
                    start: 0.63,
                    end: 1.0
                }
            ]
        );
    }

    #[test]
    fn playback_runtime_reports_closed_submit_errors() {
        let (sender, receiver) = mpsc::channel();
        drop(receiver);
        let handle = PlaybackRuntimeHandle {
            commands: sender,
            next_id: Arc::new(AtomicU64::new(1)),
        };

        assert_eq!(
            handle.try_play(test_request(0.0)),
            Err(PlaybackRuntimeSubmitError::Closed)
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

    #[test]
    fn coalescing_span_retarget_keeps_latest_update() {
        let first_id = PlaybackRequestId(1);
        let second_id = PlaybackRequestId(2);
        let (coalesced, pending, events) = coalesce_for_test(
            retarget_command(first_id, 0.1, 0.5),
            vec![retarget_command(second_id, 0.2, 0.7)],
        );

        assert!(matches!(
            coalesced,
            CoalescedCommand::RetargetSpan { id, update }
                if id == second_id && update.start == 0.2 && update.end == 0.7
        ));
        assert!(pending.is_empty());
        assert!(events.is_empty());
    }

    #[test]
    fn playback_runtime_executes_span_retarget() {
        let executor = FakeExecutor::new(vec![]);
        let retargeted = executor.retargeted();
        let runtime =
            spawn_executor(executor, PlaybackRuntimeConfig::default()).expect("spawn runtime");
        let update = span_update(0.2, 0.8, true);

        let id = runtime.handle.try_retarget_span(update).expect("retarget");
        let event = runtime.events.recv().expect("event");

        assert!(matches!(
            event,
            PlaybackRuntimeEvent::Progress {
                id: event_id,
                progress
            } if event_id == id && progress.active
        ));
        assert_eq!(retargeted.lock().expect("retargeted").as_slice(), &[update]);
    }

    #[test]
    fn runtime_normalization_reads_interleaved_f32_file_span() {
        let root = tempfile::tempdir().expect("temp root");
        let path = root.path().join("normalized-runtime.pcm");
        let mut file = File::create(&path).expect("create f32 cache");
        for sample in [
            0.1_f32, 0.1, 0.1, 0.1, 0.25, 0.5, 0.2, 0.2, 0.9, 0.9, 0.9, 0.9, 0.1, 0.1, 0.1, 0.1,
        ] {
            file.write_all(&sample.to_le_bytes()).expect("write sample");
        }
        let source = PlaybackRuntimeSource::InterleavedF32File {
            path,
            sample_count: 16,
            duration: 1.0,
            sample_rate: 48_000,
            channels: 1,
        };

        let gain = runtime_playback_gain_for_source(
            1.0,
            Some(PlaybackRuntimeGainNormalization::new(0.25, 0.5)),
            &source,
        );

        assert!((gain - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn runtime_normalization_uses_decoded_samples_without_file_io() {
        let source = PlaybackRuntimeSource::DecodedSamples {
            audio_bytes: Arc::from([]),
            samples: Arc::from([0.1_f32, -0.25, 0.5, -0.2]),
            duration: 1.0,
            sample_rate: 48_000,
            channels: 1,
        };

        let gain = runtime_playback_gain_for_source(
            1.0,
            Some(PlaybackRuntimeGainNormalization::new(0.0, 1.0)),
            &source,
        );

        assert!((gain - 2.0).abs() < f32::EPSILON);
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
            playback_gain: 1.0,
            playback_gain_normalization: None,
            edit_fade: None,
            metronome: None,
        }
    }

    fn play_command(id: PlaybackRequestId, start: f64) -> PlaybackRuntimeCommand {
        PlaybackRuntimeCommand::Play {
            id,
            request: test_request(start),
        }
    }

    fn retarget_command(id: PlaybackRequestId, start: f64, end: f64) -> PlaybackRuntimeCommand {
        PlaybackRuntimeCommand::RetargetSpan {
            id,
            update: span_update(start, end, true),
        }
    }

    fn span_update(start: f64, end: f64, looped: bool) -> PlaybackRuntimeSpanUpdate {
        PlaybackRuntimeSpanUpdate {
            start,
            end,
            offset: start,
            seek_to_offset: true,
            looped,
            playback_gain: 1.0,
            playback_gain_normalization: None,
            metronome: None,
        }
    }

    fn coalesce_for_test(
        current: PlaybackRuntimeCommand,
        queued: Vec<PlaybackRuntimeCommand>,
    ) -> (
        CoalescedCommand,
        Vec<PlaybackRuntimeCommand>,
        Vec<PlaybackRuntimeEvent>,
    ) {
        let (command_sender, command_receiver) = mpsc::sync_channel(queued.len().max(1));
        for command in queued {
            command_sender.try_send(command).expect("queue command");
        }
        let (event_sender, event_receiver) = mpsc::channel();
        let mut pending = VecDeque::new();
        let coalesced = coalesce_command(current, &command_receiver, &event_sender, &mut pending);
        let events = event_receiver.try_iter().collect();
        (coalesced, pending.into_iter().collect(), events)
    }
}
