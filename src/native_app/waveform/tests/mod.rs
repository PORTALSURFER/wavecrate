use super::{
    BAND_COUNT, LiveSelectionPreview, MIN_VISIBLE_FRAMES, WaveformActiveDragKind,
    WaveformEditFadeHandle, WaveformEditFadeOuterGainHandle, WaveformInteraction,
    WaveformSelectionEdge, WaveformSelectionKind, WaveformState, WaveformViewport, WaveformWidget,
    WaveformWidgetProps, downmix_to_mono, signal_edit_selection_for_state, split_frequency_bands,
    waveform_file_from_mono_samples, waveform_signal_surface_view,
};
use radiant::{
    gui::types::{Point, Rect, Vector2},
    prelude::IntoView,
    runtime::{GpuSurfaceContent, PaintFillRect, PaintStrokePolyline, SurfacePaintPlan},
    theme::ThemeTokens,
    widgets::{PointerButton, PointerModifiers, Widget, WidgetInput, WidgetOutput},
};
use std::{fs, sync::Arc};

mod audio;
mod edit_fade;
mod edit_fade_edge_cases;
mod edit_gain;
mod extraction;
mod paint;
mod signal_widget;
mod state;
mod widget_input;
mod zero_crossing_snap;

fn waveform_widget_for_state(state: &WaveformState) -> WaveformWidget {
    waveform_widget_for_state_with_beat_guides(state, false, 4)
}

fn waveform_widget_for_state_with_beat_guides(
    state: &WaveformState,
    enabled: bool,
    count: u8,
) -> WaveformWidget {
    WaveformWidget::new(WaveformWidgetProps::from_state(state, enabled, count))
}

fn fill_rects(plan: &SurfacePaintPlan) -> Vec<&PaintFillRect> {
    plan.fill_rects().collect()
}

fn stroke_polylines(plan: &SurfacePaintPlan) -> Vec<&PaintStrokePolyline> {
    plan.stroke_polylines().collect()
}

fn assert_pointer_location_output(output: Option<WidgetOutput>) {
    assert!(matches!(
        output.and_then(|output| output.typed_copied::<WaveformInteraction>()),
        Some(WaveformInteraction::RememberPointerLocation { .. })
    ));
}

fn gpu_surface_revision_for_file(file: Arc<super::WaveformFile>) -> u64 {
    let viewport = super::WaveformViewport::full(file.frames);
    let plan = waveform_signal_surface_plan(file, viewport, None, None);
    plan.gpu_surfaces()
        .map(|surface| surface.revision)
        .next()
        .expect("waveform gpu surface")
}

fn waveform_signal_surface_plan(
    file: Arc<super::WaveformFile>,
    viewport: super::WaveformViewport,
    edit_selection: Option<wavecrate::selection::SelectionRange>,
    sample_slide_frame_offset: Option<i64>,
) -> SurfacePaintPlan {
    let view =
        waveform_signal_surface_view(file, viewport, edit_selection, sample_slide_frame_offset)
            .id(crate::native_app::test_support::waveform::WAVEFORM_SIGNAL_WIDGET_ID)
            .size(200.0, 80.0);
    let surface = view.into_surface();
    let bounds = Rect::from_size(200.0, 80.0);
    let layout = radiant::layout::layout_tree(&surface.layout_node(), bounds);
    surface.paint_plan(&layout, &ThemeTokens::default())
}

fn write_test_wav_i16(path: &std::path::Path, samples: &[i16]) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    for sample in samples {
        writer.write_sample(*sample).expect("write sample");
    }
    writer.finalize().expect("finalize wav");
}

fn write_test_wav_i16_stereo(path: &std::path::Path, frames: &[(i16, i16)]) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    for (left, right) in frames {
        writer.write_sample(*left).expect("write left sample");
        writer.write_sample(*right).expect("write right sample");
    }
    writer.finalize().expect("finalize wav");
}

fn read_test_wav_i16(path: &std::path::Path) -> Vec<i16> {
    let mut reader = hound::WavReader::open(path).expect("open wav");
    reader
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .expect("read samples")
}

fn read_test_wav_f32(path: &std::path::Path) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("open wav");
    reader
        .samples::<f32>()
        .collect::<Result<Vec<_>, _>>()
        .expect("read samples")
}

fn write_interleaved_f32_file(path: &std::path::Path, samples: &[f32]) {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(samples));
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, bytes).expect("write f32 sidecar");
}
