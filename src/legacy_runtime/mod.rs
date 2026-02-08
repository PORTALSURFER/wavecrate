//! Legacy runtime namespace used by migration adapters.
//!
//! This module is the crate-internal compatibility boundary for legacy UI
//! runtime components while native runtime migration is completed.

/// Legacy controller compatibility surface.
#[cfg(feature = "legacy-egui-runtime")]
pub(crate) mod controller {
    pub(crate) use crate::app::controller::*;
}

/// Legacy state compatibility surface.
#[cfg(feature = "legacy-egui-runtime")]
pub(crate) mod state {
    pub(crate) use crate::app::state::*;
}

/// Legacy view-model compatibility surface.
#[cfg(feature = "legacy-egui-runtime")]
pub(crate) mod view_model {
    pub(crate) use crate::app::view_model::*;
}

/// Controller compatibility stub when legacy runtime is disabled.
#[cfg(not(feature = "legacy-egui-runtime"))]
pub(crate) mod controller {}

/// State compatibility stub when legacy runtime is disabled.
#[cfg(not(feature = "legacy-egui-runtime"))]
pub(crate) mod state {}

/// View-model compatibility stub when legacy runtime is disabled.
#[cfg(not(feature = "legacy-egui-runtime"))]
pub(crate) mod view_model {}
