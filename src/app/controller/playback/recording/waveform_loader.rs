//! Background worker for recording waveform refresh tasks.

use super::{RECORDING_MAX_FULL_FRAMES, RECORDING_MAX_PEAK_BUCKETS};
use crate::sample_sources::SourceId;
use crate::waveform::{DecodedWaveform, WaveformPeaks};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, OnceLock, mpsc::Receiver};
use std::{fs, thread};
use tracing::warn;

/// Waveform aggregation state, cache map, and peak-analysis helpers.
mod aggregation;
/// WAV decode and rebuild helpers for full/incremental refresh paths.
mod decode;
/// Incremental waveform refresh orchestration.
mod incremental_update;
/// Filesystem IO and result helpers for recording refresh jobs.
mod io;
/// Worker queue and sender/handle lifecycle helpers.
mod queue;
/// Public result/update/error payload types.
mod result;
/// Retained state-cache helpers for incremental recording refreshes.
mod state_cache;
#[cfg(test)]
mod tests;

use self::aggregation::*;
#[cfg(test)]
use self::decode::decode_recording_waveform;
use self::decode::*;
use self::incremental_update::load_recording_waveform;
use self::io::*;
#[cfg(test)]
use self::queue::RecordingWaveformJobQueue;
pub(crate) use self::queue::{
    RecordingWaveformJobSender, RecordingWaveformWorkerHandle, spawn_recording_waveform_loader,
};
pub(crate) use self::result::{
    RecordingWaveformError, RecordingWaveformLoadResult, RecordingWaveformUpdate,
};
#[cfg(test)]
use self::state_cache::clear_recording_state;
use self::state_cache::*;

/// Request data needed to refresh a recording waveform off the UI thread.
#[derive(Clone, Debug)]
pub(crate) struct RecordingWaveformJob {
    pub(crate) request_id: u64,
    pub(crate) source_id: SourceId,
    pub(crate) relative_path: PathBuf,
    pub(crate) absolute_path: PathBuf,
    pub(crate) last_file_len: u64,
    pub(crate) loaded_once: bool,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
}
