#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct WavChunk {
    id: [u8; 4],
    offset: usize,
    data_offset: usize,
    data_size: u32,
    data_end: usize,
    next_offset: usize,
}

impl WavChunk {
    pub(super) fn id(&self) -> &[u8; 4] {
        &self.id
    }

    pub(super) fn offset(&self) -> usize {
        self.offset
    }

    pub(super) fn data_size(&self) -> usize {
        self.data_size as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WavChunkItem {
    Chunk(WavChunk),
    IncompletePrefix { offset: usize },
    Invalid(WavChunkError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WavHeaderError {
    HeaderTooShort,
    NotRiffWave,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WavChunkError {
    TruncatedHeader {
        offset: usize,
    },
    ChunkDataOutOfFile {
        offset: usize,
        data_offset: usize,
        data_size: u32,
        total_file_len: u64,
    },
    OffsetOverflow {
        offset: usize,
    },
}

#[derive(Debug)]
pub(super) struct WavChunkIter<'a> {
    bytes: &'a [u8],
    total_file_len: u64,
    offset: usize,
    done: bool,
}

impl<'a> WavChunkIter<'a> {
    pub(super) fn new(bytes: &'a [u8], total_file_len: u64) -> Result<Self, WavHeaderError> {
        if bytes.len() < 12 {
            return Err(WavHeaderError::HeaderTooShort);
        }
        if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(WavHeaderError::NotRiffWave);
        }
        Ok(Self {
            bytes,
            total_file_len,
            offset: 12,
            done: false,
        })
    }
}

impl Iterator for WavChunkIter<'_> {
    type Item = WavChunkItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        if self.offset as u64 >= self.total_file_len {
            self.done = true;
            return None;
        }
        if self.offset + 8 > self.bytes.len() {
            self.done = true;
            if self.bytes.len() as u64 >= self.total_file_len {
                return Some(WavChunkItem::Invalid(WavChunkError::TruncatedHeader {
                    offset: self.offset,
                }));
            }
            return Some(WavChunkItem::IncompletePrefix {
                offset: self.offset,
            });
        }

        let id = [
            self.bytes[self.offset],
            self.bytes[self.offset + 1],
            self.bytes[self.offset + 2],
            self.bytes[self.offset + 3],
        ];
        let data_size = u32::from_le_bytes(
            self.bytes[self.offset + 4..self.offset + 8]
                .try_into()
                .unwrap(),
        );
        let Some(data_offset) = self.offset.checked_add(8) else {
            self.done = true;
            return Some(WavChunkItem::Invalid(WavChunkError::OffsetOverflow {
                offset: self.offset,
            }));
        };
        let data_end_u64 = data_offset as u64 + data_size as u64;
        if data_end_u64 > self.total_file_len {
            self.done = true;
            return Some(WavChunkItem::Invalid(WavChunkError::ChunkDataOutOfFile {
                offset: self.offset,
                data_offset,
                data_size,
                total_file_len: self.total_file_len,
            }));
        }
        let Ok(data_end) = usize::try_from(data_end_u64) else {
            self.done = true;
            return Some(WavChunkItem::Invalid(WavChunkError::OffsetOverflow {
                offset: self.offset,
            }));
        };
        if data_end > self.bytes.len() {
            self.done = true;
            return Some(WavChunkItem::IncompletePrefix {
                offset: self.offset,
            });
        }

        let next_offset = match padded_next_offset(data_end, data_size) {
            Some(offset) => offset,
            None => {
                self.done = true;
                return Some(WavChunkItem::Invalid(WavChunkError::OffsetOverflow {
                    offset: self.offset,
                }));
            }
        };
        let chunk = WavChunk {
            id,
            offset: self.offset,
            data_offset,
            data_size,
            data_end,
            next_offset,
        };
        self.offset = next_offset;
        Some(WavChunkItem::Chunk(chunk))
    }
}

fn padded_next_offset(data_end: usize, data_size: u32) -> Option<usize> {
    if data_size % 2 == 1 {
        data_end.checked_add(1)
    } else {
        Some(data_end)
    }
}

#[cfg(test)]
mod tests;
