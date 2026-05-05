use super::*;

/// Return the dominant waveform refresh reason when multiple requests coalesce.
fn merge_waveform_refresh_reason(
    existing: Option<WaveformRefreshReason>,
    incoming: WaveformRefreshReason,
) -> WaveformRefreshReason {
    let rank = |reason: WaveformRefreshReason| -> u8 {
        match reason {
            WaveformRefreshReason::View => 1,
            WaveformRefreshReason::Size => 2,
            WaveformRefreshReason::Data => 3,
        }
    };
    match existing {
        Some(current) if rank(current) >= rank(incoming) => current,
        _ => incoming,
    }
}

impl AppController {
    /// Begin a waveform refresh batch to coalesce repeated refresh requests.
    pub(crate) fn begin_waveform_refresh_batch(&mut self) {
        self.runtime.begin_waveform_refresh_batch();
    }

    /// End the active waveform refresh batch.
    pub(crate) fn end_waveform_refresh_batch(&mut self) {
        self.runtime.end_waveform_refresh_batch();
    }

    /// Update the waveform render target to match the current view size.
    pub fn update_waveform_size(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self.sample_view.waveform.size == [width, height] {
            return;
        }
        self.sample_view.waveform.size = [width, height];
        self.refresh_waveform_image_with_reason(WaveformRefreshReason::Size);
    }

    /// Request a waveform image refresh, coalescing while a refresh batch is active.
    pub(crate) fn refresh_waveform_image(&mut self) {
        self.refresh_waveform_image_with_reason(WaveformRefreshReason::View);
    }

    /// Request a waveform refresh with an explicit reason used for coalesced batching.
    pub(super) fn refresh_waveform_image_with_reason(&mut self, reason: WaveformRefreshReason) {
        if self.runtime.waveform_refresh_batch_active() {
            self.runtime.waveform_refresh_pending = true;
            self.runtime.waveform_refresh_pending_reason = Some(merge_waveform_refresh_reason(
                self.runtime.waveform_refresh_pending_reason,
                reason,
            ));
            return;
        }
        self.runtime.waveform_refresh_pending = false;
        self.runtime.waveform_refresh_pending_reason = None;
        self.refresh_waveform_image_now();
    }

    /// Flush a queued waveform image refresh once batching has completed.
    pub(crate) fn flush_pending_waveform_image_refresh(&mut self) {
        if self.runtime.waveform_refresh_batch_active() || !self.runtime.waveform_refresh_pending {
            return;
        }
        let reason = self
            .runtime
            .waveform_refresh_pending_reason
            .unwrap_or(WaveformRefreshReason::View);
        self.refresh_waveform_image_with_reason(reason);
    }

    /// Return true when a waveform-image refresh is queued.
    pub(crate) fn has_pending_waveform_image_refresh(&self) -> bool {
        self.runtime.waveform_refresh_pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::dummy_controller;
    use crate::waveform::DecodedWaveform;

    #[test]
    fn waveform_refresh_batch_keeps_highest_priority_reason() {
        let (mut controller, _) = dummy_controller();
        controller.sample_view.waveform.size = [16, 8];
        controller.begin_waveform_refresh_batch();
        controller.refresh_waveform_image();
        controller.update_waveform_size(24, 8);
        controller.apply_waveform_image_shared(Arc::new(decoded_waveform()), None);

        assert!(controller.has_pending_waveform_image_refresh());
        assert_eq!(
            controller.runtime.waveform_refresh_pending_reason,
            Some(WaveformRefreshReason::Data)
        );
    }

    #[test]
    fn flush_pending_waveform_refresh_renders_after_batch_finishes() {
        let (mut controller, _) = dummy_controller();
        controller.sample_view.waveform.size = [32, 8];
        controller.sample_view.waveform.decoded = Some(Arc::new(decoded_waveform()));
        controller.begin_waveform_refresh_batch();
        controller.refresh_waveform_image();
        assert!(controller.ui.waveform.image.is_none());

        controller.end_waveform_refresh_batch();
        controller.flush_pending_waveform_image_refresh();

        assert!(!controller.has_pending_waveform_image_refresh());
        assert!(controller.ui.waveform.image.is_some());
        assert!(controller.sample_view.waveform.render_meta.is_some());
    }

    fn decoded_waveform() -> DecodedWaveform {
        DecodedWaveform {
            cache_token: 1,
            samples: Arc::from(
                (0..256)
                    .map(|index| index as f32 / 256.0)
                    .collect::<Vec<_>>(),
            ),
            analysis_samples: Arc::from(Vec::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: None,
            duration_seconds: 1.0,
            sample_rate: 48_000,
            channels: 1,
        }
    }
}
