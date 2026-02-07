//! Backend-neutral view-model aliases for migration consumers.
//!
//! This re-export keeps runtime-facing projection code independent from direct
//! `egui_app::view_model` module paths while controller internals continue to
//! migrate.

pub use crate::egui_app::view_model::*;
