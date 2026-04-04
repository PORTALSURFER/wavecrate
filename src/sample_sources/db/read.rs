/// Batch BPM lookup helpers for source databases.
mod bpm_queries;
/// Row and relative-path decoding helpers for source-database reads.
mod decode;
/// Wav-file and path-list query helpers for source databases.
mod file_queries;
/// Path-specific metadata and lookup helpers for source databases.
mod metadata_queries;

pub(crate) use self::file_queries::{SearchEntryMetadata, SearchEntryRow};

#[cfg(test)]
mod tests;
