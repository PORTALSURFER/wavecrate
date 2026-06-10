use super::*;

#[test]
fn canonical_chunks_include_offsets_and_padding() {
    let bytes = wav_with_chunks(&[(b"fmt ", &[1, 2, 3, 4][..]), (b"JUNK", &[9][..])]);
    let chunks = WavChunkIter::new(&bytes, bytes.len() as u64)
        .expect("wav")
        .collect::<Vec<_>>();

    assert_eq!(chunks.len(), 2);
    let WavChunkItem::Chunk(fmt) = chunks[0] else {
        panic!("fmt chunk");
    };
    assert_eq!(fmt.id(), b"fmt ");
    assert_eq!(fmt.offset(), 12);
    assert_eq!(fmt.data_size(), 4);
    assert_eq!(fmt.data_offset, 20);
    assert_eq!(fmt.data_end, 24);
    assert_eq!(fmt.next_offset, 24);

    let WavChunkItem::Chunk(junk) = chunks[1] else {
        panic!("junk chunk");
    };
    assert_eq!(junk.id(), b"JUNK");
    assert_eq!(junk.data_size(), 1);
    assert_eq!(junk.data_end, 33);
    assert_eq!(junk.next_offset, 34);
}

#[test]
fn truncated_top_level_header_is_rejected() {
    assert_eq!(
        WavChunkIter::new(b"RIFF", 4).expect_err("too short"),
        WavHeaderError::HeaderTooShort
    );
}

#[test]
fn non_wave_riff_is_rejected() {
    let mut bytes = [0_u8; 12];
    bytes[0..4].copy_from_slice(b"RIFF");
    bytes[8..12].copy_from_slice(b"AIFF");

    assert_eq!(
        WavChunkIter::new(&bytes, bytes.len() as u64).expect_err("not wave"),
        WavHeaderError::NotRiffWave
    );
}

#[test]
fn truncated_chunk_header_reports_error_at_eof() {
    let bytes = wav_with_trailing_bytes(b"JUN");
    let item = WavChunkIter::new(&bytes, bytes.len() as u64)
        .expect("wav")
        .next()
        .expect("item");

    assert_eq!(
        item,
        WavChunkItem::Invalid(WavChunkError::TruncatedHeader { offset: 12 })
    );
}

#[test]
fn incomplete_probe_prefix_is_distinct_from_invalid_chunk() {
    let bytes = wav_with_trailing_bytes(b"JUN");
    let item = WavChunkIter::new(&bytes, 100)
        .expect("wav")
        .next()
        .expect("item");

    assert_eq!(item, WavChunkItem::IncompletePrefix { offset: 12 });
}

#[test]
fn chunk_size_past_file_is_invalid() {
    let mut bytes = wav_header();
    bytes.extend_from_slice(b"JUNK");
    bytes.extend_from_slice(&100_u32.to_le_bytes());

    let item = WavChunkIter::new(&bytes, bytes.len() as u64)
        .expect("wav")
        .next()
        .expect("item");

    assert_eq!(
        item,
        WavChunkItem::Invalid(WavChunkError::ChunkDataOutOfFile {
            offset: 12,
            data_offset: 20,
            data_size: 100,
            total_file_len: bytes.len() as u64,
        })
    );
}

#[test]
fn unknown_chunks_are_preserved_as_chunks() {
    let bytes = wav_with_chunks(&[(b"abcd", &[1, 2][..])]);
    let item = WavChunkIter::new(&bytes, bytes.len() as u64)
        .expect("wav")
        .next()
        .expect("item");

    let WavChunkItem::Chunk(chunk) = item else {
        panic!("chunk");
    };
    assert_eq!(chunk.id(), b"abcd");
    assert_eq!(chunk.data_size(), 2);
}

fn wav_with_chunks(chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let mut bytes = wav_header();
    for (id, data) in chunks {
        bytes.extend_from_slice(*id);
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(data);
        if data.len() % 2 == 1 {
            bytes.push(0);
        }
    }
    bytes
}

fn wav_with_trailing_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut wav = wav_header();
    wav.extend_from_slice(bytes);
    wav
}

fn wav_header() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes
}
