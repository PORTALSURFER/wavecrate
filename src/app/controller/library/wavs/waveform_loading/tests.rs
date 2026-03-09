use super::*;
use crate::app::controller::test_support::{
    prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::selection::SelectionRange;
use std::path::Path;

/// Decode one test waveform into the shared finalize payload shape used by load completion paths.
fn decode_payload(
    controller: &AppController,
    source: &SampleSource,
    relative_path: &Path,
) -> (Arc<DecodedWaveform>, Arc<[u8]>) {
    let bytes: Arc<[u8]> = controller
        .read_waveform_bytes(source, relative_path)
        .expect("waveform bytes")
        .into();
    let decoded = Arc::new(
        controller
            .sample_view
            .renderer
            .decode_from_bytes(bytes.as_ref())
            .expect("decoded waveform"),
    );
    (decoded, bytes)
}

#[test]
/// Finalizing a new waveform should clear transient selection state and queue metadata persistence.
fn finish_waveform_load_shared_resets_selection_state_for_new_sample() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "finalize.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let relative_path = Path::new("finalize.wav");
    write_test_wav(&source.root.join(relative_path), &[0.0, 0.2, -0.2, 0.4]);
    let (decoded, bytes) = decode_payload(&controller, &source, relative_path);
    controller.ui.waveform.view = WaveformView {
        start: 0.25,
        end: 0.5,
    };
    controller.ui.waveform.cursor = Some(0.7);
    controller.ui.waveform.selection = Some(SelectionRange::new(0.1, 0.8));
    controller.ui.waveform.edit_selection = Some(SelectionRange::new(0.2, 0.7));
    controller.ui.waveform.notice = Some("loading".into());
    controller.ui.waveform.loading = Some(relative_path.to_path_buf());

    controller
        .finish_waveform_load_shared(FinishWaveformLoadShared {
            source: &source,
            relative_path,
            decoded,
            bytes,
            intent: AudioLoadIntent::Selection,
            preserve_selections: false,
            transients: Some(Arc::from([])),
        })
        .expect("finalized waveform load");

    assert_eq!(controller.ui.waveform.view, WaveformView::default());
    assert_eq!(controller.ui.waveform.cursor, Some(0.0));
    assert!(controller.ui.waveform.selection.is_none());
    assert!(controller.ui.waveform.edit_selection.is_none());
    assert!(controller.ui.waveform.notice.is_none());
    assert!(controller.ui.waveform.loading.is_none());
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(relative_path)
    );
    assert!(controller.has_pending_loaded_duration_metadata_write());
}

/// Query persisted analysis duration metadata for one sample path.
fn sample_duration_seconds(source: &SampleSource, relative_path: &Path) -> Option<f64> {
    let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), relative_path);
    let conn = match analysis_jobs::open_source_db(&source.root) {
        Ok(conn) => conn,
        Err(_) => return None,
    };
    conn.query_row(
        "SELECT duration_seconds FROM samples WHERE sample_id = ?1",
        rusqlite::params![sample_id],
        |row| row.get::<_, Option<f64>>(0),
    )
    .ok()
    .flatten()
}

#[test]
/// Loaded duration metadata writes should queue and only persist after deferred flush.
fn loaded_duration_metadata_write_is_deferred_until_flush() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "deferred.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let relative_path = Path::new("deferred.wav");
    let wav_path = source.root.join(relative_path);
    write_test_wav(&wav_path, &[0.0, 0.2, -0.2, 0.4]);

    let loaded = controller.load_waveform_for_selection(&source, relative_path);
    assert!(loaded.is_ok(), "waveform load failed: {loaded:?}");
    assert!(controller.has_pending_loaded_duration_metadata_write());
    assert!(sample_duration_seconds(&source, relative_path).is_none());

    controller
        .runtime
        .pending_loaded_duration_metadata_not_before =
        Some(Instant::now() - Duration::from_millis(1));
    controller.flush_pending_loaded_duration_metadata_write();

    assert!(!controller.has_pending_loaded_duration_metadata_write());
    let duration = sample_duration_seconds(&source, relative_path);
    assert!(duration.is_some(), "expected deferred duration metadata");
}

#[test]
/// Deferred metadata flush should wait while its debounce deadline is still active.
fn loaded_duration_metadata_flush_respects_deadline() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "deadline.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let relative_path = Path::new("deadline.wav");
    let wav_path = source.root.join(relative_path);
    write_test_wav(&wav_path, &[0.0, 0.1, 0.2, 0.3]);

    let loaded = controller.load_waveform_for_selection(&source, relative_path);
    assert!(loaded.is_ok(), "waveform load failed: {loaded:?}");
    controller
        .runtime
        .pending_loaded_duration_metadata_not_before =
        Some(Instant::now() + Duration::from_secs(60));

    controller.flush_pending_loaded_duration_metadata_write();

    assert!(controller.has_pending_loaded_duration_metadata_write());
    assert!(sample_duration_seconds(&source, relative_path).is_none());
}
