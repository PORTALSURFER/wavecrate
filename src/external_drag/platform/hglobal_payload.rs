use super::super::payload::build_dropfiles_payload;
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::Com::{STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock,
};
use windows::Win32::System::Ole::DROPEFFECT;

pub(super) fn drop_effect_medium(effect: DROPEFFECT) -> windows::core::Result<STGMEDIUM> {
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, std::mem::size_of::<u32>()) }
        .map_err(|_| windows::core::Error::from_thread())?;
    let ptr = unsafe { GlobalLock(handle) } as *mut u32;
    if ptr.is_null() {
        unsafe {
            let _ = GlobalUnlock(handle);
        }
        return Err(windows::core::Error::from_thread());
    }
    unsafe {
        *ptr = effect.0;
        let _ = GlobalUnlock(handle);
    }
    Ok(hglobal_medium(handle))
}

pub(super) fn drop_effect_from_hglobal(handle: HGLOBAL) -> windows::core::Result<DROPEFFECT> {
    let ptr = unsafe { GlobalLock(handle) } as *const u32;
    if ptr.is_null() {
        unsafe {
            let _ = GlobalUnlock(handle);
        }
        return Err(windows::core::Error::from_thread());
    }
    let effect = unsafe { *ptr };
    unsafe {
        let _ = GlobalUnlock(handle);
    }
    Ok(DROPEFFECT(effect))
}

pub(super) fn path_list_medium(paths: &[PathBuf]) -> Result<STGMEDIUM, std::io::Error> {
    create_hglobal_for_paths(paths).map(hglobal_medium)
}

fn create_hglobal_for_paths(paths: &[PathBuf]) -> Result<HGLOBAL, std::io::Error> {
    let payload = build_dropfiles_payload(paths);
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, payload.len()) }
        .map_err(last_error_from_win32)?;
    let ptr = unsafe { GlobalLock(handle) };
    if ptr.is_null() {
        unsafe {
            let _ = GlobalUnlock(handle);
        }
        return Err(std::io::Error::last_os_error());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(payload.as_ptr(), ptr.cast::<u8>(), payload.len());
        let _ = GlobalUnlock(handle);
    }
    Ok(handle)
}

fn hglobal_medium(handle: HGLOBAL) -> STGMEDIUM {
    STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 { hGlobal: handle },
        pUnkForRelease: ManuallyDrop::new(None),
    }
}

fn last_error_from_win32(err: windows::core::Error) -> std::io::Error {
    std::io::Error::from_raw_os_error(err.code().0)
}
