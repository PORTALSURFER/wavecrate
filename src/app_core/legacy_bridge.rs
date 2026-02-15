//! Migration boundary bridge for legacy `crate::app` dependencies.
//!
//! This module centralizes legacy imports that still flow through the app-core
//! runtime layer while the GUI migration remains in progress.

/// Legacy controller type access for migration-facing runtime orchestration.
pub(crate) mod controller {
    /// Legacy app controller type.
    pub(crate) use crate::app::controller::AppController;
}

/// Legacy state module access for migration-facing projections and conversions.
pub(crate) mod state {
    /// Legacy app state surface.
    pub(crate) use crate::app::state::*;
}

/// Legacy view-model helper access for migration-facing formatting glue.
pub(crate) mod view_model {
    /// Legacy sample view-model helpers.
    pub(crate) use crate::app::view_model::*;
}
