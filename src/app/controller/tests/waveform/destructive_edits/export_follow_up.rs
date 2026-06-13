use super::*;

fn pump_background_jobs_until(
    controller: &mut crate::app::controller::AppController,
    mut predicate: impl FnMut(&mut crate::app::controller::AppController) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        controller.poll_background_jobs();
        if predicate(controller) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for background job condition");
}

fn wait_for_analysis_enqueue_finished(
    controller: &mut crate::app::controller::AppController,
) -> AnalysisJobMessage {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match controller.runtime.jobs.try_recv_message() {
            Ok(JobMessage::Analysis(message @ AnalysisJobMessage::EnqueueFinished { .. })) => {
                return message;
            }
            Ok(_) => {}
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("unexpected receive error: {err:?}"),
        }
    }
    panic!("timed out waiting for analysis enqueue message");
}

#[test]
fn cropping_selection_enqueues_reanalysis_without_overwriting_status() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "edit.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    load_waveform_selection(
        &mut controller,
        &source,
        "edit.wav",
        &[0.1, 0.2, 0.3, 0.4],
        SelectionRange::new(0.25, 0.75),
    );

    controller.crop_waveform_selection().unwrap();

    match wait_for_analysis_enqueue_finished(&mut controller) {
        AnalysisJobMessage::EnqueueFinished {
            inserted, announce, ..
        } => {
            assert!(inserted >= 1);
            assert!(!announce);
        }
        other => panic!("unexpected analysis message: {other:?}"),
    }
    assert_eq!(controller.ui.status.text, "Cropped selection edit.wav");
}

#[test]
fn crop_to_new_sample_queues_export_and_async_loads_new_clip() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "crop.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "crop.wav",
        &[0.1, 0.2, 0.3, 0.4],
        SelectionRange::new(0.25, 0.75),
    );

    controller.crop_waveform_selection_to_new_sample().unwrap();

    assert_eq!(controller.ui.status.status_tone, StatusTone::Busy);
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| &audio.relative_path),
        Some(&std::path::PathBuf::from("crop.wav"))
    );
    pump_background_jobs_until(&mut controller, |controller| {
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| audio.relative_path == Path::new("crop_crop001.wav"))
    });

    assert!(source.root.join("crop_crop001.wav").is_file());
    assert!(wav_path.is_file());
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| &audio.relative_path),
        Some(&std::path::PathBuf::from("crop_crop001.wav"))
    );
}
