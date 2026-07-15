mod events;
mod state;
mod telemetry;
mod waveform_publish;

pub(in crate::native_app) use telemetry::FrameSurfaceRevisionTracker;

#[cfg(test)]
mod tests;
