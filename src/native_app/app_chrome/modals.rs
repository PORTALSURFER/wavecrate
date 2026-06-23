use crate::native_app::{
    app::{
        FileMoveConflictResolution, FileMoveConflictResolutionRequest, GuiMessage, NativeAppState,
        ShortcutHelpItem, ShortcutHelpSection, shortcut_help_sections,
    },
    transaction_history::{TRANSACTION_LIST_MODAL_ID, TransactionListItem, TransactionListState},
};
use radiant::prelude as ui;

const SHORTCUT_HELP_MODAL_WIDTH: f32 = 860.0;
const SHORTCUT_HELP_MODAL_HEIGHT: f32 = 640.0;
const SHORTCUT_HELP_KEY_WIDTH: f32 = 190.0;

pub(in crate::native_app) fn transaction_list(state: &NativeAppState) -> ui::View<GuiMessage> {
    let items = state.transactions.history.list_items();
    let summary = transaction_list_summary(state);
    let list = if items.is_empty() {
        ui::column([
            ui::text_line("No transactions registered", 24.0).fill_width(),
            ui::text_line("Undoable actions will appear here.", 22.0).fill_width(),
        ])
        .spacing(4.0)
        .fill_width()
    } else {
        ui::scroll(
            ui::column(
                items
                    .into_iter()
                    .map(transaction_list_row)
                    .collect::<Vec<_>>(),
            )
            .spacing(4.0)
            .fill_width(),
        )
        .fill_width()
        .fill_height()
    };
    let content = ui::column([summary, list])
        .spacing(6.0)
        .fill_width()
        .fill_height();

    ui::closeable_panel_section_layer_from_parts(
        ui::PanelSectionLayerParts::new(
            ui::PanelSectionParts::dialog("Transactions", content, ui::WidgetTone::Neutral),
            ui::Vector2::new(420.0, 300.0),
        ),
        GuiMessage::CloseTransactionList,
    )
    .key("transaction-list-modal")
    .id(TRANSACTION_LIST_MODAL_ID)
}

pub(in crate::native_app) fn shortcut_help(state: &NativeAppState) -> ui::View<GuiMessage> {
    let sections = shortcut_help_sections(state);
    let body = ui::scroll(
        ui::column(
            sections
                .into_iter()
                .map(shortcut_help_section_view)
                .collect::<Vec<_>>(),
        )
        .spacing(8.0)
        .fill_width(),
    )
    .fill_width()
    .fill_height();
    let content = ui::column([
        ui::text_line(
            "Context-aware keyboard shortcuts. Press Esc or Command-/ to close.",
            20.0,
        )
        .muted_text()
        .fill_width(),
        body,
    ])
    .spacing(6.0)
    .fill_width()
    .fill_height();

    ui::closeable_panel_section_layer_from_parts(
        ui::PanelSectionLayerParts::new(
            ui::PanelSectionParts::dialog("Shortcuts", content, ui::WidgetTone::Neutral),
            ui::Vector2::new(SHORTCUT_HELP_MODAL_WIDTH, SHORTCUT_HELP_MODAL_HEIGHT),
        ),
        GuiMessage::CloseShortcutHelp,
    )
    .key("shortcut-help-modal")
}

pub(in crate::native_app) fn file_move_conflict(state: &NativeAppState) -> ui::View<GuiMessage> {
    let conflict = state
        .library
        .folder_browser
        .pending_file_move_conflict_view()
        .expect("file move conflict modal requires pending conflict state");
    let summary = format!(
        "Conflict {} of {}",
        conflict.current_number, conflict.total_count
    );
    let apply_to_remaining = state
        .ui
        .browser_interaction
        .file_move_conflict_apply_to_remaining;
    let content = ui::column([
        ui::text_line(summary, 22.0).fill_width(),
        ui::text_line(conflict.file_name, 24.0).fill_width(),
        ui::text_line(
            format!("Destination: {}", conflict.destination_folder),
            20.0,
        )
        .fill_width(),
        ui::row([
            ui::checkbox(apply_to_remaining)
                .message(GuiMessage::SetFileMoveConflictApplyToRemaining)
                .width(20.0)
                .height(20.0),
            ui::text_line("Apply to all remaining conflicts", 20.0).fill_width(),
        ])
        .spacing(6.0)
        .fill_width()
        .height(22.0),
        ui::row([
            ui::button("Overwrite")
                .danger()
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolutionRequest::new(
                        FileMoveConflictResolution::Overwrite,
                        apply_to_remaining,
                    ),
                ))
                .width(92.0)
                .height(24.0),
            ui::button("Rename")
                .primary()
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolutionRequest::new(
                        FileMoveConflictResolution::Rename,
                        apply_to_remaining,
                    ),
                ))
                .width(78.0)
                .height(24.0),
            ui::button("Skip")
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolutionRequest::new(
                        FileMoveConflictResolution::Skip,
                        apply_to_remaining,
                    ),
                ))
                .width(64.0)
                .height(24.0),
        ])
        .spacing(6.0)
        .fill_width()
        .height(26.0),
    ])
    .spacing(6.0)
    .fill_width()
    .fill_height();

    ui::closeable_panel_section_layer_from_parts(
        ui::PanelSectionLayerParts::new(
            ui::PanelSectionParts::dialog("File Move Conflict", content, ui::WidgetTone::Warning),
            ui::Vector2::new(430.0, 210.0),
        ),
        GuiMessage::CancelFileMoveConflicts,
    )
    .key("file-move-conflict-modal")
}

pub(in crate::native_app) fn folder_delete_confirmation(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    let target = state
        .ui
        .browser_interaction
        .pending_folder_delete
        .as_ref()
        .expect("folder delete modal requires pending folder delete state");
    let content = ui::column([
        ui::text_line(target.name.clone(), 24.0).fill_width(),
        ui::text_line("Move folder contents to the configured trash folder?", 20.0).fill_width(),
        ui::text_line(
            "The folder tree will update after the move completes.",
            20.0,
        )
        .fill_width(),
        ui::row([
            ui::button("Delete Folder")
                .danger()
                .message(GuiMessage::ConfirmContextFolderDelete)
                .width(112.0)
                .height(24.0),
            ui::button("Cancel")
                .message(GuiMessage::CancelContextFolderDelete)
                .width(72.0)
                .height(24.0),
        ])
        .spacing(6.0)
        .fill_width()
        .height(26.0),
    ])
    .spacing(6.0)
    .fill_width()
    .fill_height();

    ui::closeable_panel_section_layer_from_parts(
        ui::PanelSectionLayerParts::new(
            ui::PanelSectionParts::dialog("Delete Folder", content, ui::WidgetTone::Warning),
            ui::Vector2::new(440.0, 190.0),
        ),
        GuiMessage::CancelContextFolderDelete,
    )
    .key("folder-delete-confirmation-modal")
}

pub(in crate::native_app) fn waveform_destructive_edit_confirmation(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    let pending = state
        .ui
        .browser_interaction
        .pending_waveform_destructive_edit
        .as_ref()
        .expect("waveform destructive modal requires pending edit state");
    let content = ui::column([
        ui::text_line(pending.prompt.title.clone(), 24.0).fill_width(),
        ui::text_line(pending.prompt.message.clone(), 20.0).fill_width(),
        ui::row([
            ui::button("Apply Edit")
                .danger()
                .message(GuiMessage::ConfirmPendingWaveformDestructiveEdit)
                .width(92.0)
                .height(24.0),
            ui::button("Cancel")
                .message(GuiMessage::CancelPendingWaveformDestructiveEdit)
                .width(72.0)
                .height(24.0),
        ])
        .spacing(6.0)
        .fill_width()
        .height(26.0),
    ])
    .spacing(6.0)
    .fill_width()
    .fill_height();

    ui::closeable_panel_section_layer_from_parts(
        ui::PanelSectionLayerParts::new(
            ui::PanelSectionParts::dialog("Destructive Edit", content, ui::WidgetTone::Warning),
            ui::Vector2::new(500.0, 190.0),
        ),
        GuiMessage::CancelPendingWaveformDestructiveEdit,
    )
    .key("waveform-destructive-edit-modal")
}

fn transaction_list_summary(state: &NativeAppState) -> ui::View<GuiMessage> {
    let undo = if state.transactions.history.can_undo() {
        "undo ready"
    } else {
        "no undo"
    };
    let redo = if state.transactions.history.can_redo() {
        "redo ready"
    } else {
        "no redo"
    };
    let active = if state.transactions.history.is_transaction_open() {
        "open transaction"
    } else {
        "closed"
    };
    ui::text_line(format!("{undo} | {redo} | {active}"), 20.0)
        .key("transaction-list-summary")
        .fill_width()
}

fn shortcut_help_section_view(section: ShortcutHelpSection) -> ui::View<GuiMessage> {
    ui::column([
        ui::text_line(section.title, 20.0)
            .style(ui::WidgetStyle::strong(ui::WidgetTone::Accent))
            .fill_width(),
        ui::column(
            section
                .items
                .into_iter()
                .map(shortcut_help_row)
                .collect::<Vec<_>>(),
        )
        .spacing(2.0)
        .fill_width(),
    ])
    .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
    .padding(6.0)
    .spacing(4.0)
    .fill_width()
}

fn shortcut_help_row(item: ShortcutHelpItem) -> ui::View<GuiMessage> {
    ui::row([
        ui::passive_badge(item.keys)
            .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
            .width(SHORTCUT_HELP_KEY_WIDTH)
            .height(20.0),
        ui::text_line(item.action, 20.0).fill_width(),
    ])
    .spacing(6.0)
    .fill_width()
    .height(22.0)
}

fn transaction_list_row(item: TransactionListItem) -> ui::View<GuiMessage> {
    let action_label = match item.action_count {
        1 => String::from("1 action"),
        count => format!("{count} actions"),
    };
    let action_summary = if item.action_labels.is_empty() {
        action_label
    } else {
        format!("{}: {}", action_label, item.action_labels.join(", "))
    };
    ui::row([
        ui::passive_badge(item.state.label().to_string())
            .style(transaction_list_state_style(item.state))
            .size(58.0, 20.0),
        ui::text_line(item.label, 22.0).fill_width(),
        ui::text_line(action_summary, 22.0).width(150.0),
    ])
    .key(format!("transaction-list-row-{}", item.id))
    .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
    .padding_x(6.0)
    .spacing(6.0)
    .fill_width()
    .height(26.0)
}

fn transaction_list_state_style(state: TransactionListState) -> ui::WidgetStyle {
    match state {
        TransactionListState::Active => ui::WidgetStyle::strong(ui::WidgetTone::Warning),
        TransactionListState::Undoable => ui::WidgetStyle::strong(ui::WidgetTone::Accent),
        TransactionListState::Redoable => ui::WidgetStyle::subtle(ui::WidgetTone::Neutral),
    }
}
