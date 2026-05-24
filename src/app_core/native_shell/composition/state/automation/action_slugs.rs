//! Stable automation action identifiers grouped by action domain.

use super::*;

#[path = "action_slugs/app.rs"]
mod app;
#[path = "action_slugs/browser.rs"]
mod browser;
#[path = "action_slugs/shell_sources.rs"]
mod shell_sources;
#[path = "action_slugs/waveform.rs"]
mod waveform;

/// Convert one concrete action into its stable automation action id.
pub(super) fn action_slug(action: &UiAction) -> String {
    action_slug_str(action).to_string()
}

fn action_slug_str(action: &UiAction) -> &'static str {
    shell_sources::action_slug(action)
        .or_else(|| browser::action_slug(action))
        .or_else(|| waveform::action_slug(action))
        .or_else(|| app::action_slug(action))
        .expect("all runtime-contract actions must expose stable automation slugs")
}
