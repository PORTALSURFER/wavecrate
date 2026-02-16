//! Direct API bridge to legacy `app` modules used by migration-facing runtime layers.
//!
//! This module intentionally holds all legacy crossings in a single location while
//! `app` internals continue to be progressively extracted.
pub(crate) mod controller {
    /// Legacy application controller implementation.
    pub(crate) use crate::app::controller::AppController;
}

pub(crate) mod state {
    /// Legacy application state types.
    pub(crate) use crate::app::state::*;
}

pub(crate) mod view_model {
    /// Legacy sample view-model helpers.
    pub(crate) use crate::app::view_model::*;
}
