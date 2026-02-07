mod actions;
mod delegates;
mod helpers;

pub(crate) use actions::WaveformActions;
pub(crate) use helpers::WaveformController;

use super::*;
use crate::app::state::{FocusContext, WaveformView};
use std::time::{Duration, Instant};
