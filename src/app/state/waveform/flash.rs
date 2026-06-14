use std::time::Instant;

pub(super) struct WaveformFlashState {
    pub(super) copy_at: Option<Instant>,
    pub(super) selection_export_nonce: u64,
    pub(super) selection_export_failure_nonce: u64,
    pub(super) edit_selection_apply_nonce: u64,
}

impl Default for WaveformFlashState {
    fn default() -> Self {
        Self {
            copy_at: None,
            selection_export_nonce: 0,
            selection_export_failure_nonce: 0,
            edit_selection_apply_nonce: 0,
        }
    }
}
