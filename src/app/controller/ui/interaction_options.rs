mod effects;
mod policy;

pub(crate) use policy::{clamp_anti_clip_fade_ms, clamp_scroll_speed, clamp_zoom_factor};

#[cfg(test)]
use super::AppController;
#[cfg(test)]
use policy::{wheel_zoom_factor_to_speed, wheel_zoom_speed_to_factor};

#[cfg(test)]
/// Unit tests for interaction-options controls and waveform option sync.
mod tests;
