use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformPanDrag {
    anchor_x: f32,
    start_micros: u32,
    end_micros: u32,
}

impl<B: NativeAppBridge> WavecrateRuntimeBridge<B> {
    pub(super) fn begin_waveform_pan_drag(&mut self, layout: &ShellLayout, position: Point) {
        self.waveform_pan_drag = None;
        if !layout.waveform_plot.contains(position) {
            return;
        }
        let viewport = self.model.waveform.viewport();
        if viewport.end_micros.saturating_sub(viewport.start_micros) >= 999_999 {
            return;
        }
        self.waveform_pan_drag = Some(WaveformPanDrag {
            anchor_x: position.x,
            start_micros: viewport.start_micros,
            end_micros: viewport.end_micros,
        });
    }

    pub(super) fn waveform_pan_drag_action(
        &self,
        layout: &ShellLayout,
        position: Point,
    ) -> Option<UiAction> {
        let drag = self.waveform_pan_drag?;
        let span = drag.end_micros.saturating_sub(drag.start_micros).max(1);
        if span >= 999_999 || layout.waveform_plot.width() <= 1.0 {
            return None;
        }
        let delta_ratio = (position.x - drag.anchor_x) / layout.waveform_plot.width().max(1.0);
        let delta_micros = (delta_ratio * span as f32).round() as i64;
        let max_start = 1_000_000i64.saturating_sub(i64::from(span));
        let start = (i64::from(drag.start_micros) - delta_micros).clamp(0, max_start);
        let center_micros = (start + i64::from(span / 2)).clamp(0, 1_000_000) as u32;
        Some(UiAction::SetWaveformViewCenter {
            center_micros,
            center_nanos: None,
        })
    }
}
