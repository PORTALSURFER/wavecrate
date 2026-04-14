//! ANN container payload encoding, decoding, and checksum helpers.

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};

use super::{ANN_CONTAINER_CHECKSUM_LEN, MAX_MODEL_ID_LEN, header::AnnContainerHeader};

pub(super) fn encode_id_map(id_map: &[String]) -> Result<Vec<u8>, String> {
    serde_json::to_vec(id_map).map_err(|err| format!("Failed to encode ANN id map: {err}"))
}

pub(super) fn decode_id_map(bytes: &[u8]) -> Result<Vec<String>, String> {
    serde_json::from_slice(bytes).map_err(|err| format!("Failed to decode ANN id map: {err}"))
}

pub(super) fn write_model_id(
    file: &mut File,
    model_id: &[u8],
    hasher: &mut Sha256,
) -> Result<(), String> {
    file.write_all(model_id)
        .map_err(|err| format!("Failed to write ANN model id: {err}"))?;
    hasher.update(model_id);
    Ok(())
}

pub(super) fn read_model_id(
    file: &mut File,
    header: &AnnContainerHeader,
    hasher: &mut Sha256,
) -> Result<String, String> {
    if header.model_id_len > MAX_MODEL_ID_LEN {
        return Err(format!(
            "ANN container model id length too large: {} bytes (max {})",
            header.model_id_len, MAX_MODEL_ID_LEN
        ));
    }
    let mut model_id = vec![0u8; header.model_id_len as usize];
    file.read_exact(&mut model_id)
        .map_err(|err| format!("Failed to read ANN model id: {err}"))?;
    hasher.update(&model_id);
    String::from_utf8(model_id).map_err(|err| format!("ANN model id invalid UTF-8: {err}"))
}

pub(super) fn write_id_map(
    file: &mut File,
    bytes: &[u8],
    hasher: &mut Sha256,
) -> Result<(), String> {
    file.write_all(bytes)
        .map_err(|err| format!("Failed to write ANN id map: {err}"))?;
    hasher.update(bytes);
    Ok(())
}

pub(super) fn verify_checksum(header: &AnnContainerHeader, hasher: Sha256) -> Result<(), String> {
    let checksum: [u8; ANN_CONTAINER_CHECKSUM_LEN] = hasher.finalize().into();
    if checksum != header.checksum {
        return Err("ANN container checksum mismatch".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};
    use tempfile::tempfile;

    fn test_header(model_id_len: u32) -> AnnContainerHeader {
        AnnContainerHeader {
            model_id_len,
            graph_offset: 0,
            graph_len: 0,
            data_offset: 0,
            data_len: 0,
            id_map_offset: 0,
            id_map_len: 0,
            checksum: [0u8; ANN_CONTAINER_CHECKSUM_LEN],
        }
    }

    #[test]
    fn ann_container_rejects_checksum_mismatch() {
        let mut hasher = Sha256::new();
        hasher.update(b"payload");
        let err = verify_checksum(&test_header(0), hasher).unwrap_err();
        assert!(err.contains("checksum mismatch"));
    }

    #[test]
    fn ann_container_rejects_invalid_utf8_model_id() {
        let mut file = tempfile().expect("temp model file");
        file.write_all(&[0xff, 0xfe]).expect("write model id bytes");
        file.seek(SeekFrom::Start(0)).expect("rewind model file");
        let mut hasher = Sha256::new();
        let err = read_model_id(&mut file, &test_header(2), &mut hasher).unwrap_err();
        assert!(err.contains("invalid UTF-8"));
    }
}
