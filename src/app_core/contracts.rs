//! Migration boundary contracts for legacy `crate::app` dependencies.
//!
//! This module is the single point in `app_core` where direct references to
//! legacy `crate::app` modules remain during runtime migration. Keep this file
//! minimal and remove/replace it once migration-facing callers no longer need
//! those legacy paths.

/// Legacy controller types used by migration-facing runtime orchestration.
pub(crate) mod controller {
    pub(crate) use crate::app::controller::AppController;
}

/// Legacy state module used by migration-facing projection and conversion code.
pub(crate) mod state {
    pub(crate) use crate::app::state::*;
}

/// Legacy view-model helpers used by migration-facing formatting glue.
pub(crate) mod view_model {
    pub(crate) use crate::app::view_model::*;
}
