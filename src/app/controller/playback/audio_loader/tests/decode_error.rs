use super::*;

#[test]
fn load_audio_inner_reports_non_wav_decode_failures_as_audio_format_errors() {
    let renderer = WaveformRenderer::new(16, 16);
    let latest = AtomicU64::new(1);
    let mut temp = NamedTempFile::with_suffix(".aif").expect("temp aif");
    temp.write_all(b"not-a-supported-aif")
        .expect("write aif fixture");
    let relative_path = PathBuf::from(temp.path().file_name().expect("temp filename"));
    let job = test_job_with_root(
        1,
        temp.path().parent().expect("temp parent"),
        &relative_path,
        None,
    );

    let err = super::super::stages::load_audio_inner(&renderer, &job, &latest)
        .expect_err("unsupported non-wav decode should fail");
    let super::super::AudioLoadError::Failed(message) = err else {
        panic!("expected failed audio load");
    };

    assert!(
        !message.starts_with("Invalid wav"),
        "non-wav load should not surface raw wav wording: {message}"
    );
    assert!(
        !message.contains("Symphonia"),
        "non-wav load should not lead with decoder internals: {message}"
    );
    assert!(
        message.contains("unsupported or unreadable audio format")
            || message.contains("Unsupported audio codec"),
        "expected deliberate audio-format wording, got: {message}"
    );
}
