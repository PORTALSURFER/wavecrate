use super::super::*;

impl From<UiAction> for runtime_contract::UiAction {
    fn from(value: UiAction) -> Self {
        let value = match super::shell_sources::product_to_generic(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::browser_content::product_to_generic(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::waveform_editing::product_to_generic(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::app_commands::product_to_generic(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::waveform_options::product_to_generic(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::waveform_navigation::product_to_generic(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::waveform_ranges::product_to_generic(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::waveform_drag::product_to_generic(value) {
            Ok(action) => return action,
            Err(value) => value,
        };

        unreachable!(
            "shell/source action mapper must claim its domain before browser/waveform mapping: {value:?}"
        )
    }
}
