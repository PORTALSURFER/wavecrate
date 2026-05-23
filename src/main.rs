#![deny(missing_docs)]
#![deny(warnings)]
#![allow(clippy::too_many_arguments)]

//! Wavecrate application entry point.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app_registration;
mod gui_app;

fn main() -> Result<(), String> {
    app_registration::ensure_registration()?;
    gui_app::run()
}
