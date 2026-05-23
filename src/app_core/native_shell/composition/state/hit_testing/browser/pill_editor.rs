use super::super::super::browser_pill_editor::browser_pill_editor_layout;
use super::*;

pub(super) fn browser_pill_editor_action_at_point(
    rows_rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let layout = browser_pill_editor_layout(rows_rect, sizing, model)?;
    if layout.auto_rename_rect.contains(point) {
        return Some(UiAction::ToggleBrowserPillEditorPrimaryAction);
    }
    if layout.input_rect.contains(point) {
        return Some(UiAction::FocusBrowserPillEditorInput);
    }
    for (index, rect) in layout.playback_rects.iter().enumerate() {
        if rect.contains(point) {
            return Some(UiAction::SetBrowserSidebarLooped { looped: index == 0 });
        }
    }
    for (pill, rect) in model
        .browser
        .pill_editor()
        .option_pills
        .iter()
        .zip(layout.normal_tag_rects.iter())
    {
        if rect.contains(point) {
            return Some(UiAction::ToggleBrowserPillOption {
                label: pill.label.clone(),
            });
        }
    }
    if let (Some(pill), Some(rect)) = (
        model.browser.pill_editor().create_pill.as_ref(),
        layout.create_tag_rect,
    ) && rect.contains(point)
    {
        return Some(UiAction::ToggleBrowserPillOption {
            label: pill.id.clone(),
        });
    }
    None
}
