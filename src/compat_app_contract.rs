//! Compatibility re-export for older native-shell tests and adapters.
//!
//! Production native-shell runtime and composition code should import the
//! Sempal-owned contract from `app_core::native_shell::runtime_contract`
//! directly. This module remains only as a narrow crate-local compatibility
//! facade while existing focused tests are migrated.
#![allow(unused_imports)]

pub use crate::app_core::native_shell::runtime_contract::*;
