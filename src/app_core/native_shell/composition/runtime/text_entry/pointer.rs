use super::*;

fn text_value_for_input_target<B: NativeAppBridge>(
    runner: &NativeVelloRunner<B>,
    target: TextInputTarget,
) -> Option<String> {
    match target {
        TextInputTarget::None => None,
        TextInputTarget::BrowserSearch => Some(
            runner
                .current_text_value()
                .unwrap_or_else(|| runner.model.browser.search_query.clone()),
        ),
        TextInputTarget::BrowserPillEditor => Some(
            runner
                .current_text_value()
                .unwrap_or_else(|| runner.model.browser.pill_editor.input_value.clone()),
        ),
        TextInputTarget::WaveformBpm => Some(
            runner
                .current_text_value()
                .unwrap_or_else(|| runner.waveform_bpm_text_from_model()),
        ),
        TextInputTarget::FolderCreate => Some(runner.current_text_value().unwrap_or_else(|| {
            runner
                .folder_inline_edit_row()
                .and_then(|row| row.input_value.clone())
                .unwrap_or_default()
        })),
        TextInputTarget::FolderSearch | TextInputTarget::PromptInput => None,
    }
}

fn text_input_rect_for_target<B: NativeAppBridge>(
    runner: &mut NativeVelloRunner<B>,
    layout: &ShellLayout,
    target: TextInputTarget,
) -> Option<UiRect> {
    match target {
        TextInputTarget::BrowserSearch => runner
            .shell_state
            .browser_search_text_rect(layout, &runner.model),
        TextInputTarget::BrowserPillEditor => runner
            .shell_state
            .browser_pill_editor_text_rect(layout, &runner.model),
        TextInputTarget::WaveformBpm => runner
            .shell_state
            .waveform_bpm_text_rect(layout, &runner.model),
        TextInputTarget::FolderCreate => runner
            .shell_state
            .folder_create_text_rect(layout, &runner.model),
        TextInputTarget::None | TextInputTarget::FolderSearch | TextInputTarget::PromptInput => {
            None
        }
    }
}

fn text_click_byte_index<B: NativeAppBridge>(
    runner: &mut NativeVelloRunner<B>,
    layout: &ShellLayout,
    point: Point,
    target: TextInputTarget,
) -> Option<usize> {
    let text_rect = text_input_rect_for_target(runner, layout, target)?;
    let text = text_value_for_input_target(runner, target)?;
    let font_size = runner.cached_style_for_layout(layout).sizing.font_meta;
    let mut editor = runner
        .text_editor_state
        .clone()
        .unwrap_or_else(|| SingleLineTextEditorState::collapsed_at_end(&text));
    let layout_state = build_text_field_layout(
        &mut runner.text_renderer,
        &mut editor,
        &text,
        font_size,
        text_rect.width(),
    );
    Some(byte_index_for_local_x(
        &layout_state,
        (point.x - text_rect.min.x).clamp(0.0, text_rect.width()),
    ))
}

pub(super) fn sync_text_editor_visual_state_for_target<B: NativeAppBridge>(
    runner: &mut NativeVelloRunner<B>,
    target: TextInputTarget,
) {
    match target {
        TextInputTarget::BrowserSearch => runner.sync_browser_search_editor_state(),
        TextInputTarget::BrowserPillEditor => runner.sync_browser_pill_editor_state(),
        TextInputTarget::FolderCreate => runner.sync_folder_create_editor_state(),
        TextInputTarget::WaveformBpm => runner.sync_waveform_bpm_editor_state(),
        TextInputTarget::None | TextInputTarget::FolderSearch | TextInputTarget::PromptInput => {}
    }
}

fn activate_pointer_text_input_target<B: NativeAppBridge>(
    runner: &mut NativeVelloRunner<B>,
    target: TextInputTarget,
) {
    match target {
        TextInputTarget::BrowserSearch => {
            if runner.text_input_target != TextInputTarget::BrowserSearch {
                runner.emit_model_action(UiAction::FocusBrowserSearch);
                runner.activate_text_input_target(TextInputTarget::BrowserSearch);
            }
        }
        TextInputTarget::BrowserPillEditor => {
            if runner.text_input_target != TextInputTarget::BrowserPillEditor {
                runner.emit_model_action(UiAction::FocusBrowserPillEditorInput);
                runner.activate_text_input_target(TextInputTarget::BrowserPillEditor);
            }
        }
        TextInputTarget::WaveformBpm => {
            if runner.text_input_target != TextInputTarget::WaveformBpm {
                runner.activate_waveform_bpm_input();
            }
        }
        TextInputTarget::FolderCreate => {
            if runner.text_input_target != TextInputTarget::FolderCreate {
                runner.emit_model_action(UiAction::FocusFolderCreateInput);
                runner.activate_text_input_target(TextInputTarget::FolderCreate);
            }
        }
        TextInputTarget::None | TextInputTarget::FolderSearch | TextInputTarget::PromptInput => {}
    }
}

fn handle_text_input_pointer_press<B: NativeAppBridge>(
    runner: &mut NativeVelloRunner<B>,
    layout: &ShellLayout,
    field_rect: UiRect,
    point: Point,
    extend_selection: bool,
    target: TextInputTarget,
) -> bool {
    if !field_rect.contains(point) {
        return false;
    }
    activate_pointer_text_input_target(runner, target);
    let Some(byte_index) = text_click_byte_index(runner, layout, point, target) else {
        return false;
    };
    let Some(text) = text_value_for_input_target(runner, target) else {
        return false;
    };
    let editor = runner
        .text_editor_state
        .get_or_insert_with(|| SingleLineTextEditorState::collapsed_at_end(&text));
    editor.set_cursor(&text, byte_index, extend_selection);
    runner.text_input_drag_active = true;
    sync_text_editor_visual_state_for_target(runner, target);
    runner.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
    true
}

pub(super) fn handle_browser_search_pointer_press<B: NativeAppBridge>(
    runner: &mut NativeVelloRunner<B>,
    layout: &ShellLayout,
    point: Point,
    extend_selection: bool,
) -> bool {
    let Some(field_rect) = runner
        .shell_state
        .browser_search_field_rect(layout, &runner.model)
    else {
        return false;
    };
    handle_text_input_pointer_press(
        runner,
        layout,
        field_rect,
        point,
        extend_selection,
        TextInputTarget::BrowserSearch,
    )
}

pub(super) fn handle_waveform_bpm_pointer_press<B: NativeAppBridge>(
    runner: &mut NativeVelloRunner<B>,
    layout: &ShellLayout,
    point: Point,
    extend_selection: bool,
) -> bool {
    let Some(field_rect) = runner
        .shell_state
        .waveform_bpm_input_rect(layout, &runner.model)
    else {
        return false;
    };
    handle_text_input_pointer_press(
        runner,
        layout,
        field_rect,
        point,
        extend_selection,
        TextInputTarget::WaveformBpm,
    )
}

pub(super) fn handle_folder_create_pointer_press<B: NativeAppBridge>(
    runner: &mut NativeVelloRunner<B>,
    layout: &ShellLayout,
    point: Point,
    extend_selection: bool,
) -> bool {
    let Some(field_rect) = runner
        .shell_state
        .folder_create_input_rect(layout, &runner.model)
    else {
        return false;
    };
    handle_text_input_pointer_press(
        runner,
        layout,
        field_rect,
        point,
        extend_selection,
        TextInputTarget::FolderCreate,
    )
}

pub(super) fn process_text_input_drag<B: NativeAppBridge>(
    runner: &mut NativeVelloRunner<B>,
    point: Point,
) -> bool {
    if !runner.text_input_drag_active {
        return false;
    }
    let target = runner.text_input_target;
    let Some((byte_index, text)) = runner
        .with_shell_layout(|this, layout| {
            let byte_index = text_click_byte_index(this, layout, point, target)?;
            let text = text_value_for_input_target(this, target)?;
            Some((byte_index, text))
        })
        .flatten()
    else {
        return false;
    };
    let Some(editor) = runner.text_editor_state.as_mut() else {
        return false;
    };
    editor.set_cursor(&text, byte_index, true);
    sync_text_editor_visual_state_for_target(runner, target);
    runner.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
    true
}
