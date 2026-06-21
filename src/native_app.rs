//! Default Wavecrate native application built on Radiant's current public API.

mod app;
mod app_chrome;
mod audio;
mod metadata;
mod sample_library;
mod shell;
#[cfg(test)]
mod test_support;
mod transaction_history;
mod ui;
mod waveform;
mod waveform_edit_effects;
mod waveform_edits;
mod workflows;

pub(crate) use shell::run;

#[cfg(test)]
mod tests;
