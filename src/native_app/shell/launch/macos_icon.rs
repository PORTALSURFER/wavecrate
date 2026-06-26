#[cfg(target_os = "macos")]
pub(super) fn install_macos_application_icon() {
    crate::native_app::shell::macos_app_icon::install_wavecrate_application_icon();
}

#[cfg(not(target_os = "macos"))]
pub(super) fn install_macos_application_icon() {}
