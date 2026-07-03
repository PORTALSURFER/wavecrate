use std::{
    fs::File,
    io::{BufWriter, ErrorKind, Read, Seek, SeekFrom, Write},
    path::Path,
    time::Duration,
};

use wavecrate::audio::short_edge_fade_frame_count;
use wavecrate::selection::SelectionRange;

mod fade;

const RIFF_ID: &[u8; 4] = b"RIFF";
const WAVE_ID: &[u8; 4] = b"WAVE";
const FMT_ID: &[u8; 4] = b"fmt ";
const DATA_ID: &[u8; 4] = b"data";
const WAVE_FORMAT_PCM: u16 = 0x0001;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xfffe;
const MIN_FMT_CHUNK_BYTES: u32 = 16;
const MAX_FMT_CHUNK_BYTES: u32 = 4096;
const WAVE_SUBFORMAT_TAIL: [u8; 14] = [
    0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

pub(super) fn copy_selection_to_file<R: Read + Seek>(
    reader: &mut R,
    loaded_frames: usize,
    selection: SelectionRange,
    output_path: &Path,
    gain: f32,
    fade_duration: Duration,
) -> Result<bool, String> {
    if selection.has_edit_effects() {
        return Ok(false);
    }
    let Some(layout) = parse_layout(reader)? else {
        return Ok(false);
    };
    let total_frames = layout.complete_frames().min(loaded_frames);
    if total_frames == 0 {
        return Ok(false);
    }
    let frame_range = selection.frame_bounds(total_frames);
    let byte_span = layout.byte_span(frame_range.start_frame, frame_range.end_frame)?;
    write_raw_slice(reader, &layout, byte_span, output_path, gain, fade_duration)?;
    Ok(true)
}

#[derive(Clone)]
struct RawWavLayout {
    fmt_chunk: Vec<u8>,
    block_align: u16,
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    encoding: SampleEncoding,
    data_offset: u64,
    data_len: u64,
}

impl RawWavLayout {
    fn complete_frames(&self) -> usize {
        let frames = self.data_len / u64::from(self.block_align);
        usize::try_from(frames).unwrap_or(usize::MAX)
    }

    fn byte_span(&self, start_frame: usize, end_frame: usize) -> Result<RawByteSpan, String> {
        let frame_count = end_frame.saturating_sub(start_frame);
        let start_frame =
            u64::try_from(start_frame).map_err(|_| String::from("WAV selection is too large"))?;
        let frame_count =
            u64::try_from(frame_count).map_err(|_| String::from("WAV selection is too large"))?;
        let block_align = u64::from(self.block_align);
        let start_data_byte = start_frame
            .checked_mul(block_align)
            .ok_or_else(|| String::from("WAV selection starts too late"))?;
        let byte_len = frame_count
            .checked_mul(block_align)
            .ok_or_else(|| String::from("WAV selection is too large"))?;
        let end_data_byte = start_data_byte
            .checked_add(byte_len)
            .ok_or_else(|| String::from("WAV selection is too large"))?;
        if end_data_byte > self.data_len {
            return Err(String::from("WAV selection exceeds the source data chunk"));
        }
        let source_offset = self
            .data_offset
            .checked_add(start_data_byte)
            .ok_or_else(|| String::from("WAV selection starts too late"))?;
        Ok(RawByteSpan {
            source_offset,
            byte_len,
        })
    }
}

#[derive(Clone, Copy)]
struct RawByteSpan {
    source_offset: u64,
    byte_len: u64,
}

#[derive(Clone)]
struct ParsedFmt {
    chunk: Vec<u8>,
    block_align: u16,
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    encoding: SampleEncoding,
}

#[derive(Clone, Copy)]
enum SampleEncoding {
    Int,
    Float,
}

fn parse_layout<R: Read + Seek>(reader: &mut R) -> Result<Option<RawWavLayout>, String> {
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|err| format!("failed to seek WAV header: {err}"))?;

    let mut riff_header = [0_u8; 12];
    if !read_exact_or_missing(reader, &mut riff_header)? {
        return Ok(None);
    }
    if &riff_header[0..4] != RIFF_ID || &riff_header[8..12] != WAVE_ID {
        return Ok(None);
    }

    let mut fmt = None;
    loop {
        let Some(chunk) = read_chunk_header(reader)? else {
            return Ok(None);
        };
        let chunk_data_offset = reader
            .stream_position()
            .map_err(|err| format!("failed to inspect WAV chunk position: {err}"))?;
        if &chunk.id == FMT_ID {
            fmt = read_fmt_chunk(reader, chunk.size)?;
            if fmt.is_none() {
                return Ok(None);
            }
            skip_chunk_padding(reader, chunk.size)?;
            continue;
        }
        if &chunk.id == DATA_ID {
            let Some(fmt) = fmt else {
                return Ok(None);
            };
            let data_len = u64::from(chunk.size);
            if data_len == 0 || data_len % u64::from(fmt.block_align) != 0 {
                return Ok(None);
            }
            return Ok(Some(RawWavLayout {
                fmt_chunk: fmt.chunk,
                block_align: fmt.block_align,
                channels: fmt.channels,
                sample_rate: fmt.sample_rate,
                bits_per_sample: fmt.bits_per_sample,
                encoding: fmt.encoding,
                data_offset: chunk_data_offset,
                data_len,
            }));
        }
        skip_chunk_payload(reader, chunk.size)?;
    }
}

struct ChunkHeader {
    id: [u8; 4],
    size: u32,
}

fn read_chunk_header<R: Read>(reader: &mut R) -> Result<Option<ChunkHeader>, String> {
    let mut header = [0_u8; 8];
    if !read_exact_or_missing(reader, &mut header)? {
        return Ok(None);
    }
    Ok(Some(ChunkHeader {
        id: [header[0], header[1], header[2], header[3]],
        size: u32::from_le_bytes([header[4], header[5], header[6], header[7]]),
    }))
}

fn read_fmt_chunk<R: Read>(reader: &mut R, size: u32) -> Result<Option<ParsedFmt>, String> {
    if !(MIN_FMT_CHUNK_BYTES..=MAX_FMT_CHUNK_BYTES).contains(&size) {
        return Ok(None);
    }
    let mut chunk = vec![0_u8; size as usize];
    if !read_exact_or_missing(reader, &mut chunk)? {
        return Ok(None);
    }
    Ok(parse_fmt_chunk(chunk))
}

fn parse_fmt_chunk(chunk: Vec<u8>) -> Option<ParsedFmt> {
    let format_tag = le_u16(&chunk, 0)?;
    let channels = le_u16(&chunk, 2)?;
    let sample_rate = le_u32(&chunk, 4)?;
    let byte_rate = le_u32(&chunk, 8)?;
    let block_align = le_u16(&chunk, 12)?;
    let bits_per_sample = le_u16(&chunk, 14)?;
    let encoding = match format_tag {
        WAVE_FORMAT_PCM => SampleEncoding::Int,
        WAVE_FORMAT_IEEE_FLOAT => SampleEncoding::Float,
        WAVE_FORMAT_EXTENSIBLE => extensible_encoding(&chunk, bits_per_sample)?,
        _ => return None,
    };
    validate_audio_format(
        encoding,
        channels,
        sample_rate,
        byte_rate,
        block_align,
        bits_per_sample,
    )?;
    Some(ParsedFmt {
        chunk,
        block_align,
        channels,
        sample_rate,
        bits_per_sample,
        encoding,
    })
}

fn extensible_encoding(chunk: &[u8], container_bits_per_sample: u16) -> Option<SampleEncoding> {
    if chunk.len() < 40 {
        return None;
    }
    let extension_size = le_u16(chunk, 16)?;
    let valid_bits_per_sample = le_u16(chunk, 18)?;
    if extension_size < 22
        || valid_bits_per_sample == 0
        || valid_bits_per_sample > container_bits_per_sample
    {
        return None;
    }
    let subformat = chunk.get(24..40)?;
    if subformat.get(2..16)? != WAVE_SUBFORMAT_TAIL.as_slice() {
        return None;
    }
    match le_u16(subformat, 0)? {
        WAVE_FORMAT_PCM => Some(SampleEncoding::Int),
        WAVE_FORMAT_IEEE_FLOAT => Some(SampleEncoding::Float),
        _ => None,
    }
}

fn validate_audio_format(
    encoding: SampleEncoding,
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
) -> Option<()> {
    if channels == 0 || sample_rate == 0 || block_align == 0 || !bits_per_sample.is_multiple_of(8) {
        return None;
    }
    match encoding {
        SampleEncoding::Int if !matches!(bits_per_sample, 8 | 16 | 24 | 32) => return None,
        SampleEncoding::Float if bits_per_sample != 32 => return None,
        _ => {}
    }
    let bytes_per_sample = bits_per_sample / 8;
    let expected_block_align = channels.checked_mul(bytes_per_sample)?;
    if block_align != expected_block_align {
        return None;
    }
    let expected_byte_rate = u64::from(sample_rate) * u64::from(block_align);
    if u64::from(byte_rate) != expected_byte_rate {
        return None;
    }
    Some(())
}

fn write_raw_slice<R: Read + Seek>(
    reader: &mut R,
    layout: &RawWavLayout,
    span: RawByteSpan,
    output_path: &Path,
    gain: f32,
    fade_duration: Duration,
) -> Result<(), String> {
    let data_len = u32::try_from(span.byte_len)
        .map_err(|_| String::from("WAV selection is too large for RIFF output"))?;
    let riff_size = riff_payload_size(layout.fmt_chunk.len(), span.byte_len)
        .ok_or_else(|| String::from("WAV selection is too large for RIFF output"))?;
    reader
        .seek(SeekFrom::Start(span.source_offset))
        .map_err(|err| format!("failed to seek WAV selection: {err}"))?;

    let file = File::create(output_path).map_err(|err| {
        format!(
            "failed to create extraction {}: {err}",
            output_path.display()
        )
    })?;
    let mut writer = BufWriter::new(file);
    write_header(&mut writer, layout, data_len, riff_size)
        .map_err(|err| format!("failed to write extraction header: {err}"))?;

    let frame_count = span.byte_len / u64::from(layout.block_align);
    let fade_frames =
        short_edge_fade_frame_count(layout.sample_rate, frame_count as usize, fade_duration);
    if fade_frames == 0 && (gain - 1.0).abs() <= f32::EPSILON {
        copy_exact_data(reader, &mut writer, span.byte_len)?;
    } else {
        fade::write_faded_data(reader, &mut writer, layout, frame_count, fade_frames, gain)?;
    }
    if span.byte_len % 2 == 1 {
        writer
            .write_all(&[0])
            .map_err(|err| format!("failed to pad extraction data chunk: {err}"))?;
    }
    writer
        .flush()
        .map_err(|err| format!("failed to finalize extraction: {err}"))?;
    Ok(())
}

pub(super) fn copy_exact_data<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    byte_len: u64,
) -> Result<(), String> {
    let copied = {
        let mut limited = reader.take(byte_len);
        std::io::copy(&mut limited, writer)
            .map_err(|err| format!("failed to copy WAV selection data: {err}"))?
    };
    if copied != byte_len {
        return Err(String::from("failed to copy complete WAV selection data"));
    }
    Ok(())
}

fn write_header<W: Write>(
    writer: &mut W,
    layout: &RawWavLayout,
    data_len: u32,
    riff_size: u32,
) -> std::io::Result<()> {
    writer.write_all(RIFF_ID)?;
    writer.write_all(&riff_size.to_le_bytes())?;
    writer.write_all(WAVE_ID)?;
    writer.write_all(FMT_ID)?;
    writer.write_all(&(layout.fmt_chunk.len() as u32).to_le_bytes())?;
    writer.write_all(&layout.fmt_chunk)?;
    if layout.fmt_chunk.len() % 2 == 1 {
        writer.write_all(&[0])?;
    }
    writer.write_all(DATA_ID)?;
    writer.write_all(&data_len.to_le_bytes())?;
    Ok(())
}

fn riff_payload_size(fmt_len: usize, data_len: u64) -> Option<u32> {
    let fmt_len = u64::try_from(fmt_len).ok()?;
    let payload = 4_u64
        .checked_add(chunk_total_len(fmt_len)?)?
        .checked_add(chunk_total_len(data_len)?)?;
    u32::try_from(payload).ok()
}

fn chunk_total_len(data_len: u64) -> Option<u64> {
    let pad_byte = if data_len % 2 == 1 { 1 } else { 0 };
    8_u64.checked_add(data_len)?.checked_add(pad_byte)
}

fn skip_chunk_payload<R: Seek>(reader: &mut R, size: u32) -> Result<(), String> {
    let offset = i64::from(size) + i64::from(size % 2);
    reader
        .seek(SeekFrom::Current(offset))
        .map_err(|err| format!("failed to skip WAV chunk: {err}"))?;
    Ok(())
}

fn skip_chunk_padding<R: Seek>(reader: &mut R, size: u32) -> Result<(), String> {
    if size.is_multiple_of(2) {
        return Ok(());
    }
    reader
        .seek(SeekFrom::Current(1))
        .map_err(|err| format!("failed to skip WAV chunk padding: {err}"))?;
    Ok(())
}

fn read_exact_or_missing<R: Read>(reader: &mut R, buffer: &mut [u8]) -> Result<bool, String> {
    match reader.read_exact(buffer) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == ErrorKind::UnexpectedEof => Ok(false),
        Err(err) => Err(format!("failed to read WAV data: {err}")),
    }
}

fn le_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let bytes = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn le_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let bytes = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}
