use std::sync::OnceLock;
use windows::Win32::Foundation::DV_E_FORMATETC;
use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Ole::CF_HDROP;
use windows::core::w;

pub(super) fn file_drop_format() -> FORMATETC {
    FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

pub(super) fn drop_effect_format(format: u16) -> FORMATETC {
    FORMATETC {
        cfFormat: format,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

pub(super) fn drop_effect_formats() -> Result<(u16, u16), String> {
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

pub(super) fn unsupported_format_error() -> windows::core::Error {
    windows::core::Error::from(DV_E_FORMATETC)
}
