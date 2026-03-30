use super::super::*;
use crate::app::controller::test_support;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::controller::AppControllerNativeRuntimeExt;
use crate::waveform::DecodedWaveform;

mod clearing;
mod selection;
mod zoom;

/// Seed minimal decoded waveform state so zoom tests can exercise view math.
pub(super) fn seed_waveform_for_zoom(controller: &mut AppController) {
    controller.sample_view.waveform.size = [240, 24];
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
