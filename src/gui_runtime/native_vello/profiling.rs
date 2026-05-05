use super::*;

#[cfg(feature = "gui-performance")]
const REDRAW_PROFILE_INTERVAL_FRAMES: u64 = 240;
#[cfg(feature = "gui-performance")]
const REDRAW_PROFILE_ENV: &str = "RADIANT_NATIVE_RENDER_PROFILE";

#[cfg(not(feature = "gui-performance"))]
mod noop;
#[cfg(feature = "gui-performance")]
mod report;
#[cfg(feature = "gui-performance")]
mod stats;

#[cfg(not(feature = "gui-performance"))]
pub(super) use self::noop::NativeVelloProfiler;
#[cfg(feature = "gui-performance")]
pub(super) use self::stats::NativeVelloProfiler;

/// Interaction classes tracked by runtime performance profiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InteractionProfileKind {
    Hover,
    Wheel,
    SpatialPanProxy,
    Timeline,
    Volume,
}
