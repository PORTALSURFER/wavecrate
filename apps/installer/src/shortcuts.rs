use std::path::Path;

#[cfg(any(test, target_os = "windows"))]
use std::fs;

#[cfg(target_os = "windows")]
use crate::paths;

#[cfg(target_os = "windows")]
use windows::{
    Win32::{
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize, IPersistFile,
        },
        UI::Shell::{IShellLinkW, ShellLink},
    },
    core::{HSTRING, Interface, PCWSTR},
};

#[cfg(any(test, target_os = "windows"))]
const START_MENU_SHORTCUT_NAME: &str = "Wavecrate.lnk";
#[cfg(any(test, target_os = "windows"))]
const LEGACY_START_MENU_SHORTCUT_NAMES: &[&str] = &["SemPal.lnk"];

pub(crate) fn create_start_menu_shortcut(install_dir: &Path) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    let _ = install_dir;
    #[cfg(target_os = "windows")]
    {
        let start_menu = paths::start_menu_dir().ok_or_else(|| "APPDATA not set".to_string())?;
        fs::create_dir_all(&start_menu)
            .map_err(|err| format!("Failed to create Start Menu folder: {err}"))?;

        let shortcut_path = start_menu_shortcut_path(&start_menu);
        let target_path = install_dir.join("wavecrate.exe");
        let icon_path = install_dir.join("wavecrate.ico");

        struct ComGuard;
        impl Drop for ComGuard {
            fn drop(&mut self) {
                unsafe { CoUninitialize() };
            }
        }

        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .ok()
                .map_err(|err| format!("Failed to init COM: {err}"))?;
            let _guard = ComGuard;
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|err| format!("Failed to create IShellLink: {err}"))?;

            let target = HSTRING::from(target_path.display().to_string());
            link.SetPath(PCWSTR::from_raw(target.as_ptr()))
                .map_err(|err| format!("Failed to set shortcut target: {err}"))?;

            let icon = HSTRING::from(icon_path.display().to_string());
            let _ = link.SetIconLocation(PCWSTR::from_raw(icon.as_ptr()), 0);

            let persist: IPersistFile = link
                .cast()
                .map_err(|err| format!("Failed to cast IPersistFile: {err}"))?;
            let shortcut = HSTRING::from(shortcut_path.display().to_string());
            persist
                .Save(PCWSTR::from_raw(shortcut.as_ptr()), true)
                .map_err(|err| format!("Failed to save shortcut: {err}"))?;
        }
        remove_legacy_start_menu_shortcuts(&start_menu)?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err("Start Menu shortcut is only supported on Windows.".to_string())
}

#[cfg(any(test, target_os = "windows"))]
fn start_menu_shortcut_path(start_menu: &Path) -> std::path::PathBuf {
    start_menu.join(START_MENU_SHORTCUT_NAME)
}

#[cfg(any(test, target_os = "windows"))]
fn legacy_start_menu_shortcut_paths(
    start_menu: &Path,
) -> impl Iterator<Item = std::path::PathBuf> + '_ {
    LEGACY_START_MENU_SHORTCUT_NAMES
        .iter()
        .map(|name| start_menu.join(name))
}

#[cfg(any(test, target_os = "windows"))]
fn remove_legacy_start_menu_shortcuts(start_menu: &Path) -> Result<usize, String> {
    let mut removed = 0;
    for path in legacy_start_menu_shortcut_paths(start_menu) {
        if !path.exists() {
            continue;
        }
        fs::remove_file(&path)
            .map_err(|err| format!("Failed to remove legacy shortcut {}: {err}", path.display()))?;
        removed += 1;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_menu_shortcut_path_uses_wavecrate_branding() {
        let start_menu = Path::new(
            r"C:\Users\portal\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\SemPal",
        );

        assert_eq!(
            start_menu_shortcut_path(start_menu),
            start_menu.join("Wavecrate.lnk")
        );
    }

    #[test]
    fn legacy_start_menu_shortcut_paths_target_only_installer_owned_sempal_link() {
        let start_menu = Path::new(
            r"C:\Users\portal\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\SemPal",
        );
        let legacy = legacy_start_menu_shortcut_paths(start_menu).collect::<Vec<_>>();

        assert_eq!(legacy, vec![start_menu.join("SemPal.lnk")]);
        assert!(!legacy.contains(&start_menu.join("Wavecrate.lnk")));
    }

    #[test]
    fn remove_legacy_start_menu_shortcuts_removes_only_sempal_link() {
        let temp = tempfile::tempdir().expect("tempdir");
        let start_menu = temp.path();
        let legacy = start_menu.join("SemPal.lnk");
        let current = start_menu.join("Wavecrate.lnk");
        let custom = start_menu.join("Custom.lnk");
        fs::write(&legacy, b"legacy").expect("legacy shortcut");
        fs::write(&current, b"current").expect("current shortcut");
        fs::write(&custom, b"custom").expect("custom shortcut");

        let removed =
            remove_legacy_start_menu_shortcuts(start_menu).expect("legacy cleanup succeeds");

        assert_eq!(removed, 1);
        assert!(!legacy.exists());
        assert!(current.exists());
        assert!(custom.exists());
    }

    #[test]
    fn remove_legacy_start_menu_shortcuts_ignores_absent_legacy_link() {
        let temp = tempfile::tempdir().expect("tempdir");

        let removed =
            remove_legacy_start_menu_shortcuts(temp.path()).expect("legacy cleanup succeeds");

        assert_eq!(removed, 0);
    }
}
