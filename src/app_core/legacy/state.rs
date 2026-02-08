//! Legacy state module adapter.
//!
//! Migration-facing projection and bridge code should depend on this adapter
//! rather than importing legacy state modules directly.

pub(crate) use crate::legacy_runtime::state::*;
