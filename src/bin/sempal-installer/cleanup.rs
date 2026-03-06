#[cfg(target_os = "windows")]
use std::{env, fs, io::ErrorKind, path::Path, path::PathBuf};

#[cfg(target_os = "windows")]
use crate::{UNINSTALL_KEY, paths};

#[cfg(target_os = "windows")]
use windows::{
    Win32::Storage::FileSystem::{MOVEFILE_DELAY_UNTIL_REBOOT, MoveFileExW},
    core::PCWSTR,
};
#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "windows")]
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

/// Run uninstall cleanup for the current app installation.
pub(crate) fn run_uninstall() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let install_dir = read_install_dir_from_registry()?;
        remove_install_payload(&install_dir)?;
        remove_start_menu_shortcut_dir()?;
        remove_uninstall_registry_key()?;
        finalize_install_dir(&install_dir)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Uninstall is only supported on Windows.".to_string())
    }
}

#[cfg(target_os = "windows")]
fn read_install_dir_from_registry() -> Result<PathBuf, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let uninstall_key = hkcu
        .open_subkey_with_flags(UNINSTALL_KEY, KEY_READ)
        .map_err(|err| format!("Failed to open uninstall registry key: {err}"))?;
    let install_location: String = uninstall_key
        .get_value("InstallLocation")
        .map_err(|err| format!("Failed to read InstallLocation: {err}"))?;
    Ok(PathBuf::from(install_location))
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
fn remove_uninstall_registry_key() -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.delete_subkey_all(UNINSTALL_KEY)
        .map_err(|err| format!("Failed to remove uninstall registry key: {err}"))
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
