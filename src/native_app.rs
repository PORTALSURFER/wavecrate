//! Default Wavecrate native application built on Radiant's current public API.

mod app;
mod app_chrome;
mod audio;
#[cfg(any(test, feature = "legacy-controller"))]
pub(crate) mod automation;
mod metadata;
mod protected_source_feedback;
mod release_update;
mod sample_identity_diagnostics;
mod sample_library;
mod shell;
mod source_processing;
mod starmap_audition_telemetry;
#[cfg(test)]
mod test_support;
mod transaction_history;
mod ui;
mod waveform;
mod waveform_edit_effects;
mod waveform_edits;
mod workflows;

pub use shell::run;

#[cfg(test)]
mod tests;
