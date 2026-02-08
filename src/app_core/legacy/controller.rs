//! Legacy controller module adapter.
//!
//! Keeping this re-export in one place ensures migration-facing code does not
//! import `crate::app::controller` directly.

pub(crate) use crate::app::controller::*;
