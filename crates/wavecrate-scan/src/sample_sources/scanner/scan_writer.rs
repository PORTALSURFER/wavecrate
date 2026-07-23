/// Database mutation phases exposed by the scanner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanWritePhase {
    /// Open or migrate the source database before scanning.
    Open,
    /// Commit an indexed manifest batch or scan-completion checkpoint.
    Manifest,
    /// Publish deferred hashes and proven rename reconciliation.
    DeferredHash,
}

/// Coordinates scanner database writes with an owning runtime.
///
/// The returned guard must retain exclusive write ownership until it is dropped. Filesystem
/// traversal, file reads, and content hashing deliberately happen before `lock` is called.
pub trait ScanWriter {
    /// Guard retaining write ownership for one bounded mutation.
    type Guard;

    /// Acquire write ownership for the supplied scanner phase.
    fn lock(&self, phase: ScanWritePhase) -> Self::Guard;
}

/// Default coordinator for callers that already own serialization or only need SQLite's
/// per-database transaction boundary.
#[derive(Clone, Copy, Debug, Default)]
pub struct UncoordinatedScanWriter;

impl ScanWriter for UncoordinatedScanWriter {
    type Guard = ();

    fn lock(&self, _phase: ScanWritePhase) -> Self::Guard {}
}
