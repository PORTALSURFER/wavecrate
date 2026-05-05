//! ANN container streaming IO and filesystem helpers.

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use super::ANN_CONTAINER_HEADER_LEN;

pub(super) fn open_container(path: &Path) -> Result<File, String> {
    File::open(path).map_err(|err| format!("Failed to open ANN container: {err}"))
}

pub(super) fn file_len(path: &Path) -> Result<u64, String> {
    let meta = std::fs::metadata(path)
        .map_err(|err| format!("Failed to read ANN payload metadata: {err}"))?;
    Ok(meta.len())
}

pub(super) fn file_len_from_file(file: &File) -> Result<u64, String> {
    file.metadata()
        .map(|meta| meta.len())
        .map_err(|err| format!("Failed to read ANN container metadata: {err}"))
}

pub(super) fn create_tempfile(path: &Path) -> Result<tempfile::NamedTempFile, String> {
    let dir = path
        .parent()
        .ok_or_else(|| "ANN container path missing parent".to_string())?;
    tempfile::Builder::new()
        .prefix("ann_container")
        .tempfile_in(dir)
        .map_err(|err| format!("Failed to create ANN tempfile: {err}"))
}

pub(super) fn write_placeholder_header(file: &mut File) -> Result<(), String> {
    let zeros = vec![0u8; ANN_CONTAINER_HEADER_LEN];
    file.write_all(&zeros)
        .map_err(|err| format!("Failed to write ANN header placeholder: {err}"))
}

pub(super) fn copy_with_hash(
    src: &Path,
    out: &mut File,
    hasher: &mut Sha256,
) -> Result<(), String> {
    let mut input = File::open(src).map_err(|err| format!("Failed to open ANN data: {err}"))?;
    copy_reader_with_hash(&mut input, out, hasher)
}

pub(super) fn copy_range_with_hash(
    file: &mut File,
    offset: u64,
    len: u64,
    out_path: &Path,
    hasher: &mut Sha256,
) -> Result<(), String> {
    file.seek(SeekFrom::Start(offset))
        .map_err(|err| format!("Failed to seek ANN container: {err}"))?;
    let mut reader = file.take(len);
    let mut out =
        File::create(out_path).map_err(|err| format!("Failed to write ANN payload: {err}"))?;
    copy_reader_with_hash(&mut reader, &mut out, hasher)
}

pub(super) fn read_range(file: &mut File, offset: u64, len: u64) -> Result<Vec<u8>, String> {
    let len =
        usize::try_from(len).map_err(|_| "ANN container payload too large to read".to_string())?;
    let mut buf = vec![0u8; len];
    file.seek(SeekFrom::Start(offset))
        .map_err(|err| format!("Failed to seek ANN container: {err}"))?;
    file.read_exact(&mut buf)
        .map_err(|err| format!("Failed to read ANN payload: {err}"))?;
    Ok(buf)
}

pub(super) fn dump_paths_for(dir: &Path, basename: &str) -> (PathBuf, PathBuf) {
    (
        dir.join(format!("{basename}.hnsw.graph")),
        dir.join(format!("{basename}.hnsw.data")),
    )
}

fn copy_reader_with_hash(
    reader: &mut dyn Read,
    out: &mut File,
    hasher: &mut Sha256,
) -> Result<(), String> {
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|err| format!("Failed to read ANN payload: {err}"))?;
        if read == 0 {
            break;
        }
        out.write_all(&buffer[..read])
            .map_err(|err| format!("Failed to write ANN payload: {err}"))?;
        hasher.update(&buffer[..read]);
    }
    Ok(())
}
