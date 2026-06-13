use super::*;
#[test]
fn sample_auto_rename_streams_per_item_progress() {
    let (_temp, source) = setup_fixture(&["alpha.wav", "beta.wav"]);
    let (progress, rx) = file_op_progress_capture();

    let result = run_sample_auto_rename_job(
        source,
        vec![
            rename_request("alpha.wav", "alpha_renamed.wav"),
            rename_request("beta.wav", "beta_renamed.wav"),
        ],
        Arc::new(AtomicBool::new(false)),
        Some(progress),
    );

    assert!(result.errors.is_empty());
    let messages = drain_file_op_progress(rx);
    assert!(
        messages
            .iter()
            .any(|(completed, detail, item)| *completed == 1
                && detail.as_deref() == Some("Renamed alpha_renamed.wav")
                && *item
                    == Some(SampleAutoRenameProgress::Completed {
                        old_relative: PathBuf::from("alpha.wav"),
                        new_relative: PathBuf::from("alpha_renamed.wav"),
                    })),
        "missing first rename progress: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|(completed, detail, item)| *completed == 2
                && detail.as_deref() == Some("Renamed beta_renamed.wav")
                && *item
                    == Some(SampleAutoRenameProgress::Completed {
                        old_relative: PathBuf::from("beta.wav"),
                        new_relative: PathBuf::from("beta_renamed.wav"),
                    })),
        "missing second rename progress: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|(completed, detail, item)| *completed == 0
                && detail.is_none()
                && *item
                    == Some(SampleAutoRenameProgress::Active {
                        old_relative: PathBuf::from("alpha.wav")
                    }))
    );
}

#[test]
fn sample_auto_rename_cancel_stops_after_partial_completion() {
    const WORKER_PROGRESS_TIMEOUT: Duration = Duration::from_secs(10);
    const WORKER_STOP_TIMEOUT: Duration = Duration::from_secs(60);

    let (_temp, source) = setup_fixture(&["alpha.wav", "beta.wav", "gamma.wav"]);
    let cancel = Arc::new(AtomicBool::new(false));
    let (progress, rx) = file_op_progress_capture();
    let worker_cancel = cancel.clone();
    let worker_source = source.clone();
    let (result_tx, result_rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = run_sample_auto_rename_job(
            worker_source,
            vec![
                rename_request("alpha.wav", "alpha_renamed.wav"),
                rename_request("beta.wav", "beta_renamed.wav"),
                rename_request("gamma.wav", "gamma_renamed.wav"),
            ],
            worker_cancel,
            Some(progress),
        );
        result_tx.send(result).expect("send auto-rename result");
    });

    loop {
        if let JobMessage::FileOps(FileOpMessage::Progress { completed: 1, .. }) = rx
            .recv_timeout(WORKER_PROGRESS_TIMEOUT)
            .expect("wait for first progress")
        {
            cancel.store(true, Ordering::Relaxed);
            break;
        }
    }

    let result = result_rx
        .recv_timeout(WORKER_STOP_TIMEOUT)
        .expect("worker should stop after cancellation");

    assert!(!result.renamed.is_empty());
    assert!(
        result.renamed.len() < result.requested_paths.len(),
        "cancellation should stop before the full batch completes: {result:?}"
    );
    assert_eq!(result.errors.len(), 1);
    let cancelled_path = result.errors[0].0.clone();
    assert_eq!(result.errors[0].1, "Rename cancelled");
    assert!(source.root.join("alpha_renamed.wav").exists());
    assert!(source.root.join(&cancelled_path).exists());
    let cancelled_target = match cancelled_path.to_string_lossy().as_ref() {
        "beta.wav" => "beta_renamed.wav",
        "gamma.wav" => "gamma_renamed.wav",
        other => panic!("unexpected cancelled path: {other}"),
    };
    assert!(!source.root.join(cancelled_target).exists());
}
