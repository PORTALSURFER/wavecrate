mod commands;
mod diagnostics;
mod execution;
mod intent;
mod loop_control;
mod policy;
mod progress;
mod random_audition;
mod span;

use std::time::Duration;

pub(in crate::native_app) const PLAYBACK_START_ACTIVE_SOURCE_GRACE: Duration =
    Duration::from_millis(120);

pub(in crate::native_app) use intent::PlaybackIntent;
pub(in crate::native_app) use policy::tagged_playback_mode_for_tags;
#[cfg(test)]
pub(in crate::native_app) use random_audition::{
    RandomAuditionSource, random_audition_span_for_unit,
};
