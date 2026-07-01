#[cfg(target_os = "windows")]
use std::{env, fs, io::ErrorKind, path::Path, path::PathBuf};

#[cfg(target_os = "windows")]
use crate::paths;
#[cfg(any(test, target_os = "windows"))]
use crate::{LEGACY_UNINSTALL_KEYS, UNINSTALL_KEY};

#[cfg(target_os = "windows")]
use windows::{
    Win32::Storage::FileSystem::{MOVEFILE_DELAY_UNTIL_REBOOT, MoveFileExW},
    core::PCWSTR,
};
#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "windows")]
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

#[cfg(target_os = "windows")]
#[derive(Debug)]
struct RegisteredInstall {
    key_path: &'static str,
    install_dir: PathBuf,
}

/// Run uninstall cleanup for the current app installation.
pub(crate) fn run_uninstall() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let registration = read_install_dir_from_registry()?;
        let install_dir = registration.install_dir;
        remove_install_payload(&install_dir)?;
        remove_start_menu_shortcut_dir()?;
        remove_uninstall_registry_keys(registration.key_path, &install_dir)?;
        finalize_install_dir(&install_dir)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Uninstall is only supported on Windows.".to_string())
    }
}

#[cfg(target_os = "windows")]
fn read_install_dir_from_registry() -> Result<RegisteredInstall, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let mut errors = Vec::new();
    for key_path in uninstall_key_candidates() {
        match hkcu.open_subkey_with_flags(key_path, KEY_READ) {
            Ok(uninstall_key) => {
                let install_location: String =
                    uninstall_key.get_value("InstallLocation").map_err(|err| {
                        format!("Failed to read InstallLocation from {key_path}: {err}")
                    })?;
                return Ok(RegisteredInstall {
                    key_path,
                    install_dir: PathBuf::from(install_location),
                });
            }
            Err(err) => errors.push(format!("{key_path}: {err}")),
        }
    }
    Err(format!(
        "Failed to open Wavecrate or legacy SemPal uninstall registry key: {}",
        errors.join("; ")
    ))
}

#[cfg(target_os = "windows")]
fn remove_install_payload(install_dir: &Path) -> Result<(), String> {
    if !install_dir.exists() {
        return Ok(());
    }
    let current_exe = env::current_exe().ok();
    let entries = fs::read_dir(install_dir).map_err(|err| {
        format!(
            "Failed to read install dir {}: {err}",
            install_dir.display()
        )
    })?;
    for entry in entries {
        let path = entry
            .map_err(|err| format!("Failed to read install entry: {err}"))?
            .path();
        if current_exe.as_ref().is_some_and(|exe| exe == &path) {
            continue;
        }
        remove_path_tree(&path)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_path_tree(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|err| format!("Failed to remove directory {}: {err}", path.display()))
    } else {
        fs::remove_file(path)
            .map_err(|err| format!("Failed to remove file {}: {err}", path.display()))
    }
}

#[cfg(target_os = "windows")]
fn remove_start_menu_shortcut_dir() -> Result<(), String> {
    let Some(path) = paths::start_menu_dir() else {
        return Ok(());
    };
    if !path.exists() {
        return Ok(());
    }
    fs::remove_dir_all(&path).map_err(|err| {
        format!(
            "Failed to remove Start Menu folder {}: {err}",
            path.display()
        )
    })
}

#[cfg(target_os = "windows")]
fn remove_uninstall_registry_keys(
    selected_key_path: &'static str,
    install_dir: &Path,
) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for key_path in uninstall_key_candidates() {
        if key_path != selected_key_path
            && !registry_key_points_to_install_dir(&hkcu, key_path, install_dir)
        {
            continue;
        }
        match hkcu.delete_subkey_all(key_path) {
            Ok(()) => {}
            Err(err) if key_path != selected_key_path => {}
            Err(err) => {
                return Err(format!(
                    "Failed to remove uninstall registry key {key_path}: {err}"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn registry_key_points_to_install_dir(hkcu: &RegKey, key_path: &str, install_dir: &Path) -> bool {
    let Ok(key) = hkcu.open_subkey_with_flags(key_path, KEY_READ) else {
        return false;
    };
    key.get_value::<String, _>("InstallLocation")
        .ok()
        .is_some_and(|location| {
            crate::registry::install_location_matches_install_dir(&location, install_dir)
        })
}

#[cfg(any(test, target_os = "windows"))]
fn uninstall_key_candidates() -> impl Iterator<Item = &'static str> {
    std::iter::once(UNINSTALL_KEY).chain(LEGACY_UNINSTALL_KEYS.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uninstall_key_candidates_prefer_wavecrate_before_legacy_sempal() {
        let keys = uninstall_key_candidates().collect::<Vec<_>>();

        assert_eq!(
            keys,
            vec![
                "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Wavecrate",
                "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SemPal",
            ]
        );
    }
}

#[cfg(target_os = "windows")]
fn finalize_install_dir(install_dir: &Path) -> Result<(), String> {
    if !install_dir.exists() {
        return Ok(());
    }
    match fs::remove_dir(install_dir) {
        Ok(()) => Ok(()),
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::PermissionDenied | ErrorKind::DirectoryNotEmpty
            ) =>
        {
            schedule_pending_delete(install_dir)
        }
        Err(err) => Err(format!(
            "Failed to remove install directory {}: {err}",
            install_dir.display()
        )),
    }
}

#[cfg(target_os = "windows")]
fn schedule_pending_delete(install_dir: &Path) -> Result<(), String> {
    if let Ok(current_exe) = env::current_exe()
        && current_exe.starts_with(install_dir)
    {
        schedule_delete_on_reboot(&current_exe)?;
    }
    schedule_delete_on_reboot(install_dir)
}

#[cfg(target_os = "windows")]
fn schedule_delete_on_reboot(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    use std::os::windows::ffi::OsStrExt;
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    unsafe {
        MoveFileExW(
            PCWSTR::from_raw(wide.as_ptr()),
            PCWSTR::null(),
            MOVEFILE_DELAY_UNTIL_REBOOT,
        )
    }
    .map_err(|err| {
        format!(
            "Failed to schedule deletion on reboot for {}: {err}",
            path.display()
        )
    })
}
