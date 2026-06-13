use super::*;

#[test]
fn destructive_edit_request_prompts_without_yolo_mode() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "warn.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "warn.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );
    controller.set_destructive_yolo_mode(false);

    let outcome = controller
        .request_destructive_selection_edit(DestructiveSelectionEdit::CropSelection)
        .unwrap();

    assert!(matches!(outcome, SelectionEditRequest::Prompted));
    let prompt = controller
        .ui
        .waveform
        .pending_destructive
        .as_ref()
        .expect("pending destructive prompt");
    assert!(prompt.message.contains("current write format"));
    assert!(prompt.message.contains("Source rate, 32-bit float"));
    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples.len(), 4);
}

#[test]
fn destructive_edit_request_blocks_multichannel_wav_before_prompt() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "surround.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = source.root.join("surround.wav");
    write_test_wav_with_spec(
        &wav_path,
        WavSpec {
            channels: 3,
            sample_rate: 44_100,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        },
        &[0.0, 0.1, 0.2, 0.3, 0.4, 0.5],
    );
    controller
        .load_waveform_for_selection(&source, Path::new("surround.wav"))
        .unwrap();
    controller.ui.waveform.selection = Some(SelectionRange::new(0.0, 0.5));
    controller.set_destructive_yolo_mode(false);

    let err = match controller
        .request_destructive_selection_edit(DestructiveSelectionEdit::CropSelection)
    {
        Ok(_) => panic!("multichannel destructive edit should be blocked"),
        Err(err) => err,
    };

    assert!(err.contains("mono or stereo"));
    assert!(err.contains("3 channels"));
    assert!(controller.ui.waveform.pending_destructive.is_none());
}

#[test]
fn yolo_mode_applies_destructive_edit_immediately() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "yolo.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "yolo.wav",
        &[0.1, 0.2, 0.3, 0.4],
        SelectionRange::new(0.25, 0.75),
    );
    controller.set_destructive_yolo_mode(true);

    let outcome = controller
        .request_destructive_selection_edit(DestructiveSelectionEdit::CropSelection)
        .unwrap();

    assert!(matches!(outcome, SelectionEditRequest::Applied));
    assert!(controller.ui.waveform.pending_destructive.is_none());
    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples, vec![0.2, 0.3]);
}

fn write_test_wav_with_spec(path: &Path, spec: WavSpec, samples: &[f32]) {
    let mut writer = WavWriter::create(path, spec).expect("create wav");
    for sample in samples {
        writer.write_sample(*sample).expect("write sample");
    }
    writer.finalize().expect("finalize wav");
}

#[test]
fn confirming_pending_destructive_edit_clears_prompt() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "confirm.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "confirm.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );
    controller.set_destructive_yolo_mode(false);
    controller
        .request_destructive_selection_edit(DestructiveSelectionEdit::TrimSelection)
        .unwrap();
    let prompt = controller.ui.waveform.pending_destructive.clone().unwrap();

    controller.apply_confirmed_destructive_edit(prompt.edit);

    assert!(controller.ui.waveform.pending_destructive.is_none());
    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples, vec![0.0, 0.3]);
}
