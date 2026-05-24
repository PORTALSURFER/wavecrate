pub(super) use super::super::*;
pub(super) use crate::app::controller::test_support;
pub(super) use crate::app::controller::test_support::{
    load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry,
};
pub(super) use std::cell::RefCell;
pub(super) use std::rc::Rc;
pub(super) use std::time::{Duration, Instant};

use crate::waveform::DecodedWaveform;
use std::sync::Arc;

/// Seed minimal waveform state so seek tests exercise cursor updates on a ready waveform.
pub(super) fn seed_waveform_ready_for_seek(controller: &mut AppController) {
    controller.sample_view.waveform.decoded = Some(Arc::new(DecodedWaveform {
        cache_token: 1,
        samples: Arc::from(vec![0.0; 16]),
        analysis_samples: Arc::from(Vec::new()),
        analysis_sample_rate: 0,
        analysis_stride: 1,
        peaks: None,
        duration_seconds: 1.0,
        sample_rate: 48_000,
        channels: 1,
    }));
}
