use super::super::{audio_file::PersistedPlaybackCacheFile, execute_waveform_extraction};
use super::*;

#[test]
fn playmark_extraction_writes_sibling_wav_range() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-playmark-extract-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let source = root.join("source.wav");
    write_test_wav_i16(&source, &[0, 100, 200, 300, 400, 500]);
    let mut state = WaveformState::load_path(source).expect("load source");
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.75));

    let output = state
        .extract_play_selection_to_sibling()
        .expect("extract range");

    assert_eq!(output.file_name().unwrap(), "source_extraction.wav");
    assert_eq!(read_test_wav_i16(&output), vec![100, 200, 300, 400]);
    assert_eq!(
        state.extracted_ranges(),
        &[wavecrate::selection::SelectionRange::new(0.25, 0.75)]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn playmark_extraction_uses_channel_independent_frame_bounds() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-playmark-extract-stereo-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let source = root.join("source.wav");
    write_test_wav_i16_stereo(
        &source,
        &[
            (0, 1),
            (100, 101),
            (200, 201),
            (300, 301),
            (400, 401),
            (500, 501),
        ],
    );
    let mut state = WaveformState::load_path(source).expect("load source");
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.75));

    let output = state
        .extract_play_selection_to_sibling()
        .expect("extract range");

    assert_eq!(
        read_test_wav_i16(&output),
        vec![100, 101, 200, 201, 300, 301, 400, 401]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn playmark_drag_extraction_writes_to_target_folder() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-playmark-drag-extract-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let target = root.join("target");
    fs::create_dir_all(&target).expect("create target");
    let source = root.join("source.wav");
    write_test_wav_i16(&source, &[0, 100, 200, 300, 400, 500]);
    let mut state = WaveformState::load_path(source).expect("load source");
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.75));

    let output = state
        .extract_play_selection_to_folder(&target)
        .expect("extract range");

    assert_eq!(output.parent(), Some(target.as_path()));
    assert_eq!(output.file_name().unwrap(), "source_extraction.wav");
    assert_eq!(read_test_wav_i16(&output), vec![100, 200, 300, 400]);
    assert_eq!(
        state.extracted_ranges(),
        &[wavecrate::selection::SelectionRange::new(0.25, 0.75)]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn playmark_extraction_writes_from_decoded_playback_samples_without_audio_bytes() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-playmark-playback-samples-extract-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let source = root.join("source.aiff");
    let mut file =
        waveform_file_from_mono_samples(source, Arc::from([]), 48_000, 2, vec![0.0, 0.0, 0.0, 0.0]);
    file.playback_samples = Some(Arc::from(vec![0.0_f32, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7]));
    let mut state = WaveformState::from_cached_file(Arc::new(file));
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.75));

    let request = state
        .play_selection_extraction_request(None)
        .expect("build extraction request");
    let completion = execute_waveform_extraction(request);
    let output = completion.result.expect("extract range");

    assert_eq!(output.file_name().unwrap(), "source_extraction.wav");
    assert_eq!(read_test_wav_f32(&output), vec![0.2, 0.3, 0.4, 0.5]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn playmark_extraction_writes_from_persisted_playback_cache_without_audio_bytes() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-playmark-playback-cache-extract-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let source = root.join("source.wav");
    let cache_path = root.join("source.pcm");
    write_interleaved_f32_file(&cache_path, &[0.0_f32, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7]);
    let mut file =
        waveform_file_from_mono_samples(source, Arc::from([]), 48_000, 2, vec![0.0, 0.0, 0.0, 0.0]);
    file.playback_cache_file = PersistedPlaybackCacheFile::new(cache_path, 8);
    let mut state = WaveformState::from_cached_file(Arc::new(file));
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.75));

    let request = state
        .play_selection_extraction_request(None)
        .expect("build extraction request");
    let completion = execute_waveform_extraction(request);
    let output = completion.result.expect("extract range");

    assert_eq!(output.file_name().unwrap(), "source_extraction.wav");
    assert_eq!(read_test_wav_f32(&output), vec![0.2, 0.3, 0.4, 0.5]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn playmark_extraction_merges_extracted_range_marks() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-playmark-extract-merge-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let source = root.join("source.wav");
    write_test_wav_i16(&source, &[0, 100, 200, 300, 400, 500]);
    let mut state = WaveformState::load_path(source).expect("load source");

    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.1, 0.3));
    state
        .extract_play_selection_to_sibling()
        .expect("extract first range");
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.5));
    state
        .extract_play_selection_to_sibling()
        .expect("extract overlapping range");

    assert_eq!(
        state.extracted_ranges(),
        &[wavecrate::selection::SelectionRange::new(0.1, 0.5)]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn empty_waveform_rejects_playmark_extraction() {
    let mut state = WaveformState::empty();
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.1, 0.2));

    assert_eq!(
        state.extract_play_selection_to_sibling(),
        Err(String::from("Load a sample before extracting"))
    );
}
