//! Default Wavecrate native application built on Radiant's current public API.

mod app_scope;
mod audio;
mod browser;
mod chrome;
mod context_menu;
mod metadata_tag_metrics;
mod metadata_tags;
mod shell;
mod state;
#[cfg(test)]
mod test_support;
mod transaction_history;
mod waveform;
mod widget_ids;
mod workflows;

pub(crate) use shell::run;

#[cfg(test)]
mod tests;
