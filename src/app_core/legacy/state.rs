//! Legacy state module adapter.
//!
//! Migration-facing projection and bridge code should depend on this adapter
//! rather than importing `crate::app::state` directly.

pub(crate) use crate::app::state::*;
