//! Default Wavecrate native application built on Radiant's current public API.

mod app;
mod app_chrome;
mod audio;
mod library_browser;
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
