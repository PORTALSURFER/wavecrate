#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
use objc2::{AnyThread, MainThreadMarker};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplication, NSImage};
#[cfg(target_os = "macos")]
use objc2_foundation::NSString;

#[cfg(target_os = "macos")]
const APP_ICON_ICNS: &str = "assets/logo3.icns";

#[cfg(target_os = "macos")]
pub(super) fn install_macos_application_icon() {
    if let Err(error) = install_macos_application_icon_from_path(&wavecrate_app_icon_path()) {
        tracing::warn!(%error, "failed to install Wavecrate macOS application icon");
    }
}

#[cfg(not(target_os = "macos"))]
pub(super) fn install_macos_application_icon() {}

#[cfg(target_os = "macos")]
fn wavecrate_app_icon_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(APP_ICON_ICNS)
}

#[cfg(target_os = "macos")]
fn install_macos_application_icon_from_path(path: &Path) -> Result<(), String> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "macOS application icon must be installed on the main thread".to_owned())?;
    let path_str = path
        .to_str()
        .ok_or_else(|| format!("icon path is not valid UTF-8: {}", path.display()))?;
    let path = NSString::from_str(path_str);
    let icon = NSImage::initWithContentsOfFile(NSImage::alloc(), &path)
        .ok_or_else(|| format!("load macOS icon image from {path_str}"))?;
    let app = NSApplication::sharedApplication(mtm);

    unsafe { app.setApplicationIconImage(Some(&icon)) };
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn bundled_macos_icon_path_exists() {
        assert!(wavecrate_app_icon_path().is_file());
    }
}
