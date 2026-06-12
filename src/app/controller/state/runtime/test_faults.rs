//! Test-only controller runtime fault injection state.

/// Test-only controller runtime fault injection switches.
#[derive(Clone, Debug, Default)]
pub(crate) struct TestFaultRuntimeState {
    pub(crate) progress_cancel_after: Option<usize>,
    /// Force the next folder delete DB write to fail for rollback tests.
    pub(crate) fail_next_folder_delete_db: bool,
    /// Simulate a crash after staging a folder delete.
    pub(crate) fail_after_folder_delete_stage: bool,
    /// Simulate a crash after committing the folder delete DB update.
    pub(crate) fail_after_folder_delete_db_commit: bool,
    /// Force the next waveform-to-browser copy registration to fail after the file copy.
    pub(crate) fail_next_waveform_copy_registration: bool,
}
