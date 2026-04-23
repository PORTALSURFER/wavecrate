use super::*;

#[test]
fn normalize_browser_sample_rejects_non_wav_targets_with_explicit_message() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("clip.flac", Rating::NEUTRAL)]);
    std::fs::write(source.root.join("clip.flac"), b"not-a-wav").expect("write flac fixture");

    let err = controller
        .normalize_browser_sample(0)
        .expect_err("non-wav normalize should fail");

    assert_eq!(
        err,
        "Normalize overwrite only supports WAV files; .flac is not supported"
    );
}
