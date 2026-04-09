use super::normalize_path;
use super::payload::build_dropfiles_payload;
use std::cell::Cell;
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::{info, warn};
use windows::Win32::Foundation::{
    DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, DV_E_FORMATETC, E_INVALIDARG,
    HGLOBAL,
};
use windows::Win32::System::Com::{
    DATADIR_GET, DVASPECT_CONTENT, FORMATETC, IAdviseSink, IDataObject, IEnumFORMATETC, STGMEDIUM,
    STGMEDIUM_0, TYMED_HGLOBAL,
};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock,
};
use windows::Win32::System::Ole::{
    CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, DROPEFFECT_NONE,
    DoDragDrop, IDropSource, OleInitialize, OleUninitialize,
};
use windows::Win32::System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS};
use windows::Win32::UI::Shell::SHCreateStdEnumFmtEtc;
use windows::core::{BOOL, HRESULT, Ref, w};
use windows_implement::implement;

/// RAII guard to balance COM initialization.
struct ComApartment;

impl ComApartment {
    fn new() -> Result<Self, String> {
        unsafe { OleInitialize(None) }.map_err(|err| format!("COM init failed: {err}"))?;
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { OleUninitialize() };
    }
}

#[implement(IDataObject)]
#[derive(Clone)]
struct FileDropDataObject {
    paths: Vec<PathBuf>,
    format: FORMATETC,
    preferred_drop_effect: u16,
    performed_drop_effect: u16,
    performed_effect: Cell<DROPEFFECT>,
}

impl FileDropDataObject {
    fn new(paths: Vec<PathBuf>) -> Result<Self, String> {
        if paths.is_empty() {
            return Err("No files to drag".into());
        }
        let (preferred_drop_effect, performed_drop_effect) = drop_effect_formats()?;
        Ok(Self {
            paths,
            format: build_format(),
            preferred_drop_effect,
            performed_drop_effect,
            performed_effect: Cell::new(DROPEFFECT_NONE),
        })
    }

    fn matches_format(&self, fmt: &FORMATETC) -> bool {
        (fmt.cfFormat == CF_HDROP.0
            && fmt.dwAspect == DVASPECT_CONTENT.0
            && (fmt.tymed & TYMED_HGLOBAL.0 as u32) != 0
            && (fmt.lindex == -1 || fmt.lindex == 0))
            || ((fmt.cfFormat == self.preferred_drop_effect
                || fmt.cfFormat == self.performed_drop_effect)
                && (fmt.tymed & TYMED_HGLOBAL.0 as u32) != 0)
    }

    fn fill_medium(&self, fmt: &FORMATETC) -> windows::core::Result<STGMEDIUM> {
        if fmt.cfFormat == self.preferred_drop_effect {
            return drop_effect_medium(DROPEFFECT_COPY);
        }
        if fmt.cfFormat == self.performed_drop_effect {
            return drop_effect_medium(self.performed_effect.get());
        }
        let hglobal = create_hglobal_for_paths(&self.paths)
            .map_err(|_| windows::core::Error::from_thread())?;
        Ok(STGMEDIUM {
            tymed: TYMED_HGLOBAL.0 as u32,
            u: STGMEDIUM_0 { hGlobal: hglobal },
            pUnkForRelease: ManuallyDrop::new(None),
        })
    }
}

#[allow(non_snake_case)]
impl windows::Win32::System::Com::IDataObject_Impl for FileDropDataObject_Impl {
    fn GetData(&self, formatetcin: *const FORMATETC) -> windows::core::Result<STGMEDIUM> {
        if formatetcin.is_null() {
            return Err(windows::core::Error::from(E_INVALIDARG));
        }
        let fmt = unsafe { &*formatetcin };
        if !self.matches_format(fmt) {
            return Err(windows::core::Error::from(DV_E_FORMATETC));
        }
        self.fill_medium(fmt)
    }

    fn GetDataHere(
        &self,
        _pformatetc: *const FORMATETC,
        _pmedium: *mut STGMEDIUM,
    ) -> windows::core::Result<()> {
        Err(windows::core::Error::from(DV_E_FORMATETC))
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
        if pformatetc.is_null() {
            return E_INVALIDARG;
        }
        let fmt = unsafe { &*pformatetc };
        if self.matches_format(fmt) {
            HRESULT(0)
        } else {
            DV_E_FORMATETC
        }
    }

    fn GetCanonicalFormatEtc(
        &self,
        pformatectin: *const FORMATETC,
        pformatetcout: *mut FORMATETC,
    ) -> HRESULT {
        if pformatectin.is_null() || pformatetcout.is_null() {
            return E_INVALIDARG;
        }
        unsafe {
            *pformatetcout = *pformatectin;
        }
        HRESULT(0)
    }

    fn SetData(
        &self,
        pformatetc: *const FORMATETC,
        pmedium: *const STGMEDIUM,
        _frelease: BOOL,
    ) -> windows::core::Result<()> {
        if pformatetc.is_null() || pmedium.is_null() {
            return Err(windows::core::Error::from(E_INVALIDARG));
        }
        let fmt = unsafe { &*pformatetc };
        if fmt.cfFormat != self.performed_drop_effect || (fmt.tymed & TYMED_HGLOBAL.0 as u32) == 0 {
            return Err(windows::core::Error::from(
                windows::Win32::Foundation::E_NOTIMPL,
            ));
        }
        let medium = unsafe { &*pmedium };
        if medium.tymed != TYMED_HGLOBAL.0 as u32 {
            return Err(windows::core::Error::from(E_INVALIDARG));
        }
        let handle = unsafe { medium.u.hGlobal };
        let ptr = unsafe { GlobalLock(handle) } as *const u32;
        if ptr.is_null() {
            unsafe {
                let _ = GlobalUnlock(handle);
            }
            return Err(windows::core::Error::from_thread());
        }
        let effect = unsafe { *ptr };
        self.performed_effect.set(DROPEFFECT(effect));
        unsafe {
            let _ = GlobalUnlock(handle);
        }
        Ok(())
    }

    fn EnumFormatEtc(&self, dwdirection: u32) -> windows::core::Result<IEnumFORMATETC> {
        if dwdirection != DATADIR_GET.0 as u32 {
            return Err(windows::core::Error::from(
                windows::Win32::Foundation::E_NOTIMPL,
            ));
        }
        let formats = [
            self.format,
            build_drop_effect_format(self.preferred_drop_effect),
            build_drop_effect_format(self.performed_drop_effect),
        ];
        unsafe { SHCreateStdEnumFmtEtc(&formats) }
    }

    fn DAdvise(
        &self,
        _pformatetc: *const FORMATETC,
        _advf: u32,
        _padvsink: Ref<'_, IAdviseSink>,
    ) -> windows::core::Result<u32> {
        Err(windows::core::Error::from(
            windows::Win32::Foundation::E_NOTIMPL,
        ))
    }

    fn DUnadvise(&self, _dwconnection: u32) -> windows::core::Result<()> {
        Err(windows::core::Error::from(
            windows::Win32::Foundation::E_NOTIMPL,
        ))
    }

    fn EnumDAdvise(&self) -> windows::core::Result<windows::Win32::System::Com::IEnumSTATDATA> {
        Err(windows::core::Error::from(
            windows::Win32::Foundation::E_NOTIMPL,
        ))
    }
}

#[implement(IDropSource)]
#[derive(Clone)]
struct SimpleDropSource;

#[allow(non_snake_case)]
impl windows::Win32::System::Ole::IDropSource_Impl for SimpleDropSource_Impl {
    fn QueryContinueDrag(&self, escape_pressed: BOOL, key_state: MODIFIERKEYS_FLAGS) -> HRESULT {
        if escape_pressed.as_bool() {
            return DRAGDROP_S_CANCEL;
        }
        if key_state.0 & MK_LBUTTON.0 == 0 {
            return DRAGDROP_S_DROP;
        }
        HRESULT(0)
    }

    fn GiveFeedback(&self, _dweffect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

pub(super) fn start_file_drag(
    hwnd: windows::Win32::Foundation::HWND,
    paths: &[PathBuf],
) -> Result<(), String> {
    info!(
        hwnd = ?hwnd,
        path_count = paths.len(),
        first_path = %paths
            .first()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        "external drag: starting Windows file drag"
    );
    let _com = ComApartment::new()?;
    let absolute: Vec<PathBuf> = paths
        .iter()
        .map(|path| normalize_path(path.as_path()))
        .collect();
    info!(
        normalized_path_count = absolute.len(),
        first_path = %absolute
            .first()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        "external drag: normalized drag payload paths"
    );
    let data_object: IDataObject = FileDropDataObject::new(absolute)?.into();
    let drop_source: IDropSource = SimpleDropSource.into();
    let mut effect = DROPEFFECT(0);
    let drag_result = unsafe {
        DoDragDrop(
            &data_object,
            &drop_source,
            DROPEFFECT_COPY | DROPEFFECT_LINK | DROPEFFECT_MOVE,
            &mut effect,
        )
    }
    .ok();
    match drag_result {
        Ok(()) => info!(effect = effect.0, "external drag: DoDragDrop returned success"),
        Err(ref err) => warn!(
            error = %err,
            effect = effect.0,
            "external drag: DoDragDrop returned failure"
        ),
    }
    drag_result.map_err(|err| format!("Drag failed: {err}"))?;

    if effect == DROPEFFECT_NONE {
        warn!("external drag: drop completed with DROPEFFECT_NONE");
        Err("Drag canceled or target rejected drop".into())
    } else {
        info!(effect = effect.0, "external drag: Windows file drag completed");
        Ok(())
    }
}

fn build_format() -> FORMATETC {
    FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

fn build_drop_effect_format(format: u16) -> FORMATETC {
    FORMATETC {
        cfFormat: format,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

fn drop_effect_formats() -> Result<(u16, u16), String> {
    static FORMATS: OnceLock<Result<(u16, u16), String>> = OnceLock::new();
    FORMATS
        .get_or_init(|| {
            let preferred = unsafe { RegisterClipboardFormatW(w!("Preferred DropEffect")) };
            let performed = unsafe { RegisterClipboardFormatW(w!("Performed DropEffect")) };
            if preferred == 0 || performed == 0 {
                Err("RegisterClipboardFormatW failed".to_string())
            } else {
                Ok((preferred as u16, performed as u16))
            }
        })
        .clone()
}

fn drop_effect_medium(effect: DROPEFFECT) -> windows::core::Result<STGMEDIUM> {
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
    Ok(STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 { hGlobal: handle },
        pUnkForRelease: ManuallyDrop::new(None),
    })
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

fn last_error_from_win32(err: windows::core::Error) -> std::io::Error {
    std::io::Error::from_raw_os_error(err.code().0)
}
