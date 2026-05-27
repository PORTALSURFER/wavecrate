use std::{cell::RefCell, rc::Rc};

use crate::{audio::AudioPlayer, waveform::WaveformRenderer};
use tracing::{error, info};

use super::AppController;

/// Build a configured migration-facing controller for UI runtime hosts.
///
/// This centralizes controller creation and config loading so UI hosts need not
/// depend directly on legacy initialization details.
pub fn build_ui_app_controller(
    renderer: WaveformRenderer,
    player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<AppController, String> {
    info!("Loading startup configuration for UI app controller");
    let cfg = crate::sample_sources::config::load_or_default().map_err(|err| {
        let message = format!("Failed to load config: {err}");
        error!(err = %err, "Failed to load config for UI app controller");
        message
    })?;
    info!("Startup config loaded");
    let mut controller = AppController::new_with_job_message_queue_capacity(
        renderer,
        player,
        cfg.core.job_message_queue_capacity as usize,
    );
    info!("AppController created, applying startup configuration");
    controller.apply_configuration(cfg).map_err(|err| {
        let message = format!("Failed to load config: {err}");
        error!(err = %err, "Failed to apply startup configuration");
        message
    })?;
    info!("Startup configuration applied");
    controller.select_first_source();
    info!("Selected initial source during startup");
    Ok(controller)
}
