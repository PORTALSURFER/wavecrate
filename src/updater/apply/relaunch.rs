//! Relaunch command construction for updated executables.

use std::path::Path;
use std::process::Command;

use super::{UpdateError, UpdateManifest};

/// Relaunch helper for an updated executable.
pub(super) fn relaunch_app(
    install_dir: &Path,
    app: &str,
    manifest: &UpdateManifest,
) -> Result<(), UpdateError> {
    let candidate = app_executable_name(app, manifest);
    let exe = install_dir.join(&candidate);
    if !exe.exists() {
        return Err(UpdateError::Invalid(format!(
            "Updated executable missing: {}",
            exe.display()
        )));
    }
    let exe_display = exe.display().to_string();
    let mut cmd = Command::new(&exe);
    cmd.spawn()
        .map_err(|err| UpdateError::Invalid(format!("Failed to relaunch {exe_display}: {err}")))?;
    Ok(())
}

fn app_executable_name(app: &str, manifest: &UpdateManifest) -> String {
    let exe = format!("{app}.exe");
    if manifest.files.iter().any(|f| f == &exe) {
        return exe;
    }
    app.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_executable_name_prefers_windows_manifest_exe_when_present() {
        let manifest = windows_manifest_with_files(vec!["wavecrate.exe"]);

        assert_eq!(app_executable_name("wavecrate", &manifest), "wavecrate.exe");
    }

    #[test]
    fn app_executable_name_uses_macos_binary_name_without_exe_manifest_entry() {
        let manifest = macos_manifest_with_files(vec!["wavecrate"]);

        assert_eq!(app_executable_name("wavecrate", &manifest), "wavecrate");
    }

    fn windows_manifest_with_files(files: Vec<&str>) -> UpdateManifest {
        supported_platform_manifest_with_files("x86_64-pc-windows-msvc", "windows", files)
    }

    fn macos_manifest_with_files(files: Vec<&str>) -> UpdateManifest {
        supported_platform_manifest_with_files("x86_64-apple-darwin", "macos", files)
    }

    fn supported_platform_manifest_with_files(
        target: &str,
        platform: &str,
        files: Vec<&str>,
    ) -> UpdateManifest {
        UpdateManifest {
            app: "wavecrate".to_string(),
            channel: "stable".to_string(),
            target: target.to_string(),
            platform: platform.to_string(),
            arch: "x86_64".to_string(),
            files: files.into_iter().map(String::from).collect(),
        }
    }
}
