#![deny(missing_docs)]
#![deny(warnings)]

//! Wavecrate application entry point.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod native_app;

fn main() -> Result<(), String> {
    native_app::run()
}
