#![deny(missing_docs)]
#![deny(warnings)]
#![allow(clippy::too_many_arguments)]

//! Feature-selected Sempal application entry point.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#[cfg(all(feature = "legacy-sample", feature = "radiant-rebuild"))]
compile_error!("enable either legacy-sample or radiant-rebuild, not both");

#[cfg(not(any(feature = "legacy-sample", feature = "radiant-rebuild")))]
compile_error!("enable either legacy-sample or radiant-rebuild");

#[cfg(feature = "legacy-sample")]
mod app_icon;
#[cfg(feature = "legacy-sample")]
mod legacy_sample_app;
#[cfg(feature = "radiant-rebuild")]
mod radiant_rebuild_app;
#[cfg(feature = "legacy-sample")]
mod run_contract;

#[cfg(feature = "radiant-rebuild")]
fn main() -> Result<(), String> {
    radiant_rebuild_app::run()
}

#[cfg(feature = "legacy-sample")]
fn main() -> Result<(), String> {
    legacy_sample_app::run()
}
