use super::*;
use crate::app_core::controller::build_named_gui_fixture_controller;
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::path::Path;

mod circular_slide;
mod options;
mod routing;
mod undo;

fn write_test_wav(path: &Path, samples: &[f32]) {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 8,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(path, spec).unwrap();
    for sample in samples {
        writer.write_sample(*sample).unwrap();
    }
    writer.finalize().unwrap();
}

fn read_test_wav_samples(path: &Path) -> Vec<f32> {
    WavReader::open(path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect()
}

fn with_waveform_fixture_controller(
    run: impl FnOnce(&mut AppController, crate::sample_sources::SampleSource, std::path::PathBuf),
) {
    let mut bundle = build_named_gui_fixture_controller(WaveformRenderer::new(16, 16), "waveform")
        .unwrap_or_else(|err| panic!("failed to build waveform fixture: {err}"));
    let source = bundle
        .controller
        .current_source()
        .expect("waveform fixture should select a source");
    let wav_path = source.root.join("kick_one.wav");
    run(&mut bundle.controller, source, wav_path);
}
