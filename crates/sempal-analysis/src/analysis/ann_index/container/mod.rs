//! Thin ANN container facade over header, codec, and streaming helpers.

mod codec;
mod header;
mod stream_io;

use sha2::{Digest, Sha256};
use std::path::Path;

use codec::{
    decode_id_map, encode_id_map, read_model_id, verify_checksum, write_id_map, write_model_id,
};
use header::{AnnContainerHeader, write_header_at_start};
use stream_io::{
    copy_range_with_hash, copy_with_hash, create_tempfile, dump_paths_for, file_len,
    file_len_from_file, open_container, read_range, write_placeholder_header,
};

const ANN_CONTAINER_MAGIC: &[u8; 8] = b"SANNIDX1";
const ANN_CONTAINER_VERSION: u32 = 1;
const ANN_CONTAINER_CHECKSUM_LEN: usize = 32;
const ANN_CONTAINER_HEADER_LEN: usize = 8 + 4 + 4 + 4 + 4 + (8 * 6) + ANN_CONTAINER_CHECKSUM_LEN;
const MAX_MODEL_ID_LEN: u32 = 16 * 1024;
const MAX_ID_MAP_LEN: u64 = 16 * 1024 * 1024;

/// Payload extracted from an ANN container.
pub(crate) struct AnnContainerUnpack {
    pub(crate) model_id: String,
    pub(crate) id_map: Vec<String>,
}

/// Write a single-file ANN container from HNSW dumps and id map data.
pub(crate) fn write_container(
    path: &Path,
    model_id: &str,
    graph_path: &Path,
    data_path: &Path,
    id_map: &[String],
) -> Result<(), String> {
    let graph_len = file_len(graph_path)?;
    let data_len = file_len(data_path)?;
    let id_map_bytes = encode_id_map(id_map)?;
    let model_id_bytes = model_id.as_bytes();
    if model_id_bytes.len() > MAX_MODEL_ID_LEN as usize {
        return Err(format!(
            "ANN model id too long: {} bytes (max {})",
            model_id_bytes.len(),
            MAX_MODEL_ID_LEN
        ));
    }
    if id_map_bytes.len() as u64 > MAX_ID_MAP_LEN {
        return Err(format!(
            "ANN id map too large: {} bytes (max {})",
            id_map_bytes.len(),
            MAX_ID_MAP_LEN
        ));
    }
    let header = AnnContainerHeader::new(
        model_id_bytes.len(),
        graph_len,
        data_len,
        id_map_bytes.len(),
    );
    let mut temp = create_tempfile(path)?;
    let file = temp.as_file_mut();
    write_placeholder_header(file)?;
    let mut hasher = Sha256::new();
    write_model_id(file, model_id_bytes, &mut hasher)?;
    copy_with_hash(graph_path, file, &mut hasher)?;
    copy_with_hash(data_path, file, &mut hasher)?;
    write_id_map(file, &id_map_bytes, &mut hasher)?;
    let mut final_header = header;
    final_header.checksum = hasher.finalize().into();
    write_header_at_start(file, final_header)?;
    temp.persist(path)
        .map_err(|err| format!("Failed to persist ANN container: {err}"))?;
    Ok(())
}

/// Unpack an ANN container into HNSW dump files, returning the id map.
pub(crate) fn unpack_container(
    path: &Path,
    output_dir: &Path,
    basename: &str,
) -> Result<AnnContainerUnpack, String> {
    let mut file = open_container(path)?;
    let file_len = file_len_from_file(&file)?;
    let header = AnnContainerHeader::read(&mut file)?;
    header.validate(file_len)?;
    let mut hasher = Sha256::new();
    let model_id = read_model_id(&mut file, &header, &mut hasher)?;
    let id_map_bytes = unpack_payload(&mut file, &header, output_dir, basename, &mut hasher)?;
    verify_checksum(&header, hasher)?;
    let id_map = decode_id_map(&id_map_bytes)?;
    Ok(AnnContainerUnpack { model_id, id_map })
}

fn unpack_payload(
    file: &mut std::fs::File,
    header: &AnnContainerHeader,
    output_dir: &Path,
    basename: &str,
    hasher: &mut Sha256,
) -> Result<Vec<u8>, String> {
    let (graph_path, data_path) = dump_paths_for(output_dir, basename);
    copy_range_with_hash(
        file,
        header.graph_offset,
        header.graph_len,
        &graph_path,
        hasher,
    )?;
    copy_range_with_hash(
        file,
        header.data_offset,
        header.data_len,
        &data_path,
        hasher,
    )?;
    let id_map_bytes = read_range(file, header.id_map_offset, header.id_map_len)?;
    hasher.update(&id_map_bytes);
    Ok(id_map_bytes)
}
