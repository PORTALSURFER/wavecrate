use std::path::Path;

#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
fn fail_save_target() -> &'static Mutex<Option<PathBuf>> {
    static TARGET: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    TARGET.get_or_init(|| Mutex::new(None))
}

pub(super) fn replace_journal_file(tmp_path: &Path, path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::{
            core::PCWSTR,
            Win32::Storage::FileSystem::{
                MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            },
        };

        let from = wide_path(tmp_path);
        let to = wide_path(path);
        let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;
        unsafe { MoveFileExW(PCWSTR(from.as_ptr()), PCWSTR(to.as_ptr()), flags) }
            .map_err(|err| format!("Failed to save delete journal: {err}"))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::fs::rename(tmp_path, path)
            .map_err(|err| format!("Failed to save delete journal: {err}"))
    }
}

#[cfg(target_os = "windows")]
fn wide_path(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> =
        <std::ffi::OsStr as std::os::windows::ffi::OsStrExt>::encode_wide(path.as_os_str())
            .collect();
    wide.push(0);
    wide
}

#[cfg(not(test))]
pub(crate) fn fail_save_before_replace(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
pub(crate) fn fail_save_before_replace(path: &Path) -> Result<(), String> {
    let mut guard = fail_save_target()
        .lock()
        .map_err(|_| "Delete journal test hook lock poisoned".to_string())?;
    if guard.as_ref().is_some_and(|target| target == path) {
        *guard = None;
        return Err("Injected delete journal save failure before replace".into());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn fail_next_save_before_replace_for_tests(path: PathBuf) {
    let mut guard = fail_save_target()
        .lock()
        .expect("delete journal test hook lock");
    *guard = Some(path);
}
