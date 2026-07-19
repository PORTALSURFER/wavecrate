#[derive(Clone, Debug)]
pub(crate) struct SampleMetadata {
    pub(crate) sample_id: String,
    pub(crate) content_hash: String,
    pub(crate) size: u64,
    pub(crate) mtime_ns: i64,
}
