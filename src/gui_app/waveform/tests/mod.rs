use super::{
    BAND_COUNT, WaveformActiveDragKind, WaveformEditFadeHandle, WaveformInteraction,
    WaveformSelectionEdge, WaveformSelectionKind, WaveformSignalWidget, WaveformState,
    WaveformViewport, WaveformWidget, WaveformWidgetProps, downmix_to_mono, split_frequency_bands,
    waveform_file_from_mono_samples,
};
use radiant::{
    gui::types::{Point, Rect, Vector2},
    runtime::{GpuSurfaceContent, PaintFillRect, PaintPrimitive},
    theme::ThemeTokens,
    widgets::{PointerButton, Widget, WidgetInput},
};
use std::{fs, sync::Arc};

mod audio;
mod edit_fade;
mod edit_fade_edge_cases;
mod extraction;
mod paint;
mod signal_widget;
mod state;
mod widget_input;

fn waveform_widget_for_state(state: &WaveformState) -> WaveformWidget {
    WaveformWidget::new(WaveformWidgetProps::from_state(state))
}

fn fill_rects(primitives: &[PaintPrimitive]) -> Vec<&PaintFillRect> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some(fill),
            _ => None,
        })
        .collect()
}

fn gpu_surface_revision_for_file(file: Arc<super::WaveformFile>) -> u64 {
    let viewport = super::WaveformViewport::full(file.frames);
    let widget = WaveformSignalWidget::new(file, viewport, None, None);
    let mut primitives = Vec::new();
    widget.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0)),
        &Default::default(),
        &ThemeTokens::default(),
    );
    primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(surface) => Some(surface.revision),
            _ => None,
        })
        .expect("waveform gpu surface")
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
