use std::time::Instant;

#[derive(Default)]
pub(super) struct WaveformFlashState {
    pub(super) copy_at: Option<Instant>,
    pub(super) selection_export_nonce: u64,
    pub(super) selection_export_failure_nonce: u64,
    pub(super) edit_selection_apply_nonce: u64,
}
