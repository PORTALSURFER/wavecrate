use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Shell::DROPFILES;

/// Serialize a shell `DROPFILES` payload for one or more paths.
pub(super) fn build_dropfiles_payload(paths: &[PathBuf]) -> Vec<u8> {
    let path_bytes = encode_drag_paths(paths);
    let mut payload = Vec::with_capacity(std::mem::size_of::<DROPFILES>() + path_bytes.len());
    payload.extend_from_slice(&dropfiles_header_bytes());
    payload.extend_from_slice(&path_bytes);
    payload
}

/// Encode a double-null-terminated UTF-16 path list for `CF_HDROP`.
pub(super) fn encode_drag_paths(paths: &[PathBuf]) -> Vec<u8> {
    let mut utf16_paths = Vec::new();
    for path in paths {
        utf16_paths.extend(
            path.as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .flat_map(u16::to_le_bytes),
        );
    }
    utf16_paths.extend_from_slice(&0u16.to_le_bytes());
    utf16_paths
}

fn dropfiles_header_bytes() -> [u8; std::mem::size_of::<DROPFILES>()] {
    let header = DROPFILES {
        pFiles: std::mem::size_of::<DROPFILES>() as u32,
        pt: POINT { x: 0, y: 0 },
        fNC: false.into(),
        fWide: true.into(),
    };
    let mut bytes = [0u8; std::mem::size_of::<DROPFILES>()];
    unsafe {
        std::ptr::copy_nonoverlapping(
            (&header as *const DROPFILES).cast::<u8>(),
            bytes.as_mut_ptr(),
            bytes.len(),
        );
    }
    bytes
}
