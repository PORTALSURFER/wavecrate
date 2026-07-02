//! App-core fixture builders for deterministic GUI/runtime scenarios.
//!
//! Fixture seeding still uses the retained legacy controller implementation, but
//! callers receive the app-core runtime facade so fixture construction does not
//! leak the broad legacy controller type through `app_api`.

use tempfile::TempDir;

use crate::{app_core::controller::AppController, waveform::WaveformRenderer};

/// One app-core controller fixture and the temporary resources it keeps alive.
pub(crate) struct GuiFixtureControllerBundle {
    /// Seeded controller facade ready for GUI scenario execution.
    pub(crate) controller: AppController,
    /// Temporary directories that back seeded files and databases.
    pub(crate) sandbox_guards: Vec<TempDir>,
}

/// Build a deterministic app-core controller fixture for the requested GUI tag.
pub(crate) fn build_named_gui_fixture_controller(
    renderer: WaveformRenderer,
    fixture_tag: &str,
) -> Result<GuiFixtureControllerBundle, String> {
    let bundle = crate::app::controller::build_named_gui_fixture_controller(renderer, fixture_tag)?;
    Ok(GuiFixtureControllerBundle {
        controller: bundle.controller,
        sandbox_guards: bundle.sandbox_guards,
    })
}
