use super::super::normalize_path;
use super::com_apartment::ComApartment;
use super::data_object::FileDropDataObject;
use super::drop_source::SimpleDropSource;
use std::path::PathBuf;
use tracing::{info, warn};
use windows::Win32::System::Com::IDataObject;
use windows::Win32::System::Ole::{
    DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, DROPEFFECT_NONE, DoDragDrop,
    IDropSource,
};

pub(in crate::external_drag) fn start_file_drag(
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
        Ok(()) => info!(
            effect = effect.0,
            "external drag: DoDragDrop returned success"
        ),
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
        info!(
            effect = effect.0,
            "external drag: Windows file drag completed"
        );
        Ok(())
    }
}
