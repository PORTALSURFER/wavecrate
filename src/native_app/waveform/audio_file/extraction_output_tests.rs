use super::extraction::{
    extract_wav_reader_range_to_folder, finalize_wav_writer, publish_staged_extraction, wav_writer,
    write_extraction_atomically,
};
use std::{
    io::{Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
};
use wavecrate::selection::SelectionRange;

struct FailingDataCursor {
    inner: Cursor<Vec<u8>>,
    fail_at: u64,
}

impl Read for FailingDataCursor {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let position = self.inner.position();
        if position >= self.fail_at {
            return Err(std::io::Error::other("injected extraction read failure"));
        }
        let remaining = usize::try_from(self.fail_at - position).unwrap_or(usize::MAX);
        let read_len = buffer.len().min(remaining);
        self.inner.read(&mut buffer[..read_len])
    }
}

impl Seek for FailingDataCursor {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(position)
    }
}

#[test]
fn late_collision_preserves_existing_file_and_publishes_next_name() {
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("source.wav");
    let mut staged = tempfile::NamedTempFile::new_in(root.path()).expect("create staging file");
    staged.write_all(b"complete extraction").unwrap();
    staged.as_file().sync_all().unwrap();
    let first_candidate = root.path().join("source_extraction.wav");
    // This file represents a late arrival while the complete extraction was still staged.
    std::fs::write(&first_candidate, b"late arrival").expect("inject late collision");

    let output = publish_staged_extraction(staged, &source, root.path())
        .expect("publish after late collision");

    assert_eq!(std::fs::read(first_candidate).unwrap(), b"late arrival");
    assert_eq!(output, root.path().join("source_extraction_1.wav"));
    assert_eq!(std::fs::read(output).unwrap(), b"complete extraction");
}

#[test]
fn partial_write_and_cancel_failures_remove_owned_staging_file() {
    for error in ["injected write failure", "extraction cancelled"] {
        let root = tempfile::tempdir().expect("temp root");
        let source = root.path().join("source.wav");
        let result = write_extraction_atomically(&source, root.path(), |output| {
            output.write_all(b"partial WAV header").unwrap();
            Err(String::from(error))
        });

        assert_eq!(result.unwrap_err(), error);
        assert_no_extraction_artifacts(root.path());
    }
}

#[test]
fn finalize_failure_removes_owned_staging_file() {
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("source.wav");
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let error = write_extraction_atomically(&source, root.path(), |output| {
        let mut writer = wav_writer(output, spec)?;
        writer.write_sample(1_i16).unwrap();
        finalize_wav_writer(writer)
    })
    .unwrap_err();

    assert!(error.contains("failed to finalize extraction"));
    assert_no_extraction_artifacts(root.path());
}

#[test]
fn raw_read_failure_never_exposes_partial_wav() {
    assert_read_failure_removes_partial_wav(SelectionRange::new_precise(0.0, 1.0));
}

#[test]
fn decoded_read_failure_never_exposes_partial_wav() {
    assert_read_failure_removes_partial_wav(SelectionRange::new_precise(0.0, 1.0).with_gain(0.5));
}

fn assert_read_failure_removes_partial_wav(selection: SelectionRange) {
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("source.wav");
    write_i16_wav(&source, 256);
    let bytes = std::fs::read(&source).expect("read source wav");
    let reader = FailingDataCursor {
        inner: Cursor::new(bytes),
        fail_at: 52,
    };

    let error =
        extract_wav_reader_range_to_folder(&source, root.path(), reader, 256, selection, 1.0)
            .unwrap_err();

    assert!(error.contains("injected extraction read failure"));
    assert_no_extraction_artifacts_except_source(root.path(), &source);
}

#[test]
fn staging_open_failure_creates_no_final_output() {
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("source.wav");
    let missing = root.path().join("missing");

    let error = write_extraction_atomically(&source, &missing, |_| Ok(())).unwrap_err();

    assert!(error.contains("failed to create extraction staging file"));
    assert!(!root.path().join("source_extraction.wav").exists());
}

fn assert_no_extraction_artifacts(root: &Path) {
    assert_no_extraction_artifacts_except_source(root, Path::new(""));
}

fn assert_no_extraction_artifacts_except_source(root: &Path, source: &Path) {
    let artifacts = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path != source)
        .collect::<Vec<_>>();
    assert!(
        artifacts.is_empty(),
        "unexpected extraction artifacts: {artifacts:?}"
    );
}

fn write_i16_wav(path: &Path, frames: i16) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    for sample in 0..frames {
        writer.write_sample(sample).expect("write sample");
    }
    writer.finalize().expect("finalize wav");
}
