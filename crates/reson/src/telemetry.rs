//! Opt-in playback hot-path instrumentation for Reson.

use std::sync::OnceLock;
use std::time::Duration;

const RESON_PLAYBACK_TELEMETRY_ENV: &str = "RESON_PLAYBACK_TELEMETRY";
const WAVECRATE_HOTPATH_TELEMETRY_ENV: &str = "WAVECRATE_HOTPATH_TELEMETRY";

static PLAYBACK_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();

pub(crate) fn playback_telemetry_enabled() -> bool {
    *PLAYBACK_TELEMETRY_ENABLED.get_or_init(|| {
        env_var_truthy(RESON_PLAYBACK_TELEMETRY_ENV)
            || env_var_truthy(WAVECRATE_HOTPATH_TELEMETRY_ENV)
    })
}

pub(crate) fn elapsed_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn env_var_truthy(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}
