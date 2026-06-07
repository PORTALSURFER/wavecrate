//! Default Wavecrate native application built on Radiant's current public API.

mod app;
mod audio;
mod browser;
mod chrome;
mod context_menu;
mod metadata;
mod shell;
#[cfg(test)]
mod test_support;
mod transaction_history;
mod ui;
mod waveform;
mod workflows;

pub(crate) use shell::run;

#[cfg(test)]
mod tests;
