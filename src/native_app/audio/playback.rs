mod commands;
mod diagnostics;
mod execution;
mod intent;
mod loop_control;
mod metronome;
mod normalized;
mod planner;
mod policy;
mod progress;
mod random_audition;
mod span;

use std::time::Duration;

pub(in crate::native_app) const PLAYBACK_START_ACTIVE_SOURCE_GRACE: Duration =
    Duration::from_millis(120);

pub(in crate::native_app) use diagnostics::{
    PlayheadFrameDiagnosticsState, playhead_frame_diagnostics_observer_enabled,
};
pub(in crate::native_app) use intent::PlaybackIntent;
pub(in crate::native_app) use planner::{
    ActiveSamplePlaybackPlanState, SamplePlaybackAvailableSources, SamplePlaybackPlan,
    plan_sample_playback,
};
pub(in crate::native_app) use policy::{
    TaggedPlaybackMode, tagged_playback_mode_for_tag, tagged_playback_mode_for_tags,
};
pub(in crate::native_app) use progress::FrameSurfaceRevisionTracker;
pub(in crate::native_app) use random_audition::RandomAuditionUnits;
#[cfg(test)]
pub(in crate::native_app) use random_audition::{
    RandomAuditionSource, random_audition_span_for_units,
};
