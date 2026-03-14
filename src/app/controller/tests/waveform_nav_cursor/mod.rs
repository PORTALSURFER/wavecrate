use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use super::super::*;
use super::common::prepare_browser_sample;
use crate::app::state::FocusContext;
use crate::waveform::DecodedWaveform;
use std::path::Path;
use std::time::{Duration, Instant};

mod cursor;
mod markers;
mod playback_start;
mod zoom;

fn install_decoded_waveform(controller: &mut AppController) {
    controller.sample_view.waveform.decoded = Some(std::sync::Arc::new(DecodedWaveform {
        cache_token: 1,
        samples: std::sync::Arc::from(vec![0.0; 10_000]),
        analysis_samples: std::sync::Arc::from(Vec::new()),
        analysis_sample_rate: 0,
        analysis_stride: 1,
        peaks: None,
        duration_seconds: 1.0,
        sample_rate: 48_000,
        channels: 1,
    }));
}
