//! Legacy controller module adapter.
//!
//! Keeping this re-export in one place ensures migration-facing code does not
//! import legacy controller modules directly.

pub(crate) use crate::app::controller::*;
