#![deny(missing_docs)]
#![deny(warnings)]

//! Wavecrate application entry point.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() -> Result<(), String> {
    wavecrate::native_app::run()
}

#[cfg(test)]
mod native_app {
    #[test]
    fn production_entrypoint_is_library_owned() {
        let entrypoint: fn() -> Result<(), String> = wavecrate::native_app::run;
        let _ = entrypoint;
    }
}
