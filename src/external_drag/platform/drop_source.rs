use windows::Win32::Foundation::{
    DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS,
};
use windows::Win32::System::Ole::{DROPEFFECT, IDropSource};
use windows::Win32::System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS};
use windows::core::{BOOL, HRESULT};
use windows_implement::implement;

#[implement(IDropSource)]
#[derive(Clone)]
pub(super) struct SimpleDropSource;

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
