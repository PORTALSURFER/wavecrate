use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::ptr::copy_nonoverlapping;

use windows::Win32::Foundation::POINT;
use windows::Win32::System::DataExchange::{GetClipboardData, IsClipboardFormatAvailable};
use windows::Win32::System::Ole::CF_HDROP;
use windows::Win32::UI::Shell::{DROPFILES, DragQueryFileW, HDROP};

use super::handle::{ClipboardReader, GlobalLockGuard, OwnedHGlobal};

pub(super) fn read_file_paths() -> Result<Vec<PathBuf>, String> {
    if unsafe { IsClipboardFormatAvailable(CF_HDROP.0 as u32) }.is_err() {
        return Ok(Vec::new());
    }
    let _clipboard = ClipboardReader::new()?;
    let handle = unsafe { GetClipboardData(CF_HDROP.0 as u32) }
        .map_err(|err| format!("GetClipboardData(CF_HDROP) failed: {err}"))?;
    let hdrop = HDROP(handle.0);
    let count = unsafe { DragQueryFileW(hdrop, 0xFFFFFFFF, None) };
    if count == 0 {
        return Ok(Vec::new());
    }
    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        let len = unsafe { DragQueryFileW(hdrop, index, None) } as usize;
        if len == 0 {
            continue;
        }
        let mut buffer = vec![0u16; len + 1];
        let written = unsafe { DragQueryFileW(hdrop, index, Some(&mut buffer)) } as usize;
        if written == 0 {
            continue;
        }
        buffer.truncate(written);
        paths.push(PathBuf::from(OsString::from_wide(&buffer)));
    }
    Ok(paths)
}

pub(super) fn create_drop_effect(effect: u32) -> Result<OwnedHGlobal, String> {
    let owned = OwnedHGlobal::new(drop_effect_payload_bytes())?;
    let lock = unsafe { GlobalLockGuard::new(owned.handle()) }?;
    unsafe {
        *lock.as_mut_ptr::<u32>() = effect;
    }
    Ok(owned)
}

pub(super) fn create_hdrop(paths: &[PathBuf]) -> Result<OwnedHGlobal, String> {
    let utf16_paths = encode_hdrop_path_list(paths);
    let owned = OwnedHGlobal::new(hdrop_payload_bytes_for_units(utf16_paths.len()))?;
    let lock = unsafe { GlobalLockGuard::new(owned.handle()) }?;
    unsafe {
        let header = lock.as_mut_ptr::<DROPFILES>();
        *header = DROPFILES {
            pFiles: std::mem::size_of::<DROPFILES>() as u32,
            pt: POINT { x: 0, y: 0 },
            fNC: false.into(),
            fWide: true.into(),
        };
        let data_ptr = (lock.ptr() as *mut u8).add(std::mem::size_of::<DROPFILES>());
        copy_nonoverlapping(
            utf16_paths.as_ptr() as *const u8,
            data_ptr,
            utf16_paths.len() * std::mem::size_of::<u16>(),
        );
    }
    Ok(owned)
}

fn encode_hdrop_path_list(paths: &[PathBuf]) -> Vec<u16> {
    let mut utf16_paths = Vec::new();
    for path in paths {
        utf16_paths.extend(path.as_os_str().encode_wide().chain(std::iter::once(0)));
    }
    utf16_paths.push(0);
    utf16_paths
}

fn hdrop_payload_bytes_for_units(path_units: usize) -> usize {
    std::mem::size_of::<DROPFILES>() + path_units * std::mem::size_of::<u16>()
}

fn drop_effect_payload_bytes() -> usize {
    std::mem::size_of::<u32>()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{drop_effect_payload_bytes, encode_hdrop_path_list, hdrop_payload_bytes_for_units};

    #[test]
    fn hdrop_path_list_is_double_null_terminated() {
        let paths = [
            PathBuf::from(r"C:\samples\kick.wav"),
            PathBuf::from(r"D:\snare.wav"),
        ];
        let encoded = encode_hdrop_path_list(&paths);
        assert_eq!(encoded.last().copied(), Some(0));
        assert_eq!(encoded.iter().filter(|&&unit| unit == 0).count(), 3);
    }

    #[test]
    fn hdrop_payload_size_includes_header_and_utf16_units() {
        let encoded = encode_hdrop_path_list(&[PathBuf::from(r"C:\samples\hat.wav")]);
        assert_eq!(
            hdrop_payload_bytes_for_units(encoded.len()),
            std::mem::size_of::<windows::Win32::UI::Shell::DROPFILES>()
                + encoded.len() * std::mem::size_of::<u16>()
        );
    }

    #[test]
    fn drop_effect_payload_size_matches_u32() {
        assert_eq!(drop_effect_payload_bytes(), std::mem::size_of::<u32>());
    }
}
