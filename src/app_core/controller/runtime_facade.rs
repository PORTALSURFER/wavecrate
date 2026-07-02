//! App-core-owned runtime facade over the retained legacy controller backend.
//!
//! The mature controller implementation still lives in `src/app`. This module
//! is the only app-core production adapter that names that backend while
//! migration slices continue moving behavior behind narrower app-core methods.

/// Runtime-facing controller type owned by the app-core facade module.
pub type AppController = crate::app::controller::AppController;
