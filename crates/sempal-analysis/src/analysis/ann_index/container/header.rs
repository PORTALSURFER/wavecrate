//! ANN container header encoding, parsing, and validation.

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use super::{
    ANN_CONTAINER_CHECKSUM_LEN, ANN_CONTAINER_HEADER_LEN, ANN_CONTAINER_MAGIC,
    ANN_CONTAINER_VERSION, MAX_ID_MAP_LEN, MAX_MODEL_ID_LEN,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct AnnContainerHeader {
    pub(super) model_id_len: u32,
    pub(super) graph_offset: u64,
    pub(super) graph_len: u64,
    pub(super) data_offset: u64,
    pub(super) data_len: u64,
    pub(super) id_map_offset: u64,
    pub(super) id_map_len: u64,
    pub(super) checksum: [u8; ANN_CONTAINER_CHECKSUM_LEN],
}

impl AnnContainerHeader {
    pub(super) fn new(
        model_id_len: usize,
        graph_len: u64,
        data_len: u64,
        id_map_len: usize,
    ) -> Self {
        let model_id_len = model_id_len as u32;
        let graph_offset = (ANN_CONTAINER_HEADER_LEN as u64) + (model_id_len as u64);
        let data_offset = graph_offset + graph_len;
        let id_map_offset = data_offset + data_len;
        Self {
            model_id_len,
            graph_offset,
            graph_len,
            data_offset,
            data_len,
            id_map_offset,
            id_map_len: id_map_len as u64,
            checksum: [0u8; ANN_CONTAINER_CHECKSUM_LEN],
        }
    }

    pub(super) fn read(file: &mut File) -> Result<Self, String> {
        let header_len = read_header_prefix(file)?;
        let rest = read_header_rest(file, header_len)?;
        parse_header(rest)
    }

    pub(super) fn validate(&self, file_len: u64) -> Result<(), String> {
        if self.model_id_len > MAX_MODEL_ID_LEN {
            return Err(format!(
                "ANN container model id length too large: {} bytes (max {})",
                self.model_id_len, MAX_MODEL_ID_LEN
            ));
        }
        if self.id_map_len > MAX_ID_MAP_LEN {
            return Err(format!(
                "ANN container id map length too large: {} bytes (max {})",
                self.id_map_len, MAX_ID_MAP_LEN
            ));
        }
        let expected_graph_offset = (ANN_CONTAINER_HEADER_LEN as u64)
            .checked_add(self.model_id_len as u64)
            .ok_or_else(|| "ANN container header length overflow".to_string())?;
        if self.graph_offset != expected_graph_offset {
            return Err("ANN container graph offset mismatch".to_string());
        }
        let graph_end = self
            .graph_offset
            .checked_add(self.graph_len)
            .ok_or_else(|| "ANN container graph section overflow".to_string())?;
        if graph_end > self.data_offset {
            return Err("ANN container graph section overlaps data".to_string());
        }
        let data_end = self
            .data_offset
            .checked_add(self.data_len)
            .ok_or_else(|| "ANN container data section overflow".to_string())?;
        if data_end > self.id_map_offset {
            return Err("ANN container data section overlaps id map".to_string());
        }
        let end = self
            .id_map_offset
            .checked_add(self.id_map_len)
            .ok_or_else(|| "ANN container id map section overflow".to_string())?;
        if end > file_len {
            return Err("ANN container payload exceeds file length".to_string());
        }
        Ok(())
    }
}

pub(super) fn write_header_at_start(
    file: &mut File,
    header: AnnContainerHeader,
) -> Result<(), String> {
    file.seek(SeekFrom::Start(0))
        .map_err(|err| format!("Failed to seek ANN container: {err}"))?;
    let buf = build_header_bytes(header);
    file.write_all(&buf)
        .map_err(|err| format!("Failed to write ANN header: {err}"))?;
    Ok(())
}

fn build_header_bytes(header: AnnContainerHeader) -> Vec<u8> {
    let mut buf = Vec::with_capacity(ANN_CONTAINER_HEADER_LEN);
    buf.extend_from_slice(ANN_CONTAINER_MAGIC);
    buf.extend_from_slice(&ANN_CONTAINER_VERSION.to_le_bytes());
    buf.extend_from_slice(&(ANN_CONTAINER_HEADER_LEN as u32).to_le_bytes());
    buf.extend_from_slice(&header.model_id_len.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&header.graph_offset.to_le_bytes());
    buf.extend_from_slice(&header.graph_len.to_le_bytes());
    buf.extend_from_slice(&header.data_offset.to_le_bytes());
    buf.extend_from_slice(&header.data_len.to_le_bytes());
    buf.extend_from_slice(&header.id_map_offset.to_le_bytes());
    buf.extend_from_slice(&header.id_map_len.to_le_bytes());
    buf.extend_from_slice(&header.checksum);
    buf
}

fn read_header_prefix(file: &mut File) -> Result<usize, String> {
    let mut prefix = [0u8; 16];
    file.read_exact(&mut prefix)
        .map_err(|err| format!("Failed to read ANN container header: {err}"))?;
    if &prefix[..8] != ANN_CONTAINER_MAGIC {
        return Err("ANN container magic mismatch".to_string());
    }
    let version = u32::from_le_bytes(prefix[8..12].try_into().unwrap());
    if version != ANN_CONTAINER_VERSION {
        return Err(format!("ANN container version mismatch: {version}"));
    }
    let header_len = u32::from_le_bytes(prefix[12..16].try_into().unwrap()) as usize;
    if header_len != ANN_CONTAINER_HEADER_LEN {
        return Err(format!(
            "ANN container header length mismatch: {header_len}"
        ));
    }
    Ok(header_len)
}

fn read_header_rest(file: &mut File, header_len: usize) -> Result<Vec<u8>, String> {
    let mut rest = vec![0u8; header_len - 16];
    file.read_exact(&mut rest)
        .map_err(|err| format!("Failed to read ANN container header: {err}"))?;
    Ok(rest)
}

fn parse_header(rest: Vec<u8>) -> Result<AnnContainerHeader, String> {
    let mut cursor = Cursor::new(rest);
    let model_id_len = read_u32(&mut cursor)?;
    let _reserved = read_u32(&mut cursor)?;
    let graph_offset = read_u64(&mut cursor)?;
    let graph_len = read_u64(&mut cursor)?;
    let data_offset = read_u64(&mut cursor)?;
    let data_len = read_u64(&mut cursor)?;
    let id_map_offset = read_u64(&mut cursor)?;
    let id_map_len = read_u64(&mut cursor)?;
    let mut checksum = [0u8; ANN_CONTAINER_CHECKSUM_LEN];
    cursor
        .read_exact(&mut checksum)
        .map_err(|err| format!("Failed to read ANN checksum: {err}"))?;
    Ok(AnnContainerHeader {
        model_id_len,
        graph_offset,
        graph_len,
        data_offset,
        data_len,
        id_map_offset,
        id_map_len,
        checksum,
    })
}

fn read_u32(reader: &mut dyn Read) -> Result<u32, String> {
    let mut buf = [0u8; 4];
    reader
        .read_exact(&mut buf)
        .map_err(|err| format!("Failed to read ANN header: {err}"))?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(reader: &mut dyn Read) -> Result<u64, String> {
    let mut buf = [0u8; 8];
    reader
        .read_exact(&mut buf)
        .map_err(|err| format!("Failed to read ANN header: {err}"))?;
    Ok(u64::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempfile;

    fn write_header_prefix_bytes(magic: &[u8; 8], version: u32, header_len: u32) -> File {
        let mut file = tempfile().expect("temp header file");
        file.write_all(magic).expect("write magic");
        file.write_all(&version.to_le_bytes())
            .expect("write version");
        file.write_all(&header_len.to_le_bytes())
            .expect("write header length");
        file.seek(SeekFrom::Start(0)).expect("rewind header file");
        file
    }

    #[test]
    fn ann_container_rejects_oversized_model_id_len() {
        let model_id_len = MAX_MODEL_ID_LEN + 1;
        let graph_offset = (ANN_CONTAINER_HEADER_LEN as u64)
            .checked_add(model_id_len as u64)
            .unwrap();
        let header = AnnContainerHeader {
            model_id_len,
            graph_offset,
            graph_len: 0,
            data_offset: graph_offset,
            data_len: 0,
            id_map_offset: graph_offset,
            id_map_len: 0,
            checksum: [0u8; ANN_CONTAINER_CHECKSUM_LEN],
        };
        let err = header.validate(graph_offset).unwrap_err();
        assert!(err.contains("model id length"));
    }

    #[test]
    fn ann_container_rejects_offset_overflow() {
        let header = AnnContainerHeader {
            model_id_len: 0,
            graph_offset: ANN_CONTAINER_HEADER_LEN as u64,
            graph_len: 0,
            data_offset: ANN_CONTAINER_HEADER_LEN as u64,
            data_len: 0,
            id_map_offset: u64::MAX - 1,
            id_map_len: 10,
            checksum: [0u8; ANN_CONTAINER_CHECKSUM_LEN],
        };
        let err = header.validate(u64::MAX).unwrap_err();
        assert!(err.contains("id map section overflow"));
    }

    #[test]
    fn ann_container_rejects_inconsistent_graph_offset() {
        let header = AnnContainerHeader {
            model_id_len: 4,
            graph_offset: ANN_CONTAINER_HEADER_LEN as u64 + 1,
            graph_len: 0,
            data_offset: ANN_CONTAINER_HEADER_LEN as u64 + 1,
            data_len: 0,
            id_map_offset: ANN_CONTAINER_HEADER_LEN as u64 + 1,
            id_map_len: 0,
            checksum: [0u8; ANN_CONTAINER_CHECKSUM_LEN],
        };
        let err = header.validate(u64::MAX).unwrap_err();
        assert!(err.contains("graph offset mismatch"));
    }

    #[test]
    fn ann_container_rejects_oversized_id_map_len() {
        let graph_offset = ANN_CONTAINER_HEADER_LEN as u64;
        let header = AnnContainerHeader {
            model_id_len: 0,
            graph_offset,
            graph_len: 0,
            data_offset: graph_offset,
            data_len: 0,
            id_map_offset: graph_offset,
            id_map_len: MAX_ID_MAP_LEN + 1,
            checksum: [0u8; ANN_CONTAINER_CHECKSUM_LEN],
        };
        let err = header.validate(u64::MAX).unwrap_err();
        assert!(err.contains("id map length"));
    }

    #[test]
    fn ann_container_rejects_magic_mismatch() {
        let mut file = write_header_prefix_bytes(b"BADMAGIC", ANN_CONTAINER_VERSION, 16);
        let err = read_header_prefix(&mut file).unwrap_err();
        assert!(err.contains("magic mismatch"));
    }

    #[test]
    fn ann_container_rejects_version_mismatch() {
        let mut file = write_header_prefix_bytes(
            ANN_CONTAINER_MAGIC,
            ANN_CONTAINER_VERSION + 1,
            ANN_CONTAINER_HEADER_LEN as u32,
        );
        let err = read_header_prefix(&mut file).unwrap_err();
        assert!(err.contains("version mismatch"));
    }
}
