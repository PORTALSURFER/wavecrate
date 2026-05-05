//! Backend-neutral UI constant aliases for migration consumers.
//!
//! These constants define default/minimum viewport constraints used by native
//! runtime entrypoints.

/// Default viewport size used by the native runtime host window.
pub const DEFAULT_VIEWPORT_SIZE: [f32; 2] = [960.0, 560.0];
/// Minimum viewport size used by the native runtime host window.
pub const MIN_VIEWPORT_SIZE: [f32; 2] = [640.0, 400.0];

/// Maximum browser rows projected per native frame.
///
/// The native viewport typically displays a small subset of rows. Keeping this
/// cap moderate reduces projection cost during high-frequency focus/wheel input
/// while still rendering enough overscan for smooth navigation.
pub const MAX_RENDERED_BROWSER_ROWS: usize = 256;

/// Maximum map points projected per native frame.
pub const MAX_RENDERED_MAP_POINTS: usize = 2_500;
