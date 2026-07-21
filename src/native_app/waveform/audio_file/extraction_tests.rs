use super::extraction::{
    extract_wav_reader_range_to_folder, finalize_wav_writer, publish_staged_extraction, wav_writer,
    write_extraction_atomically, write_wav_frame_range,
};
use std::{
    cell::Cell,
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
    rc::Rc,
};
use wavecrate::selection::SelectionRange;

struct CountingCursor {
    inner: Cursor<Vec<u8>>,
    read_bytes: Rc<Cell<usize>>,
    read_calls: Rc<Cell<usize>>,
}

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

impl CountingCursor {
    fn new(bytes: Vec<u8>, read_bytes: Rc<Cell<usize>>) -> Self {
        Self::with_counters(bytes, read_bytes, Rc::new(Cell::new(0)))
    }

    fn with_counters(
        bytes: Vec<u8>,
        read_bytes: Rc<Cell<usize>>,
        read_calls: Rc<Cell<usize>>,
    ) -> Self {
        Self {
            inner: Cursor::new(bytes),
            read_bytes,
            read_calls,
        }
    }
}

impl Read for CountingCursor {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.read_calls.set(self.read_calls.get() + 1);
        let read = self.inner.read(buffer)?;
        self.read_bytes.set(self.read_bytes.get() + read);
        Ok(read)
    }
}

impl Seek for CountingCursor {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(position)
    }
}

#[test]
fn late_wav_range_extraction_seeks_instead_of_reading_prefix() {
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("source.wav");
    let output = root.path().join("late-selection.wav");
    write_i16_wav(&source, 20_000);
    let bytes = std::fs::read(&source).expect("read source wav");
    let read_bytes = Rc::new(Cell::new(0));
    let counted = CountingCursor::new(bytes, Rc::clone(&read_bytes));
    let reader = hound::WavReader::new(counted).expect("open counted wav");
    let spec = reader.spec();
    read_bytes.set(0);

    let mut output_file = File::create(&output).expect("create output");
    write_wav_frame_range(reader, spec, 1, 19_000, 19_100, &mut output_file, 1.0)
        .expect("extract late range");

    assert!(
        read_bytes.get() < 512,
        "late extraction read {} bytes after the header; it should seek over the skipped prefix",
        read_bytes.get()
    );
    let extracted = read_i16_wav(&output);
    assert_eq!(extracted.len(), 100);
    assert_eq!(extracted[0], 0);
    assert_eq!(extracted[50], 19_050);
    assert_eq!(extracted[99], 0);
}

#[test]
fn plain_wav_range_extraction_applies_short_edge_fades() {
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("source.wav");
    write_i16_wav(&source, 20_000);
    let bytes = std::fs::read(&source).expect("read source wav");
    let read_bytes = Rc::new(Cell::new(0));
    let read_calls = Rc::new(Cell::new(0));
    let counted =
        CountingCursor::with_counters(bytes, Rc::clone(&read_bytes), Rc::clone(&read_calls));
    let selection = SelectionRange::new_precise(5_000.0 / 20_000.0, 15_000.0 / 20_000.0);

    let output =
        extract_wav_reader_range_to_folder(&source, root.path(), counted, 20_000, selection, 1.0)
            .expect("extract plain wav range");

    assert!(read_calls.get() > 0);
    assert!(read_bytes.get() > 0);
    let extracted = read_i16_wav(&output);
    assert_eq!(extracted.len(), 10_000);
    assert_eq!(extracted[0], 0);
    assert_eq!(extracted[88], 5_088);
    assert_eq!(extracted[5_000], 10_000);
    assert_eq!(extracted[9_999], 0);
}

#[test]
fn wav_range_extraction_applies_forced_normalized_gain() {
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("source.wav");
    write_i16_wav(&source, 256);
    let bytes = std::fs::read(&source).expect("read source wav");
    let selection = SelectionRange::new_precise(64.0 / 256.0, 96.0 / 256.0);
    let preview_gain = wavecrate::audio::normalized_gain_from_peak(0.5);

    let output = extract_wav_reader_range_to_folder(
        &source,
        root.path(),
        Cursor::new(bytes),
        256,
        selection,
        preview_gain,
    )
    .expect("extract normalized wav range");

    let extracted = read_i16_wav(&output);
    assert_eq!(extracted.len(), 32);
    assert!((preview_gain - 2.0).abs() < f32::EPSILON);
    assert_eq!(extracted[0], 0);
    assert_eq!(extracted[16], 160);
    assert_eq!(extracted[31], 0);
}

#[test]
fn wav_range_extraction_preserves_i24_source_format_with_gain() {
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("source-24.wav");
    let mut samples = vec![0_i32; 768];
    samples[384] = 5_000_000;
    samples[448] = -5_000_000;
    write_i24_wav(&source, &samples);
    let bytes = std::fs::read(&source).expect("read source wav");
    let selection = SelectionRange::new_precise(128.0 / 768.0, 640.0 / 768.0);

    let output = extract_wav_reader_range_to_folder(
        &source,
        root.path(),
        Cursor::new(bytes),
        768,
        selection,
        2.0,
    )
    .expect("extract 24-bit wav range with gain");

    let mut reader = hound::WavReader::open(&output).expect("open extracted 24-bit wav");
    let spec = reader.spec();
    assert_eq!(spec.bits_per_sample, 24);
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);
    let extracted = reader
        .samples::<i32>()
        .map(|sample| sample.expect("read 24-bit sample"))
        .collect::<Vec<_>>();
    assert_eq!(extracted.len(), 512);
    assert_eq!(extracted[0], 0);
    assert_eq!(extracted[256], 8_388_607);
    assert_eq!(extracted[320], -8_388_608);
    assert_eq!(extracted[511], 0);
}

#[test]
fn decoded_wav_range_writer_clamps_i24_samples_to_destination_depth() {
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("decoded-source-24.wav");
    let output = root.path().join("decoded-output-24.wav");
    let mut samples = vec![0_i32; 256];
    samples[128] = 5_000_000;
    samples[160] = -5_000_000;
    write_i24_wav(&source, &samples);
    let reader = hound::WavReader::open(&source).expect("open 24-bit wav");
    let spec = reader.spec();

    let mut output_file = File::create(&output).expect("create decoded output");
    write_wav_frame_range(reader, spec, 1, 0, 256, &mut output_file, 2.0)
        .expect("write decoded 24-bit range with gain");

    let mut reader = hound::WavReader::open(&output).expect("open decoded 24-bit output");
    let spec = reader.spec();
    assert_eq!(spec.bits_per_sample, 24);
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);
    let extracted = reader
        .samples::<i32>()
        .map(|sample| sample.expect("read 24-bit sample"))
        .collect::<Vec<_>>();
    assert_eq!(extracted[128], 8_388_607);
    assert_eq!(extracted[160], -8_388_608);
}

#[test]
fn wav_range_extraction_handles_metadata_chunk_before_data() {
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("source.wav");
    write_i16_wav(&source, 256);
    let bytes = wav_with_junk_before_data(std::fs::read(&source).expect("read source wav"));
    let read_bytes = Rc::new(Cell::new(0));
    let read_calls = Rc::new(Cell::new(0));
    let counted =
        CountingCursor::with_counters(bytes, Rc::clone(&read_bytes), Rc::clone(&read_calls));
    let selection = SelectionRange::new_precise(64.0 / 256.0, 96.0 / 256.0);

    let output =
        extract_wav_reader_range_to_folder(&source, root.path(), counted, 256, selection, 1.0)
            .expect("extract wav range with metadata chunk");

    assert!(read_calls.get() > 0);
    let extracted = read_i16_wav(&output);
    assert_eq!(extracted.len(), 32);
    assert_eq!(extracted[0], 0);
    assert_eq!(extracted[16], 80);
    assert_eq!(extracted[31], 0);
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
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("source.wav");
    write_i16_wav(&source, 256);
    let bytes = std::fs::read(&source).expect("read source wav");
    let reader = FailingDataCursor {
        inner: Cursor::new(bytes),
        fail_at: 52,
    };

    let error = extract_wav_reader_range_to_folder(
        &source,
        root.path(),
        reader,
        256,
        SelectionRange::new_precise(0.0, 1.0),
        1.0,
    )
    .unwrap_err();

    assert!(error.contains("injected extraction read failure"));
    assert_no_extraction_artifacts_except_source(root.path(), &source);
}

#[test]
fn decoded_read_failure_never_exposes_partial_wav() {
    let root = tempfile::tempdir().expect("temp root");
    let source = root.path().join("source.wav");
    write_i16_wav(&source, 256);
    let bytes = std::fs::read(&source).expect("read source wav");
    let reader = FailingDataCursor {
        inner: Cursor::new(bytes),
        fail_at: 52,
    };
    let selection = SelectionRange::new_precise(0.0, 1.0).with_gain(0.5);

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

fn write_i16_wav(path: &std::path::Path, frames: i16) {
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

fn write_i24_wav(path: &std::path::Path, samples: &[i32]) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 24,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create 24-bit wav");
    for sample in samples {
        writer.write_sample(*sample).expect("write 24-bit sample");
    }
    writer.finalize().expect("finalize 24-bit wav");
}

fn read_i16_wav(path: &std::path::Path) -> Vec<i16> {
    let mut reader = hound::WavReader::open(path).expect("open wav");
    reader
        .samples::<i16>()
        .map(|sample| sample.expect("read sample"))
        .collect()
}

fn wav_with_junk_before_data(bytes: Vec<u8>) -> Vec<u8> {
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[12..16], b"fmt ");
    let fmt_len = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
    let data_header_offset = 12 + 8 + fmt_len + (fmt_len % 2);
    assert_eq!(&bytes[data_header_offset..data_header_offset + 4], b"data");

    let junk_payload = b"abcde";
    let mut with_junk = Vec::with_capacity(bytes.len() + 14);
    with_junk.extend_from_slice(&bytes[..data_header_offset]);
    with_junk.extend_from_slice(b"JUNK");
    with_junk.extend_from_slice(&(junk_payload.len() as u32).to_le_bytes());
    with_junk.extend_from_slice(junk_payload);
    with_junk.push(0);
    with_junk.extend_from_slice(&bytes[data_header_offset..]);
    let riff_size = (with_junk.len() - 8) as u32;
    with_junk[4..8].copy_from_slice(&riff_size.to_le_bytes());
    with_junk
}
