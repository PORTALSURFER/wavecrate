//! Backend-neutral application-core helpers shared by GUI runtimes.
//!
//! This module centralizes migration-facing types so runtime hosts can rely on
//! `app_core` without taking direct `app` module dependencies.
pub mod app_api;

/// Controller aliases and helpers used by migration-facing runtimes.
pub mod controller;

/// Native runtime action/model aliases for migration-facing glue code.
pub mod actions;

/// Native runtime bridge implementations for migration-facing runtimes.
pub mod native_bridge;
pub(crate) mod native_shell;

/// Migration-facing state projections used by app bridge adapters.
pub mod state;

/// Migration-facing view-model helpers used by projections.
pub mod view_model;

/// Migration-facing UI constants used by runtime hosts.
pub mod ui;
