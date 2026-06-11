#![deny(missing_docs)]
#![deny(warnings)]

//! Wavecrate application entry point.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#[cfg(not(wavecrate_internal_build))]
mod app_registration;
mod native_app;

fn main() -> Result<(), String> {
    #[cfg(not(wavecrate_internal_build))]
    app_registration::ensure_registration()?;
    native_app::run()
}
