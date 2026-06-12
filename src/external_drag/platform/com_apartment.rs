use windows::Win32::System::Ole::{OleInitialize, OleUninitialize};

/// RAII guard to balance COM initialization.
pub(super) struct ComApartment;

impl ComApartment {
    pub(super) fn new() -> Result<Self, String> {
        unsafe { OleInitialize(None) }.map_err(|err| format!("COM init failed: {err}"))?;
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { OleUninitialize() };
    }
}
