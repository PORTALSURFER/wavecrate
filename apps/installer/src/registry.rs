use std::path::Path;

#[cfg(target_os = "windows")]
use std::env;

#[cfg(target_os = "windows")]
use crate::LEGACY_UNINSTALL_KEYS;
#[cfg(any(test, target_os = "windows"))]
use crate::{APP_NAME, APP_PUBLISHER, UNINSTALL_KEY};

#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "windows")]
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

#[cfg(any(test, target_os = "windows"))]
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct UninstallEntryMetadata {
    pub(crate) key_path: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) publisher: &'static str,
}

#[cfg(any(test, target_os = "windows"))]
pub(crate) fn uninstall_entry_metadata() -> UninstallEntryMetadata {
    UninstallEntryMetadata {
        key_path: UNINSTALL_KEY,
        display_name: APP_NAME,
        publisher: APP_PUBLISHER,
    }
}

pub(crate) fn register_uninstall_entry(install_dir: &Path) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    let _ = install_dir;
    #[cfg(target_os = "windows")]
    {
        let metadata = uninstall_entry_metadata();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey(metadata.key_path)
            .map_err(|err| format!("Failed to create uninstall registry key: {err}"))?;
        let exe_path = install_dir.join("wavecrate-installer.exe");
        let uninstall = format!("\"{}\" --uninstall", exe_path.display());
        key.set_value("DisplayName", &metadata.display_name)
            .map_err(|err| format!("Failed to set DisplayName: {err}"))?;
        key.set_value("DisplayVersion", &env!("CARGO_PKG_VERSION"))
            .map_err(|err| format!("Failed to set DisplayVersion: {err}"))?;
        key.set_value("Publisher", &metadata.publisher)
            .map_err(|err| format!("Failed to set Publisher: {err}"))?;
        key.set_value("InstallLocation", &install_dir.display().to_string())
            .map_err(|err| format!("Failed to set InstallLocation: {err}"))?;
        key.set_value("UninstallString", &uninstall)
            .map_err(|err| format!("Failed to set UninstallString: {err}"))?;
        key.set_value(
            "DisplayIcon",
            &install_dir.join("wavecrate.ico").display().to_string(),
        )
        .map_err(|err| format!("Failed to set DisplayIcon: {err}"))?;
        remove_legacy_uninstall_entries_for_install_dir(&hkcu, install_dir)?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err("Uninstall registry entry is only supported on Windows.".to_string())
}

#[cfg(target_os = "windows")]
fn remove_legacy_uninstall_entries_for_install_dir(
    hkcu: &RegKey,
    install_dir: &Path,
) -> Result<(), String> {
    for key_path in LEGACY_UNINSTALL_KEYS {
        let Ok(key) = hkcu.open_subkey_with_flags(key_path, KEY_READ) else {
            continue;
        };
        let install_location = key.get_value::<String, _>("InstallLocation").ok();
        drop(key);
        if install_location
            .as_deref()
            .is_some_and(|location| install_location_matches_install_dir(location, install_dir))
        {
            hkcu.delete_subkey_all(key_path)
                .map_err(|err| format!("Failed to remove legacy uninstall registry key: {err}"))?;
        }
    }
    Ok(())
}

#[cfg(any(test, target_os = "windows"))]
pub(crate) fn install_location_matches_install_dir(
    install_location: &str,
    install_dir: &Path,
) -> bool {
    let lhs = normalize_install_location_text(install_location);
    let rhs = normalize_install_location_text(&install_dir.display().to_string());
    lhs.eq_ignore_ascii_case(&rhs)
}

#[cfg(any(test, target_os = "windows"))]
fn normalize_install_location_text(path: &str) -> String {
    path.trim().trim_end_matches(['\\', '/']).replace('/', "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uninstall_entry_metadata_registers_wavecrate_not_sempal() {
        assert_eq!(
            uninstall_entry_metadata(),
            UninstallEntryMetadata {
                key_path: "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Wavecrate",
                display_name: "Wavecrate",
                publisher: "Wavecrate",
            }
        );
    }

    #[test]
    fn install_location_match_handles_case_and_separator_drift() {
        assert!(install_location_matches_install_dir(
            r"c:/users/portal/appdata/local/programs/wavecrate/",
            Path::new(r"C:\Users\portal\AppData\Local\Programs\Wavecrate")
        ));
    }

    #[test]
    fn install_location_match_rejects_separate_legacy_install_dir() {
        assert!(!install_location_matches_install_dir(
            r"C:\Users\portal\AppData\Local\Programs\SemPal",
            Path::new(r"C:\Users\portal\AppData\Local\Programs\Wavecrate")
        ));
    }
}
