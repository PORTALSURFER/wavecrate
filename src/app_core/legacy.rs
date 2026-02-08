//! Legacy `app` module aliases used by migration-facing `app_core` code.
//!
//! Centralizing these aliases keeps the remaining migration boundary explicit:
//! `app_core` can depend on this module while the rest of runtime glue avoids
//! importing `crate::app::*` directly.

pub(crate) use crate::app::controller;
pub(crate) use crate::app::state;
pub(crate) use crate::app::view_model;
