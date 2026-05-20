use super::super::*;

impl AppController {
    /// Refuse permanent deletion from Wavecrate's normal cleanup workflow.
    pub fn take_out_trash(&mut self) {
        self.set_status(
            "Permanent trash deletion is outside Wavecrate cleanup; no files were deleted",
            StatusTone::Warning,
        );
    }
}
