use super::*;

#[test]
fn click_removal_interpolates_selected_span() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "click.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let selection = SelectionRange::new(0.4, 0.6);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "click.wav",
        &[0.0, 1.0, 9.0, -1.0, 0.0],
        selection,
    );
    let preserved_view = WaveformView {
        start: 0.2,
        end: 0.4,
    };
    controller.ui.waveform.view = preserved_view;

    controller.repair_clicks_selection().unwrap();

    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert!(samples[2].abs() < 1e-6);
    assert_eq!(controller.ui.waveform.selection, Some(selection));
    assert!((controller.ui.waveform.view.start - preserved_view.start).abs() < 1e-6);
    assert!((controller.ui.waveform.view.end - preserved_view.end).abs() < 1e-6);
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);
}

#[test]
fn cropping_selection_overwrites_file() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "edit.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "edit.wav",
        &[0.1, 0.2, 0.3, 0.4],
        SelectionRange::new(0.25, 0.75),
    );

    controller.crop_waveform_selection().unwrap();

    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples, vec![0.2, 0.3]);
    assert!(controller.ui.waveform.selection.is_none());
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);
}

#[test]
fn trimming_selection_removes_span() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "trim.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "trim.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );

    controller.trim_waveform_selection().unwrap();

    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples, vec![0.0, 0.3]);
    assert!(controller.ui.waveform.selection.is_none());
    let entry = controller.wav_entry(0).unwrap();
    assert!(entry.file_size > 0);
}
