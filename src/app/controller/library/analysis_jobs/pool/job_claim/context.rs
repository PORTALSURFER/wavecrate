use crate::app::controller::jobs::JobMessageSender;
use crate::app::controller::library::analysis_jobs::pool::progress_cache::ProgressCache;
use crate::app::controller::library::analysis_jobs::wakeup::ClaimWakeup;
use crate::gui::repaint::SharedRepaintSignal;
use crate::sample_sources::SourceId;
use std::collections::HashSet;
use std::sync::{
    Arc, Mutex, RwLock,
    atomic::{AtomicBool, AtomicU32},
};

use super::DecodedQueue;

/// Shared inputs for one decoder worker thread.
pub(crate) struct DecoderWorkerContext {
    pub(crate) decode_queue: Arc<DecodedQueue>,
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) shutdown: Arc<AtomicBool>,
    pub(crate) pause_claiming: Arc<AtomicBool>,
    pub(crate) allowed_source_ids: Arc<RwLock<Option<HashSet<SourceId>>>>,
    pub(crate) max_duration_bits: Arc<AtomicU32>,
    pub(crate) analysis_sample_rate: Arc<AtomicU32>,
    pub(crate) decode_queue_target: usize,
    pub(crate) claim_wakeup: Arc<ClaimWakeup>,
    pub(crate) reset_done: Arc<Mutex<HashSet<std::path::PathBuf>>>,
}

/// Shared inputs for one compute worker thread.
pub(crate) struct ComputeWorkerContext {
    pub(crate) tx: JobMessageSender,
    pub(crate) signal: Arc<SharedRepaintSignal>,
    pub(crate) decode_queue: Arc<DecodedQueue>,
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) shutdown: Arc<AtomicBool>,
    pub(crate) use_cache: Arc<AtomicBool>,
    pub(crate) allowed_source_ids: Arc<RwLock<Option<HashSet<SourceId>>>>,
    pub(crate) max_duration_bits: Arc<AtomicU32>,
    pub(crate) analysis_sample_rate: Arc<AtomicU32>,
    pub(crate) analysis_version_override: Arc<std::sync::RwLock<Option<String>>>,
    pub(crate) progress_cache: Arc<RwLock<ProgressCache>>,
    pub(crate) progress_wakeup: Arc<super::super::job_progress::ProgressPollerWakeup>,
}
