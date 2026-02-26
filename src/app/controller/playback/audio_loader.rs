use super::*;
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::waveform::{DecodedWaveform, WaveformRenderer};
use std::{
    fs,
    mem::size_of,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, Sender},
    },
    thread,
    time::{Duration, Instant},
};

const AUDIO_LOADER_POLL_INTERVAL: Duration = Duration::from_millis(200);
const HOTPATH_TELEMETRY_ENV: &str = "SEMPAL_HOTPATH_TELEMETRY";
const AUDIO_LOADER_TELEMETRY_LOG_EVERY: u64 = 128;
static AUDIO_LOADER_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();
static AUDIO_LOADER_JOBS_RECEIVED: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_JOBS_COALESCED: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_JOBS_COMPLETED: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_JOBS_FAILED: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_DROPPED_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_DISPATCH: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_PRE_IO: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_POST_IO: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_POST_DECODE: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_POST_STRETCH: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_POST_TRANSIENTS: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_PRE_SEND: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_IO_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_DECODE_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STRETCH_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_TRANSIENT_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_READ_BYTES_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_OUTPUT_BYTES_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_ALLOC_ESTIMATE_BYTES_TOTAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum StaleDropStage {
    Dispatch,
    PreIo,
    PostIo,
    PostDecode,
    PostStretch,
    PostTransients,
    PreSend,
}

fn parse_hotpath_telemetry_enabled(value: &str) -> bool {
    let normalized = value.trim();
    normalized == "1"
        || normalized.eq_ignore_ascii_case("true")
        || normalized.eq_ignore_ascii_case("on")
        || normalized.eq_ignore_ascii_case("yes")
}

fn audio_loader_telemetry_enabled() -> bool {
    *AUDIO_LOADER_TELEMETRY_ENABLED.get_or_init(|| {
        std::env::var(HOTPATH_TELEMETRY_ENV)
            .ok()
            .is_some_and(|value| parse_hotpath_telemetry_enabled(&value))
    })
}

fn saturating_add_duration_ns(counter: &AtomicU64, duration: Duration) {
    let dur_ns = duration.as_nanos().min(u64::MAX as u128) as u64;
    counter.fetch_add(dur_ns, Ordering::Relaxed);
}

fn record_audio_loader_duration(counter: &AtomicU64, duration: Duration) {
    if !audio_loader_telemetry_enabled() {
        return;
    }
    saturating_add_duration_ns(counter, duration);
}

fn record_audio_loader_bytes(counter: &AtomicU64, bytes: usize) {
    if !audio_loader_telemetry_enabled() || bytes == 0 {
        return;
    }
    counter.fetch_add(bytes.min(u64::MAX as usize) as u64, Ordering::Relaxed);
}

fn record_audio_loader_stale(stage: StaleDropStage) {
    if !audio_loader_telemetry_enabled() {
        return;
    }
    let sample_tick = AUDIO_LOADER_STALE_DROPPED_TOTAL.fetch_add(1, Ordering::Relaxed) + 1;
    match stage {
        StaleDropStage::Dispatch => {
            AUDIO_LOADER_STALE_DISPATCH.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PreIo => {
            AUDIO_LOADER_STALE_PRE_IO.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PostIo => {
            AUDIO_LOADER_STALE_POST_IO.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PostDecode => {
            AUDIO_LOADER_STALE_POST_DECODE.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PostStretch => {
            AUDIO_LOADER_STALE_POST_STRETCH.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PostTransients => {
            AUDIO_LOADER_STALE_POST_TRANSIENTS.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PreSend => {
            AUDIO_LOADER_STALE_PRE_SEND.fetch_add(1, Ordering::Relaxed);
        }
    }
    maybe_emit_audio_loader_telemetry(sample_tick);
}

fn stale_and_record(request_id: u64, latest_request_id: &AtomicU64, stage: StaleDropStage) -> bool {
    if is_stale_request(request_id, latest_request_id) {
        record_audio_loader_stale(stage);
        return true;
    }
    false
}

fn maybe_emit_audio_loader_telemetry(sample_tick: u64) {
    if !audio_loader_telemetry_enabled()
        || sample_tick == 0
        || sample_tick % AUDIO_LOADER_TELEMETRY_LOG_EVERY != 0
    {
        return;
    }

    let jobs_received = AUDIO_LOADER_JOBS_RECEIVED.load(Ordering::Relaxed);
    let jobs_coalesced = AUDIO_LOADER_JOBS_COALESCED.load(Ordering::Relaxed);
    let jobs_completed = AUDIO_LOADER_JOBS_COMPLETED.load(Ordering::Relaxed);
    let jobs_failed = AUDIO_LOADER_JOBS_FAILED.load(Ordering::Relaxed);
    let stale_total = AUDIO_LOADER_STALE_DROPPED_TOTAL.load(Ordering::Relaxed);
    let stale_dispatch = AUDIO_LOADER_STALE_DISPATCH.load(Ordering::Relaxed);
    let stale_pre_io = AUDIO_LOADER_STALE_PRE_IO.load(Ordering::Relaxed);
    let stale_post_io = AUDIO_LOADER_STALE_POST_IO.load(Ordering::Relaxed);
    let stale_post_decode = AUDIO_LOADER_STALE_POST_DECODE.load(Ordering::Relaxed);
    let stale_post_stretch = AUDIO_LOADER_STALE_POST_STRETCH.load(Ordering::Relaxed);
    let stale_post_transients = AUDIO_LOADER_STALE_POST_TRANSIENTS.load(Ordering::Relaxed);
    let stale_pre_send = AUDIO_LOADER_STALE_PRE_SEND.load(Ordering::Relaxed);
    let io_ns_total = AUDIO_LOADER_IO_NS_TOTAL.load(Ordering::Relaxed);
    let decode_ns_total = AUDIO_LOADER_DECODE_NS_TOTAL.load(Ordering::Relaxed);
    let stretch_ns_total = AUDIO_LOADER_STRETCH_NS_TOTAL.load(Ordering::Relaxed);
    let transient_ns_total = AUDIO_LOADER_TRANSIENT_NS_TOTAL.load(Ordering::Relaxed);
    let read_bytes_total = AUDIO_LOADER_READ_BYTES_TOTAL.load(Ordering::Relaxed);
    let output_bytes_total = AUDIO_LOADER_OUTPUT_BYTES_TOTAL.load(Ordering::Relaxed);
    let alloc_estimate_total = AUDIO_LOADER_ALLOC_ESTIMATE_BYTES_TOTAL.load(Ordering::Relaxed);
    let completed_nonzero = jobs_completed.max(1);

    tracing::info!(
        target: "perf::hotpath",
        module = "audio_loader",
        jobs_received,
        jobs_coalesced,
        jobs_completed,
        jobs_failed,
        stale_total,
        stale_dispatch,
        stale_pre_io,
        stale_post_io,
        stale_post_decode,
        stale_post_stretch,
        stale_post_transients,
        stale_pre_send,
        avg_io_ms = io_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        avg_decode_ms = decode_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        avg_stretch_ms = stretch_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        avg_transient_ms = transient_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        read_bytes_total,
        output_bytes_total,
        alloc_estimate_total,
        "Audio loader telemetry snapshot"
    );
}

pub(crate) struct AudioLoadJob {
    pub request_id: u64,
    pub source_id: SourceId,
    pub root: PathBuf,
    pub relative_path: PathBuf,
    pub stretch_ratio: Option<f64>,
}

#[derive(Debug)]
pub(crate) struct AudioLoadOutcome {
    pub decoded: Arc<DecodedWaveform>,
    pub bytes: Arc<[u8]>,
    pub metadata: FileMetadata,
    pub transients: Arc<[f32]>,
    pub stretched: bool,
}

#[derive(Debug)]
pub(crate) enum AudioLoadError {
    Missing(String),
    Failed(String),
}

#[derive(Debug)]
pub(crate) struct AudioLoadResult {
    pub request_id: u64,
    pub source_id: SourceId,
    pub relative_path: PathBuf,
    pub result: Result<AudioLoadOutcome, AudioLoadError>,
}

/// Join handle and shutdown signal for the audio loader thread.
pub(crate) struct AudioLoaderHandle {
    shutdown: Arc<AtomicBool>,
    latest_request_id: Arc<AtomicU64>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl AudioLoaderHandle {
    /// Publish the most recent queued request id so stale decode work can abort early.
    pub(crate) fn publish_latest_request_id(&self, request_id: u64) {
        self.latest_request_id.store(request_id, Ordering::Relaxed);
    }

    /// Signal the loader thread to exit and wait for it to finish.
    pub(crate) fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Spawn the audio loader worker and return its job channel plus shutdown handle.
pub(crate) fn spawn_audio_loader(
    renderer: WaveformRenderer,
) -> (
    Sender<AudioLoadJob>,
    Receiver<AudioLoadResult>,
    AudioLoaderHandle,
) {
    let (tx, rx) = std::sync::mpsc::channel::<AudioLoadJob>();
    let (result_tx, result_rx) = std::sync::mpsc::channel::<AudioLoadResult>();
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_worker = Arc::clone(&shutdown);
    let latest_request_id = Arc::new(AtomicU64::new(0));
    let latest_request_id_worker = Arc::clone(&latest_request_id);
    let handle = thread::spawn(move || {
        while !shutdown_worker.load(Ordering::Relaxed) {
            match rx.recv_timeout(AUDIO_LOADER_POLL_INTERVAL) {
                Ok(job) => {
                    if audio_loader_telemetry_enabled() {
                        let sample_tick =
                            AUDIO_LOADER_JOBS_RECEIVED.fetch_add(1, Ordering::Relaxed) + 1;
                        maybe_emit_audio_loader_telemetry(sample_tick);
                    }
                    let job = drain_to_latest_job(job, &rx);
                    let outcome = load_audio(&renderer, &job, &latest_request_id_worker);
                    let AudioLoadExecution::Completed(result) = outcome else {
                        continue;
                    };
                    if stale_and_record(
                        job.request_id,
                        &latest_request_id_worker,
                        StaleDropStage::PreSend,
                    ) {
                        continue;
                    }
                    if audio_loader_telemetry_enabled() {
                        let sample_tick = match &result {
                            Ok(_) => {
                                AUDIO_LOADER_JOBS_COMPLETED.fetch_add(1, Ordering::Relaxed) + 1
                            }
                            Err(_) => AUDIO_LOADER_JOBS_FAILED.fetch_add(1, Ordering::Relaxed) + 1,
                        };
                        maybe_emit_audio_loader_telemetry(sample_tick);
                    }
                    let _ = result_tx.send(AudioLoadResult {
                        request_id: job.request_id,
                        source_id: job.source_id.clone(),
                        relative_path: job.relative_path.clone(),
                        result,
                    });
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
    (
        tx,
        result_rx,
        AudioLoaderHandle {
            shutdown,
            latest_request_id,
            join_handle: Some(handle),
        },
    )
}

enum AudioLoadExecution {
    Completed(Result<AudioLoadOutcome, AudioLoadError>),
    DroppedStale,
}

fn drain_to_latest_job(mut latest_job: AudioLoadJob, rx: &Receiver<AudioLoadJob>) -> AudioLoadJob {
    let mut coalesced = 0u64;
    while let Ok(next_job) = rx.try_recv() {
        latest_job = next_job;
        coalesced = coalesced.saturating_add(1);
    }
    if audio_loader_telemetry_enabled() && coalesced > 0 {
        let sample_tick = AUDIO_LOADER_JOBS_COALESCED.fetch_add(coalesced, Ordering::Relaxed) + 1;
        maybe_emit_audio_loader_telemetry(sample_tick);
    }
    latest_job
}

fn is_stale_request(request_id: u64, latest_request_id: &AtomicU64) -> bool {
    let latest = latest_request_id.load(Ordering::Relaxed);
    latest != 0 && latest != request_id
}

fn load_audio(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
) -> AudioLoadExecution {
    if stale_and_record(job.request_id, latest_request_id, StaleDropStage::Dispatch) {
        return AudioLoadExecution::DroppedStale;
    }
    let result = load_audio_inner(renderer, job, latest_request_id);
    match result {
        Ok(Some(outcome)) => AudioLoadExecution::Completed(Ok(outcome)),
        Ok(None) => AudioLoadExecution::DroppedStale,
        Err(err) => AudioLoadExecution::Completed(Err(err)),
    }
}

fn load_audio_inner(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
) -> Result<Option<AudioLoadOutcome>, AudioLoadError> {
    ensure_safe_relative_path(&job.relative_path)?;
    if stale_and_record(job.request_id, latest_request_id, StaleDropStage::PreIo) {
        return Ok(None);
    }

    let io_start = audio_loader_telemetry_enabled().then(Instant::now);
    let full_path = job.root.join(&job.relative_path);
    let metadata = fs::metadata(&full_path).map_err(|err| {
        let missing = err.kind() == std::io::ErrorKind::NotFound;
        if missing {
            AudioLoadError::Missing(format!("File missing: {} ({err})", full_path.display()))
        } else {
            AudioLoadError::Failed(format!(
                "Failed to read metadata for {}: {err}",
                full_path.display()
            ))
        }
    })?;
    let bytes = fs::read(&full_path).map_err(|err| {
        let missing = err.kind() == std::io::ErrorKind::NotFound;
        if missing {
            AudioLoadError::Missing(format!("File missing: {} ({err})", full_path.display()))
        } else {
            AudioLoadError::Failed(format!("Failed to read {}: {err}", full_path.display()))
        }
    })?;
    record_audio_loader_bytes(&AUDIO_LOADER_READ_BYTES_TOTAL, bytes.len());
    if stale_and_record(job.request_id, latest_request_id, StaleDropStage::PostIo) {
        return Ok(None);
    }
    let bytes = crate::wav_sanitize::sanitize_wav_bytes(bytes);
    if let Some(start) = io_start {
        record_audio_loader_duration(&AUDIO_LOADER_IO_NS_TOTAL, start.elapsed());
    }
    let modified_ns = metadata
        .modified()
        .map_err(|err| {
            AudioLoadError::Failed(format!(
                "Missing modified time for {}: {err}",
                full_path.display()
            ))
        })?
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_err(|_| {
            AudioLoadError::Failed(format!(
                "File modified time is before epoch: {}",
                full_path.display()
            ))
        })?
        .as_nanos() as i64;
    if stale_and_record(job.request_id, latest_request_id, StaleDropStage::PostIo) {
        return Ok(None);
    }
    let decode_start = audio_loader_telemetry_enabled().then(Instant::now);
    let mut decoded = Arc::new(
        renderer
            .decode_from_bytes(&bytes)
            .map_err(|err| AudioLoadError::Failed(err.to_string()))?,
    );
    if let Some(start) = decode_start {
        record_audio_loader_duration(&AUDIO_LOADER_DECODE_NS_TOTAL, start.elapsed());
    }
    if stale_and_record(
        job.request_id,
        latest_request_id,
        StaleDropStage::PostDecode,
    ) {
        return Ok(None);
    }

    let mut stretched = false;
    let mut final_bytes: Arc<[u8]> = bytes.into();

    if let Some(ratio) = job.stretch_ratio {
        if stale_and_record(
            job.request_id,
            latest_request_id,
            StaleDropStage::PostDecode,
        ) {
            return Ok(None);
        }
        let stretch_start = audio_loader_telemetry_enabled().then(Instant::now);
        let wsola = crate::audio::Wsola::new(decoded.sample_rate);
        let stretched_samples = wsola.stretch(&decoded.samples, decoded.channel_count(), ratio);
        record_audio_loader_bytes(
            &AUDIO_LOADER_ALLOC_ESTIMATE_BYTES_TOTAL,
            stretched_samples.len().saturating_mul(size_of::<f32>()),
        );
        if stale_and_record(
            job.request_id,
            latest_request_id,
            StaleDropStage::PostStretch,
        ) {
            return Ok(None);
        }
        match crate::app::controller::playback::audio_samples::wav_bytes_from_samples(
            &stretched_samples,
            decoded.sample_rate,
            decoded.channels,
        ) {
            Ok(b) => {
                final_bytes = b.into();
                stretched = true;
                // Decode the stretched bytes to get the correct duration and cache token
                if let Ok(d) = renderer.decode_from_bytes(&final_bytes) {
                    decoded = Arc::new(d);
                }
            }
            Err(err) => {
                tracing::warn!("Failed to stretch audio in background: {err}");
            }
        }
        if let Some(start) = stretch_start {
            record_audio_loader_duration(&AUDIO_LOADER_STRETCH_NS_TOTAL, start.elapsed());
        }
    }

    if stale_and_record(
        job.request_id,
        latest_request_id,
        StaleDropStage::PostStretch,
    ) {
        return Ok(None);
    }
    let transient_start = audio_loader_telemetry_enabled().then(Instant::now);
    let transients: Arc<[f32]> = crate::waveform::transients::detect_transients(
        decoded.as_ref(),
        crate::app::controller::library::wavs::waveform_rendering::DEFAULT_TRANSIENT_SENSITIVITY,
    )
    .into();
    if let Some(start) = transient_start {
        record_audio_loader_duration(&AUDIO_LOADER_TRANSIENT_NS_TOTAL, start.elapsed());
    }
    if stale_and_record(
        job.request_id,
        latest_request_id,
        StaleDropStage::PostTransients,
    ) {
        return Ok(None);
    }
    record_audio_loader_bytes(&AUDIO_LOADER_OUTPUT_BYTES_TOTAL, final_bytes.len());
    record_audio_loader_bytes(
        &AUDIO_LOADER_ALLOC_ESTIMATE_BYTES_TOTAL,
        final_bytes
            .len()
            .saturating_add(decoded.samples.len().saturating_mul(size_of::<f32>()))
            .saturating_add(transients.len().saturating_mul(size_of::<f32>())),
    );

    Ok(Some(AudioLoadOutcome {
        decoded,
        bytes: final_bytes,
        metadata: FileMetadata {
            file_size: metadata.len(),
            modified_ns,
        },
        transients,
        stretched,
    }))
}

fn ensure_safe_relative_path(path: &Path) -> Result<(), AudioLoadError> {
    let mut saw_component = false;
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => {
                saw_component = true;
            }
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AudioLoadError::Failed(format!(
                    "Invalid relative path: {}",
                    path.display()
                )));
            }
        }
    }
    if !saw_component {
        return Err(AudioLoadError::Failed(format!(
            "Invalid relative path: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AudioLoadJob, drain_to_latest_job, ensure_safe_relative_path, is_stale_request};
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;

    fn test_job(request_id: u64, relative_path: &str) -> AudioLoadJob {
        AudioLoadJob {
            request_id,
            source_id: crate::sample_sources::SourceId::from_string("source"),
            root: PathBuf::from("/tmp"),
            relative_path: PathBuf::from(relative_path),
            stretch_ratio: None,
        }
    }

    #[test]
    fn ensure_safe_relative_path_rejects_parent_dir() {
        let err = ensure_safe_relative_path(Path::new("../escape.wav")).unwrap_err();
        assert!(matches!(err, super::AudioLoadError::Failed(_)));
    }

    #[test]
    fn ensure_safe_relative_path_accepts_normal_relative_paths() {
        ensure_safe_relative_path(Path::new("folder/./file.wav")).unwrap();
    }

    #[test]
    fn drain_to_latest_job_keeps_most_recent_request() {
        let (tx, rx) = std::sync::mpsc::channel::<AudioLoadJob>();
        tx.send(test_job(2, "two.wav")).unwrap();
        tx.send(test_job(3, "three.wav")).unwrap();
        let drained = drain_to_latest_job(test_job(1, "one.wav"), &rx);
        assert_eq!(drained.request_id, 3);
        assert_eq!(drained.relative_path, Path::new("three.wav"));
    }

    #[test]
    fn stale_request_detection_ignores_zero_and_matches_latest_only() {
        let latest = AtomicU64::new(0);
        assert!(!is_stale_request(1, &latest));
        latest.store(5, std::sync::atomic::Ordering::Relaxed);
        assert!(is_stale_request(4, &latest));
        assert!(!is_stale_request(5, &latest));
    }
}
