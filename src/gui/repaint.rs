//! Backend-neutral repaint signaling primitives.

use std::sync::{Arc, RwLock};

/// Runtime-provided callback used by background systems to request a UI repaint.
pub trait RepaintSignal: Send + Sync {
    /// Request that the active UI backend schedules a repaint soon.
    fn request_repaint(&self);
}

/// Shared holder for the current repaint callback.
///
/// The active runtime updates this when UI contexts change, while background
/// workers only call [`Self::request_repaint`].
#[derive(Default)]
pub struct SharedRepaintSignal {
    signal: RwLock<Option<Arc<dyn RepaintSignal>>>,
}

impl SharedRepaintSignal {
    /// Replace the active repaint callback.
    pub fn set_signal(&self, signal: Option<Arc<dyn RepaintSignal>>) {
        if let Ok(mut lock) = self.signal.write() {
            *lock = signal;
        }
    }

    /// Request a repaint through the active callback, if one is available.
    pub fn request_repaint(&self) {
        if let Ok(lock) = self.signal.read() {
            if let Some(signal) = lock.as_ref() {
                signal.request_repaint();
            }
        }
    }
}
