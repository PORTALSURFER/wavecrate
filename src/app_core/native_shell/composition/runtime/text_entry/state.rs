use super::*;
use crate::gui::form::{DecimalTextInputPolicy, rounded_scaled_u16};

/// Sanitize inserted BPM text so the field only accepts digits and one decimal
/// separator while preserving the existing decimal point outside the selection.
pub(crate) fn sanitize_waveform_bpm_insert(
    current: &str,
    selection_range: (usize, usize),
    inserted: &str,
) -> String {
    DecimalTextInputPolicy::POSITIVE_FINITE.sanitize_insert(current, selection_range, inserted)
}

/// Parse a positive finite BPM value from one text field string.
pub(crate) fn parse_waveform_bpm_input(text: &str) -> Option<f32> {
    DecimalTextInputPolicy::POSITIVE_FINITE.parse_value(text)
}

fn parse_projected_waveform_bpm_label(label: &str) -> Option<String> {
    let number = label.split_ascii_whitespace().next()?.trim();
    if number.is_empty() {
        return None;
    }
    let parsed = parse_waveform_bpm_input(number)?;
    if parsed <= 0.0 {
        return None;
    }
    Some(number.to_string())
}

/// Convert one BPM value into the tenths-based runtime action representation.
pub(crate) fn bpm_tenths_from_value(value: f32) -> u16 {
    rounded_scaled_u16(value, 10.0)
}

pub(super) fn waveform_bpm_text_from_model<B: NativeAppBridge>(
    runner: &NativeVelloRunner<B>,
) -> String {
    runner
        .model
        .waveform
        .tempo_label
        .as_deref()
        .and_then(parse_projected_waveform_bpm_label)
        .unwrap_or_else(|| String::from("120.0"))
}

pub(super) fn current_text_value<B: NativeAppBridge>(
    runner: &NativeVelloRunner<B>,
) -> Option<String> {
    match runner.text_input_target {
        TextInputTarget::None => None,
        TextInputTarget::BrowserSearch
        | TextInputTarget::BrowserPillEditor
        | TextInputTarget::FolderSearch
        | TextInputTarget::FolderCreate
        | TextInputTarget::PromptInput => runner.text_input_buffer.clone().or_else(|| match runner
            .text_input_target
        {
            TextInputTarget::BrowserSearch => Some(runner.model.browser.search_query.clone()),
            TextInputTarget::BrowserPillEditor => {
                Some(runner.model.browser.pill_editor.input_value.clone())
            }
            TextInputTarget::FolderSearch => Some(runner.model.sources.tree_search_query.clone()),
            TextInputTarget::PromptInput => runner.model.confirm_prompt.input_value.clone(),
            TextInputTarget::FolderCreate => runner
                .folder_inline_edit_row()
                .and_then(|row| row.input_value.clone()),
            TextInputTarget::None | TextInputTarget::WaveformBpm => None,
        }),
        TextInputTarget::WaveformBpm => Some(
            runner
                .waveform_bpm_input_buffer
                .clone()
                .unwrap_or_else(|| runner.waveform_bpm_text_from_model()),
        ),
    }
}

pub(super) fn sync_text_input_target<B: NativeAppBridge>(runner: &mut NativeVelloRunner<B>) {
    if runner.model.confirm_prompt.visible && runner.model.confirm_prompt.input_value.is_some() {
        runner.text_input_target = TextInputTarget::PromptInput;
    } else if runner.text_input_target == TextInputTarget::PromptInput {
        runner.text_input_target = TextInputTarget::None;
    }
    let folder_inline_edit_row = runner.folder_inline_edit_row().cloned();
    let folder_inline_edit_focused = folder_inline_edit_row
        .as_ref()
        .is_some_and(|row| row.input_focused);
    if folder_inline_edit_focused {
        runner.text_input_target = TextInputTarget::FolderCreate;
    } else if runner.text_input_target == TextInputTarget::FolderCreate
        && folder_inline_edit_row.is_none()
    {
        runner.text_input_target = TextInputTarget::None;
    }
    if runner.text_input_target != TextInputTarget::None {
        match runner.text_input_target {
            TextInputTarget::BrowserSearch
            | TextInputTarget::BrowserPillEditor
            | TextInputTarget::FolderSearch
            | TextInputTarget::FolderCreate
            | TextInputTarget::PromptInput => {
                if runner.text_input_buffer.is_none() {
                    runner.text_input_buffer = Some(match runner.text_input_target {
                        TextInputTarget::BrowserSearch => runner.model.browser.search_query.clone(),
                        TextInputTarget::BrowserPillEditor => {
                            runner.model.browser.pill_editor.input_value.clone()
                        }
                        TextInputTarget::FolderSearch => {
                            runner.model.sources.tree_search_query.clone()
                        }
                        TextInputTarget::PromptInput => runner
                            .model
                            .confirm_prompt
                            .input_value
                            .clone()
                            .unwrap_or_default(),
                        TextInputTarget::FolderCreate => runner
                            .folder_inline_edit_row()
                            .and_then(|row| row.input_value.clone())
                            .unwrap_or_default(),
                        TextInputTarget::None | TextInputTarget::WaveformBpm => String::new(),
                    });
                }
            }
            TextInputTarget::WaveformBpm => {
                if runner.waveform_bpm_input_buffer.is_none() {
                    runner.waveform_bpm_input_buffer = Some(waveform_bpm_text_from_model(runner));
                }
            }
            TextInputTarget::None => {}
        }
        if let Some(row) = folder_inline_edit_row.as_ref()
            && runner.text_input_target == TextInputTarget::FolderCreate
        {
            let row_text = row.input_value.clone().unwrap_or_default();
            let should_seed_initial_text = runner
                .text_input_buffer
                .as_deref()
                .is_some_and(str::is_empty)
                && !row_text.is_empty();
            if should_seed_initial_text {
                runner.text_input_buffer = Some(row_text.clone());
            }
            if should_seed_initial_text
                && row.select_all_on_focus
                && let Some(editor) = runner.text_editor_state.as_mut()
            {
                editor.select_all(&row_text);
            }
        }
        let current_text = runner.current_text_value().unwrap_or_default();
        let mut editor = runner
            .text_editor_state
            .take()
            .unwrap_or_else(|| SingleLineTextEditorState::collapsed_at_end(&current_text));
        editor.clamp_to_text(&current_text);
        runner.text_editor_state = Some(editor);
    } else {
        runner.text_input_buffer = None;
        runner.text_editor_state = None;
        runner.text_input_drag_active = false;
    }
    if runner.text_input_target != TextInputTarget::WaveformBpm {
        runner.waveform_bpm_input_buffer = None;
    }
    runner.sync_waveform_bpm_editor_state();
    runner.sync_browser_search_editor_state();
    runner.sync_browser_pill_editor_state();
    runner.sync_folder_create_editor_state();
}

pub(super) fn set_text_value<B: NativeAppBridge>(
    runner: &mut NativeVelloRunner<B>,
    value: String,
) -> bool {
    let action = match runner.text_input_target {
        TextInputTarget::None => return false,
        TextInputTarget::BrowserSearch => {
            runner.text_input_buffer = Some(value.clone());
            UiAction::SetBrowserSearch { query: value }
        }
        TextInputTarget::BrowserPillEditor => {
            runner.text_input_buffer = Some(value.clone());
            UiAction::SetBrowserPillEditorInput { value }
        }
        TextInputTarget::FolderSearch => {
            runner.text_input_buffer = Some(value.clone());
            UiAction::SetFolderSearch {
                pane: Some(runner.model.sources.active_folder_pane),
                query: value,
            }
        }
        TextInputTarget::FolderCreate => {
            runner.text_input_buffer = Some(value.clone());
            UiAction::SetFolderCreateInput { value }
        }
        TextInputTarget::PromptInput => {
            runner.text_input_buffer = Some(value.clone());
            UiAction::SetPromptInput { value }
        }
        TextInputTarget::WaveformBpm => {
            runner.waveform_bpm_input_buffer = Some(value.clone());
            runner.sync_waveform_bpm_editor_state();
            runner.apply_invalidation_scope(super::RuntimeInvalidationScope::StaticAndOverlays);
            if let Some(parsed) = parse_waveform_bpm_input(&value) {
                UiAction::SetWaveformBpmValue {
                    value_tenths: bpm_tenths_from_value(parsed),
                }
            } else {
                return true;
            }
        }
    };
    runner.emit_model_action(action);
    runner.sync_browser_search_editor_state();
    runner.sync_browser_pill_editor_state();
    runner.sync_folder_create_editor_state();
    true
}

pub(super) fn append_text<B: NativeAppBridge>(
    runner: &mut NativeVelloRunner<B>,
    appended: &str,
) -> bool {
    let appended = sanitize_single_line_insert(appended);
    if appended.is_empty() {
        return false;
    }
    let Some(value) = runner.current_text_value() else {
        return false;
    };
    let Some(editor) = runner.text_editor_state.as_mut() else {
        return false;
    };
    let sanitized = if runner.text_input_target == TextInputTarget::WaveformBpm {
        sanitize_waveform_bpm_insert(&value, editor.selection_range(), &appended)
    } else {
        appended
    };
    if sanitized.is_empty() {
        return false;
    }
    let next = editor.replace_selection(&value, &sanitized);
    set_text_value(runner, next)
}

#[cfg(test)]
mod tests {
    use super::parse_projected_waveform_bpm_label;

    #[test]
    fn projected_waveform_bpm_label_parser_accepts_positive_numbers() {
        assert_eq!(
            parse_projected_waveform_bpm_label("128 BPM"),
            Some(String::from("128"))
        );
        assert_eq!(
            parse_projected_waveform_bpm_label("128.5 BPM"),
            Some(String::from("128.5"))
        );
    }

    #[test]
    fn projected_waveform_bpm_label_parser_rejects_invalid_labels() {
        assert_eq!(parse_projected_waveform_bpm_label(""), None);
        assert_eq!(parse_projected_waveform_bpm_label("0 BPM"), None);
        assert_eq!(parse_projected_waveform_bpm_label("-1 BPM"), None);
        assert_eq!(parse_projected_waveform_bpm_label("fast BPM"), None);
    }
}
