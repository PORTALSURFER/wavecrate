mod commands;
mod diagnostics;
mod execution;
mod intent;
mod loop_control;
mod policy;
mod progress;
mod random_audition;
mod runtime;
mod span;

use std::time::Duration;

pub(in crate::native_app) const PLAYBACK_START_ACTIVE_SOURCE_GRACE: Duration =
    Duration::from_millis(120);

pub(in crate::native_app) use intent::PlaybackIntent;
#[cfg(test)]
pub(in crate::native_app) use random_audition::{
    RandomAuditionSource, random_audition_span_for_unit,
};
