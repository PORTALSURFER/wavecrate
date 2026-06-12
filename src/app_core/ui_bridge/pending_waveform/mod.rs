//! Coalesced high-frequency waveform action batching helpers.

mod config;
mod emission;
mod enqueue;
mod planning;
mod preparation;
mod queue_state;

pub(super) use config::immediate_waveform_preview_enabled;
pub(super) use preparation::{LOCAL_MODEL_PULL_FAST_PATH_BURST_LIMIT, PendingModelPullPreparation};
pub(super) use queue_state::PendingWaveformActions;
