//! Narrow WAV header repair helpers for streaming and in-memory decode paths.

use std::fs::File;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

const MAX_SANITIZED_WAV_BYTES: u64 = 64 * 1024 * 1024;
const SANITIZE_PROBE_BYTES: u64 = 4096;

/// A WAV reader that exposes a logically repaired byte stream.
///
/// The reader inspects only the first [`SANITIZE_PROBE_BYTES`] bytes and repairs
/// one narrow class of malformed headers: PCM or IEEE-float `fmt ` chunks whose
/// declared size is larger than 18 bytes but whose extra bytes are all zero
/// padding after a zero `cbSize` field. Unsupported malformed inputs are passed
/// through unchanged and may still fail in downstream decoders.
///
/// When a repair is applied, [`Read`] and [`Seek`] operate on the sanitized
/// logical stream, not the original on-disk offsets. In particular, seeking
/// back into the header rewinds the chained file body so later reads continue at
/// the correct logical position.
pub enum SanitizedWavReader {
    /// Read directly from the original file when no repair is necessary.
    PassThrough(File),
    /// Read a sanitized in-memory header followed by the original file body.
    Chained {
        /// In-memory header bytes for the repaired logical stream.
        header: Cursor<Vec<u8>>,
        /// File handle positioned at the first unsanitized body byte.
        file: File,
        /// Logical header length after repair.
        header_len: u64,
        /// Physical file offset where the body begins.
        file_start_offset: u64,
    },
}

impl SanitizedWavReader {
    fn current_logical_position(&mut self) -> io::Result<u64> {
        match self {
            SanitizedWavReader::PassThrough(file) => file.stream_position(),
            SanitizedWavReader::Chained {
                header,
                file,
                header_len,
                file_start_offset,
            } => chained_logical_position(header.position(), file, *header_len, *file_start_offset),
        }
    }

    fn logical_len(&mut self) -> io::Result<u64> {
        match self {
            SanitizedWavReader::PassThrough(file) => file.seek(SeekFrom::End(0)),
            SanitizedWavReader::Chained {
                file,
                header_len,
                file_start_offset,
                ..
            } => chained_logical_len(file, *header_len, *file_start_offset),
        }
    }

    fn seek_logical_start(&mut self, offset: u64) -> io::Result<u64> {
        match self {
            SanitizedWavReader::PassThrough(file) => file.seek(SeekFrom::Start(offset)),
            SanitizedWavReader::Chained {
                header,
                file,
                header_len,
                file_start_offset,
            } => seek_chained_start(header, file, *header_len, *file_start_offset, offset),
        }
    }
}

impl Read for SanitizedWavReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            SanitizedWavReader::PassThrough(file) => file.read(buf),
            SanitizedWavReader::Chained {
                header,
                file,
                header_len,
                file_start_offset,
            } => {
                let read = header.read(buf)?;
                if read > 0 {
                    if header.position() == *header_len {
                        file.seek(SeekFrom::Start(*file_start_offset))?;
                    }
                    return Ok(read);
                }
                file.read(buf)
            }
        }
    }
}

impl Seek for SanitizedWavReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Start(offset) => self.seek_logical_start(offset),
            SeekFrom::Current(delta) => {
                let current = self.current_logical_position()?;
                let target = checked_seek_target(current, delta)?;
                self.seek_logical_start(target)
            }
            SeekFrom::End(delta) => {
                let len = self.logical_len()?;
                let target = checked_seek_target(len, delta)?;
                self.seek_logical_start(target)
            }
        }
    }
}

/// Open a WAV file for streaming reads while repairing the supported malformed
/// `fmt ` header variants.
///
/// The reader probes only the first [`SANITIZE_PROBE_BYTES`] bytes. If repair is
/// possible, the returned stream exposes the repaired logical bytes and keeps
/// `Seek` relative to that repaired stream. If no supported repair applies, the
/// original file is returned unchanged.
pub fn open_sanitized_wav(path: &Path) -> Result<SanitizedWavReader, String> {
    let mut file =
        File::open(path).map_err(|err| format!("Failed to open {}: {err}", path.display()))?;

    let mut buffer = Vec::with_capacity(SANITIZE_PROBE_BYTES as usize);
    let mut chunk = file.by_ref().take(SANITIZE_PROBE_BYTES);
    chunk
        .read_to_end(&mut buffer)
        .map_err(|err| format!("Failed to read header of {}: {err}", path.display()))?;

    let read_len = buffer.len();
    if read_len == 0 {
        return Ok(SanitizedWavReader::PassThrough(file));
    }

    let total_size = file
        .metadata()
        .map(|meta| meta.len())
        .unwrap_or(read_len as u64);
    if sanitize_wav_header(&mut buffer, total_size) {
        let header_len = buffer.len() as u64;
        Ok(SanitizedWavReader::Chained {
            header: Cursor::new(buffer),
            file,
            header_len,
            file_start_offset: read_len as u64,
        })
    } else {
        file.seek(SeekFrom::Start(0))
            .map_err(|err| format!("Failed to rewind {}: {err}", path.display()))?;
        Ok(SanitizedWavReader::PassThrough(file))
    }
}

/// Attempt to repair a supported malformed `fmt ` chunk inside a buffered WAV
/// header prefix.
///
/// The function returns `true` only when the buffer is modified. It never scans
/// beyond `total_file_len`, and it stops once the `fmt ` chunk falls outside the
/// buffered prefix because the unsupported bytes cannot be inspected safely.
fn sanitize_wav_header(bytes: &mut Vec<u8>, total_file_len: u64) -> bool {
    if bytes.len() < 12 {
        return false;
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return false;
    }

    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let chunk_id = [
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ];
        let chunk_size =
            u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let chunk_data = match offset.checked_add(8) {
            Some(value) => value,
            None => return false,
        };
        let chunk_end = match chunk_data.checked_add(chunk_size) {
            Some(value) => value,
            None => return false,
        };
        if (chunk_data as u64)
            .checked_add(chunk_size as u64)
            .is_none_or(|end| end > total_file_len)
        {
            return false;
        }
        if chunk_end > bytes.len() {
            break;
        }
        if &chunk_id == b"fmt " {
            return shrink_pcm_fmt_chunk_with_padding(bytes, offset, chunk_size, total_file_len);
        }

        offset = chunk_end;
        if chunk_size % 2 == 1 {
            offset = match offset.checked_add(1) {
                Some(value) => value,
                None => return false,
            };
        }
    }

    false
}

/// Read an entire WAV file into memory and apply the same narrow header repair
/// used by [`open_sanitized_wav`].
///
/// This helper is intended for small files and tests. It rejects files larger
/// than [`MAX_SANITIZED_WAV_BYTES`] instead of streaming them.
pub fn read_sanitized_wav_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let file_len = std::fs::metadata(path)
        .map(|meta| meta.len())
        .map_err(|err| format!("Failed to stat {}: {err}", path.display()))?;
    if file_len > MAX_SANITIZED_WAV_BYTES {
        return Err(format!(
            "Refusing to read {} ({} bytes) into memory; cap is {} bytes",
            path.display(),
            file_len,
            MAX_SANITIZED_WAV_BYTES
        ));
    }
    let mut bytes =
        std::fs::read(path).map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let len = bytes.len() as u64;
    sanitize_wav_header(&mut bytes, len);
    Ok(bytes)
}

/// Shrink an over-padded PCM or IEEE-float `fmt ` chunk down to the canonical
/// 18-byte WaveFormatEx payload when the extra bytes are provably zero padding.
fn shrink_pcm_fmt_chunk_with_padding(
    bytes: &mut Vec<u8>,
    chunk_offset: usize,
    chunk_size: usize,
    total_file_len: u64,
) -> bool {
    if chunk_size <= 18 || !chunk_size.is_multiple_of(2) {
        return false;
    }
    let fmt_data = chunk_offset + 8;
    if fmt_data + chunk_size > bytes.len() {
        return false;
    }

    let format_tag = match bytes[fmt_data..fmt_data + 2].try_into() {
        Ok(raw) => u16::from_le_bytes(raw),
        Err(_) => return false,
    };
    if !matches!(format_tag, 1 | 3) {
        return false;
    }

    let cb_size_offset = fmt_data + 16;
    if cb_size_offset + 2 > bytes.len() {
        return false;
    }
    let cb_size = u16::from_le_bytes(
        bytes[cb_size_offset..cb_size_offset + 2]
            .try_into()
            .unwrap(),
    );
    if cb_size != 0 {
        return false;
    }
    if !bytes[fmt_data + 18..fmt_data + chunk_size]
        .iter()
        .all(|byte| *byte == 0)
    {
        return false;
    }

    bytes[chunk_offset + 4..chunk_offset + 8].copy_from_slice(&(18u32).to_le_bytes());
    bytes.drain(fmt_data + 18..fmt_data + chunk_size);

    let removed_count = chunk_size - 18;
    let new_len = total_file_len.saturating_sub(removed_count as u64);
    if bytes.len() >= 8 {
        let riff_size = (new_len.saturating_sub(8) as u32).to_le_bytes();
        bytes[4..8].copy_from_slice(&riff_size);
    }

    true
}

/// Repair the supported malformed `fmt ` chunk variants inside an in-memory WAV
/// byte buffer and return the updated bytes.
pub fn sanitize_wav_bytes(mut bytes: Vec<u8>) -> Vec<u8> {
    let len = bytes.len() as u64;
    sanitize_wav_header(&mut bytes, len);
    bytes
}

fn checked_seek_target(base: u64, delta: i64) -> io::Result<u64> {
    let target = if delta >= 0 {
        base.checked_add(delta as u64)
    } else {
        base.checked_sub(delta.unsigned_abs())
    };
    target.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid seek to a negative or overflowed position",
        )
    })
}

fn chained_logical_position(
    header_position: u64,
    file: &mut File,
    header_len: u64,
    file_start_offset: u64,
) -> io::Result<u64> {
    if header_position < header_len {
        return Ok(header_position);
    }
    let body_offset = file_body_offset(file, file_start_offset)?;
    header_len
        .checked_add(body_offset)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "logical position overflow"))
}

fn chained_logical_len(
    file: &mut File,
    header_len: u64,
    file_start_offset: u64,
) -> io::Result<u64> {
    let file_end = file.seek(SeekFrom::End(0))?;
    let body_len = file_end.checked_sub(file_start_offset).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "sanitized wav body starts after end of file",
        )
    })?;
    header_len
        .checked_add(body_len)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "logical length overflow"))
}

fn seek_chained_start(
    header: &mut Cursor<Vec<u8>>,
    file: &mut File,
    header_len: u64,
    file_start_offset: u64,
    offset: u64,
) -> io::Result<u64> {
    if offset < header_len {
        header.set_position(offset);
        file.seek(SeekFrom::Start(file_start_offset))?;
        return Ok(offset);
    }

    header.set_position(header_len);
    let body_offset = offset.checked_sub(header_len).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "invalid logical body offset")
    })?;
    let physical_offset = file_start_offset.checked_add(body_offset).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "physical seek position overflow",
        )
    })?;
    file.seek(SeekFrom::Start(physical_offset))?;
    Ok(offset)
}

fn file_body_offset(file: &mut File, file_start_offset: u64) -> io::Result<u64> {
    let position = file.stream_position()?;
    position.checked_sub(file_start_offset).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "sanitized wav body position drifted before body start",
        )
    })
}

#[cfg(test)]
#[path = "wav_sanitize/tests.rs"]
mod tests;
