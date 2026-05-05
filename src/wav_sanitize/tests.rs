use super::*;
use hound::SampleFormat;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_FILE_ID: AtomicUsize = AtomicUsize::new(0);

struct TempWavFile {
    path: PathBuf,
}

impl TempWavFile {
    fn new(label: &str, bytes: &[u8]) -> Self {
        let id = NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sempal_wav_sanitize_{label}_{}_{}.wav",
            std::process::id(),
            id
        ));
        std::fs::write(&path, bytes).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempWavFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn wav_bytes_pcm_16bit(samples: &[i16]) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for &sample in samples {
            writer.write_sample(sample).unwrap();
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }
    cursor.into_inner()
}

fn padded_fmt_wav(extra_zero_bytes: usize) -> Vec<u8> {
    let mut bad = wav_bytes_pcm_16bit(&[0, 1000, -1000, 0]);
    let chunk_size = 16 + extra_zero_bytes;
    bad[16..20].copy_from_slice(&(chunk_size as u32).to_le_bytes());
    bad.splice(12 + 8 + 16..12 + 8 + 16, vec![0u8; extra_zero_bytes]);
    let riff_len = bad.len();
    bad[4..8].copy_from_slice(&((riff_len - 8) as u32).to_le_bytes());
    bad
}

fn read_exact_at(reader: &mut SanitizedWavReader, offset: u64, len: usize) -> Vec<u8> {
    reader.seek(SeekFrom::Start(offset)).unwrap();
    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes).unwrap();
    bytes
}

#[test]
fn fixes_pcm_fmt_chunk_size_20() {
    let fixed = sanitize_wav_bytes(padded_fmt_wav(4));
    assert!(hound::WavReader::new(Cursor::new(fixed.as_slice())).is_ok());
}

#[test]
fn fixes_pcm_fmt_chunk_size_22_with_padding() {
    let fixed = sanitize_wav_bytes(padded_fmt_wav(6));
    assert!(hound::WavReader::new(Cursor::new(fixed.as_slice())).is_ok());
}

#[test]
fn pass_through_reader_seek_matches_file_bytes() {
    let bytes = wav_bytes_pcm_16bit(&[10, 20, 30, 40]);
    let file = TempWavFile::new("pass_through", &bytes);
    let mut reader = open_sanitized_wav(file.path()).unwrap();

    assert_eq!(read_exact_at(&mut reader, 12, 8), bytes[12..20].to_vec());
    assert_eq!(reader.seek(SeekFrom::Current(5)).unwrap(), 25);
    assert_eq!(read_exact_at(&mut reader, 25, 4), bytes[25..29].to_vec());
    assert_eq!(
        reader.seek(SeekFrom::End(-8)).unwrap(),
        (bytes.len() - 8) as u64
    );

    let mut tail = Vec::new();
    reader.read_to_end(&mut tail).unwrap();
    assert_eq!(tail, bytes[bytes.len() - 8..]);
}

#[test]
fn sanitized_reader_read_to_end_returns_repaired_bytes() {
    let bad = padded_fmt_wav(4);
    let fixed = sanitize_wav_bytes(bad.clone());
    let file = TempWavFile::new("sanitized_read", &bad);
    let mut reader = open_sanitized_wav(file.path()).unwrap();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).unwrap();

    assert_ne!(bytes, bad);
    assert_eq!(bytes, fixed);
    assert!(hound::WavReader::new(Cursor::new(bytes)).is_ok());
}

#[test]
fn sanitized_reader_seek_start_resets_body_offset() {
    let bad = padded_fmt_wav(4);
    let fixed = sanitize_wav_bytes(bad.clone());
    let file = TempWavFile::new("seek_start_reset", &bad);
    let mut reader = open_sanitized_wav(file.path()).unwrap();

    reader.seek(SeekFrom::End(-4)).unwrap();
    reader.seek(SeekFrom::Start(0)).unwrap();

    let mut replay = Vec::new();
    reader.read_to_end(&mut replay).unwrap();
    assert_eq!(replay, fixed);
}

#[test]
fn sanitized_reader_seek_current_and_end_follow_logical_stream() {
    let bad = padded_fmt_wav(6);
    let fixed = sanitize_wav_bytes(bad.clone());
    let file = TempWavFile::new("seek_current_end", &bad);
    let mut reader = open_sanitized_wav(file.path()).unwrap();

    assert_eq!(read_exact_at(&mut reader, 0, 24), fixed[..24].to_vec());
    assert_eq!(reader.seek(SeekFrom::Current(3)).unwrap(), 27);
    assert_eq!(read_exact_at(&mut reader, 27, 5), fixed[27..32].to_vec());

    let tail_start = fixed.len() - 6;
    assert_eq!(reader.seek(SeekFrom::End(-6)).unwrap(), tail_start as u64);

    let mut tail = Vec::new();
    reader.read_to_end(&mut tail).unwrap();
    assert_eq!(tail, fixed[tail_start..]);
}

#[test]
fn sanitize_wav_header_handles_large_chunk_sizes_safely() {
    let mut bytes = vec![0u8; 20];
    bytes[0..4].copy_from_slice(b"RIFF");
    bytes[8..12].copy_from_slice(b"WAVE");
    bytes[12..16].copy_from_slice(b"JUNK");
    bytes[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    let len = bytes.len() as u64;
    assert!(!sanitize_wav_header(&mut bytes, len));
}

#[test]
fn read_sanitized_wav_bytes_rejects_large_files() {
    let path = std::env::temp_dir().join(format!(
        "sempal_wav_sanitize_large_{}_{}.wav",
        std::process::id(),
        NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_SANITIZED_WAV_BYTES + 1).unwrap();

    let result = read_sanitized_wav_bytes(&path);
    assert!(result.is_err());

    let _ = std::fs::remove_file(path);
}
