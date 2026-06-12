mod job;
mod latest;
mod pending;
mod primary;
mod result;
mod stages;
mod telemetry;
mod visual;
mod worker;

#[cfg(test)]
mod tests;

pub(crate) use self::job::AudioLoadJob;
pub(crate) use self::latest::AudioLoaderHandle;
pub(crate) use self::result::{
    AudioLoadError, AudioLoadOutcome, AudioLoadResult, AudioTransientResult, AudioVisualResult,
};
pub(crate) use self::worker::spawn_audio_loader;

#[cfg(test)]
use self::latest::{drain_to_latest_job, is_stale_request};
pub(super) use self::pending::PendingTransientCompute;
