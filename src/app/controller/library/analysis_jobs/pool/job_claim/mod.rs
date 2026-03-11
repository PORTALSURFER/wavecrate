#[cfg(test)]
use super::progress_cache::ProgressCache;

mod claim;
#[cfg(not(test))]
mod compute_worker;
mod context;
mod db;
#[cfg(not(test))]
mod decoder_worker;
mod dedup;
mod lease;
mod logging;
mod priority;
mod queue;
mod selection;

#[allow(unused_imports)]
pub(crate) use claim::{
    decode_queue_target, decode_worker_count_with_override, worker_count_with_override,
};
#[cfg(not(test))]
pub(crate) use compute_worker::spawn_compute_worker;
#[cfg(not(test))]
pub(crate) use context::{ComputeWorkerContext, DecoderWorkerContext};
#[cfg(not(test))]
pub(crate) use decoder_worker::spawn_decoder_worker;
#[cfg(not(test))]
pub(crate) use queue::DecodeOutcome;
pub(crate) use queue::DecodedQueue;

#[cfg(test)]
mod tests;
