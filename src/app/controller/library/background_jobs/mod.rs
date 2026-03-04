mod analysis;
/// Background-job polling orchestration and message dispatch helpers.
mod polling;
mod progress;
mod scan;
mod similarity;
mod updates;

pub(super) use super::jobs::JobMessage;
pub(super) use super::*;
pub(super) use crate::app::controller::playback::audio_loader::AudioLoadResult;
pub(super) use crate::app::controller::playback::recording::waveform_loader::RecordingWaveformUpdate;
pub(super) use crate::app::controller::state::audio::AudioLoadIntent;
pub(super) use crate::app::state::ProgressTaskKind;
pub(super) use std::sync::atomic::Ordering;
pub(super) use std::time::Instant;
pub(super) use trash_move::TrashMoveMessage;
