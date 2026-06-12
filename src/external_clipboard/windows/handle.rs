use windows::Win32::Foundation::{GlobalFree, HGLOBAL};
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock,
};

pub(super) struct ClipboardWriter;

impl ClipboardWriter {
    pub(super) fn new() -> Result<Self, String> {
        unsafe { OpenClipboard(None) }.map_err(|err| format!("OpenClipboard failed: {err}"))?;
        unsafe { EmptyClipboard() }.map_err(|err| format!("EmptyClipboard failed: {err}"))?;
        Ok(Self)
    }
}

impl Drop for ClipboardWriter {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

pub(super) struct ClipboardReader;

impl ClipboardReader {
    pub(super) fn new() -> Result<Self, String> {
        unsafe { OpenClipboard(None) }.map_err(|err| format!("OpenClipboard failed: {err}"))?;
        Ok(Self)
    }
}

impl Drop for ClipboardReader {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

pub(super) struct OwnedHGlobal {
    handle: HGLOBAL,
    released: bool,
}

impl OwnedHGlobal {
    pub(super) fn new(bytes: usize) -> Result<Self, String> {
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes) }
            .map_err(|_| "GlobalAlloc failed".to_string())?;
        Ok(Self {
            handle,
            released: false,
        })
    }

    pub(super) fn handle(&self) -> HGLOBAL {
        self.handle
    }

    pub(super) fn release(mut self) -> HGLOBAL {
        self.released = true;
        self.handle
    }
}

impl Drop for OwnedHGlobal {
    fn drop(&mut self) {
        if !self.released {
            unsafe {
                let _ = GlobalFree(Some(self.handle));
            }
        }
    }
}

pub(super) struct GlobalLockGuard {
    handle: HGLOBAL,
    ptr: *mut core::ffi::c_void,
}

impl GlobalLockGuard {
    pub(super) unsafe fn new(handle: HGLOBAL) -> Result<Self, String> {
        let ptr = unsafe { GlobalLock(handle) };
        if ptr.is_null() {
            return Err("GlobalLock failed".into());
        }
        Ok(Self { handle, ptr })
    }

    pub(super) fn ptr(&self) -> *mut core::ffi::c_void {
        self.ptr
    }

    pub(super) fn as_mut_ptr<T>(&self) -> *mut T {
        self.ptr as *mut T
    }
}

impl Drop for GlobalLockGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalUnlock(self.handle);
        }
    }
}
