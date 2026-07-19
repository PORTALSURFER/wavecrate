use super::*;

fn wait_for_readiness_reconciliation(
    controller: &mut crate::app::controller::AppController,
) -> AnalysisJobMessage {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match controller.runtime.jobs.try_recv_message() {
            Ok(JobMessage::Analysis(
                message @ AnalysisJobMessage::ReadinessReconciliationFinished { .. },
            )) => return message,
            Ok(_) => {}
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("unexpected receive error: {err:?}"),
        }
    }
    panic!("timed out waiting for readiness reconciliation");
}

#[test]
fn align_waveform_start_uses_hover_cursor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "align.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = source.root.join("align.wav");
    write_test_wav(&wav_path, &[1.0, 2.0, 3.0, 4.0]);
    controller
        .load_waveform_for_selection(&source, Path::new("align.wav"))
        .unwrap();
    controller.set_waveform_cursor_from_hover(0.5);
    controller.ui.waveform.last_start_marker = None;

    controller.align_waveform_start_to_last_marker().unwrap();

    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples, vec![3.0, 4.0, 1.0, 2.0]);
}

#[test]
fn align_waveform_start_reconciles_readiness_for_overwrite_in_place() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "align.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = source.root.join("align.wav");
    write_test_wav(&wav_path, &[1.0, 2.0, 3.0, 4.0]);
    controller
        .load_waveform_for_selection(&source, Path::new("align.wav"))
        .unwrap();
    controller.set_waveform_cursor_from_hover(0.5);

    controller.align_waveform_start_to_last_marker().unwrap();

    match wait_for_readiness_reconciliation(&mut controller) {
        AnalysisJobMessage::ReadinessReconciliationFinished {
            changed, announce, ..
        } => {
            assert!(changed >= 1);
            assert!(!announce);
        }
        other => panic!("unexpected analysis message: {other:?}"),
    }
    assert_eq!(controller.ui.status.text, "Slid sample align.wav");
}
