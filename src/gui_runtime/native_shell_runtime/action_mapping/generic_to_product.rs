use super::super::*;

impl From<runtime_contract::UiAction> for UiAction {
    fn from(value: runtime_contract::UiAction) -> Self {
        let value = match super::shell_sources::generic_to_product(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::browser_content::generic_to_product(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::waveform_editing::generic_to_product(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::app_commands::generic_to_product(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::waveform_options::generic_to_product(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::waveform_navigation::generic_to_product(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::waveform_ranges::generic_to_product(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::waveform_drag::generic_to_product(value) {
            Ok(action) => return action,
            Err(value) => value,
        };

        unreachable!(
            "shell/source action mapper must claim its domain before browser/waveform mapping: {value:?}"
        )
    }
}
