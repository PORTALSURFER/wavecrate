mod actions;
mod delegates;
mod helpers;
/// Waveform zoom math and view update helpers.
mod zoom;

pub(crate) use actions::WaveformActions;
pub(crate) use helpers::WaveformController;

use super::*;
use crate::app::state::{FocusContext, WaveformView};
use std::time::{Duration, Instant};
