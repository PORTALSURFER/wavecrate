use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use crate::app::controller::library::wavs;
use crate::app::controller::state::audio::AudioLoadIntent;
use crate::app::state::WaveformView;
use crate::waveform::DecodedWaveform;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

/// Keep test expectations aligned with waveform render supersampling.
const TEST_WAVEFORM_RENDER_SUPERSAMPLE_X: u32 = 4;

mod async_loading;
mod render_behavior;
