mod identity;
mod projection;
#[cfg(test)]
mod tests;

use crate::native_app::{
    app::{
        FileMoveConflictResolution, FileMoveConflictResolutionRequest, GuiMessage,
        ShortcutHelpItem, ShortcutHelpSection,
    },
    transaction_history::TransactionListState,
};
use radiant::prelude as ui;

use self::projection::{
    FileMoveConflictProjection, FolderDeleteConfirmationProjection, ShortcutHelpProjection,
    TransactionListProjection, TransactionListRowProjection, WaveformDestructiveEditProjection,
};
use crate::native_app::app::NativeAppState;

const SHORTCUT_HELP_MODAL_WIDTH: f32 = 860.0;
const SHORTCUT_HELP_MODAL_HEIGHT: f32 = 640.0;
const SHORTCUT_HELP_KEY_WIDTH: f32 = 190.0;

pub(in crate::native_app) fn transaction_list(state: &NativeAppState) -> ui::View<GuiMessage> {
    let projection = TransactionListProjection::from_state(state);
    let summary = transaction_list_summary(&projection);
    let list = if projection.rows.is_empty() {
        ui::column([
            ui::text_line(projection.empty_title, 24.0).fill_width(),
            ui::text_line(projection.empty_detail, 22.0).fill_width(),
        ])
        .spacing(4.0)
        .fill_width()
    } else {
        ui::scroll(
            ui::column(
                projection
                    .rows
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

    ui::closeable_dialog_layer(
        "Transactions",
        content,
        ui::WidgetTone::Neutral,
        ui::Vector2::new(420.0, 300.0),
        GuiMessage::CloseTransactionList,
    )
    .id(identity::TRANSACTION_LIST_MODAL_ID)
}

pub(in crate::native_app) fn shortcut_help(state: &NativeAppState) -> ui::View<GuiMessage> {
    let projection = ShortcutHelpProjection::from_state(state);
    let body = ui::scroll(
        ui::column(
            projection
                .sections
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
        ui::text_line(projection.intro, 20.0)
            .muted_text()
            .fill_width(),
        body,
    ])
    .spacing(6.0)
    .fill_width()
    .fill_height();

    ui::closeable_dialog_layer(
        "Shortcuts",
        content,
        ui::WidgetTone::Neutral,
        ui::Vector2::new(SHORTCUT_HELP_MODAL_WIDTH, SHORTCUT_HELP_MODAL_HEIGHT),
        GuiMessage::CloseShortcutHelp,
    )
    .key(identity::SHORTCUT_HELP_MODAL_KEY)
}

pub(in crate::native_app) fn file_move_conflict(state: &NativeAppState) -> ui::View<GuiMessage> {
    let projection = FileMoveConflictProjection::from_state(state);
    let apply_to_remaining = projection.apply_to_remaining;
    let content = ui::column([
        ui::text_line(projection.summary, 22.0).fill_width(),
        ui::text_line(projection.file_name, 24.0).fill_width(),
        ui::text_line(projection.destination, 20.0).fill_width(),
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
        ui::button_row([
            ui::button("Overwrite")
                .danger()
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolutionRequest::new(
                        FileMoveConflictResolution::Overwrite,
                        apply_to_remaining,
                    ),
                ))
                .width(92.0),
            ui::button("Rename")
                .primary()
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolutionRequest::new(
                        FileMoveConflictResolution::Rename,
                        apply_to_remaining,
                    ),
                ))
                .width(78.0),
            ui::button("Skip")
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolutionRequest::new(
                        FileMoveConflictResolution::Skip,
                        apply_to_remaining,
                    ),
                ))
                .width(64.0),
        ]),
    ])
    .spacing(6.0)
    .fill_width()
    .fill_height();

    ui::closeable_dialog_layer(
        "File Move Conflict",
        content,
        ui::WidgetTone::Warning,
        ui::Vector2::new(430.0, 210.0),
        GuiMessage::CancelFileMoveConflicts,
    )
    .key(identity::FILE_MOVE_CONFLICT_MODAL_KEY)
}

pub(in crate::native_app) fn folder_delete_confirmation(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    let projection = FolderDeleteConfirmationProjection::from_state(state);
    let content = ui::column([
        ui::text_line(projection.name, 24.0).fill_width(),
        ui::text_line(projection.question, 20.0).fill_width(),
        ui::text_line(projection.detail, 20.0).fill_width(),
        ui::button_row([
            ui::button("Delete Folder")
                .danger()
                .message(GuiMessage::ConfirmContextFolderDelete)
                .width(112.0),
            ui::button("Cancel")
                .message(GuiMessage::CancelContextFolderDelete)
                .width(72.0),
        ]),
    ])
    .spacing(6.0)
    .fill_width()
    .fill_height();

    ui::closeable_dialog_layer(
        "Delete Folder",
        content,
        ui::WidgetTone::Warning,
        ui::Vector2::new(440.0, 190.0),
        GuiMessage::CancelContextFolderDelete,
    )
    .key(identity::FOLDER_DELETE_CONFIRMATION_MODAL_KEY)
}

pub(in crate::native_app) fn waveform_destructive_edit_confirmation(
    state: &NativeAppState,
) -> ui::View<GuiMessage> {
    let projection = WaveformDestructiveEditProjection::from_state(state);
    let content = ui::column([
        ui::text_line(projection.title, 24.0).fill_width(),
        ui::text_line(projection.message, 20.0).fill_width(),
        ui::button_row([
            ui::button("Apply Edit")
                .danger()
                .message(GuiMessage::ConfirmPendingWaveformDestructiveEdit)
                .width(92.0),
            ui::button("Cancel")
                .message(GuiMessage::CancelPendingWaveformDestructiveEdit)
                .width(72.0),
        ]),
    ])
    .spacing(6.0)
    .fill_width()
    .fill_height();

    ui::closeable_dialog_layer(
        "Destructive Edit",
        content,
        ui::WidgetTone::Warning,
        ui::Vector2::new(500.0, 190.0),
        GuiMessage::CancelPendingWaveformDestructiveEdit,
    )
    .key(identity::WAVEFORM_DESTRUCTIVE_EDIT_MODAL_KEY)
}

fn transaction_list_summary(projection: &TransactionListProjection) -> ui::View<GuiMessage> {
    ui::text_line(projection.summary.clone(), 20.0)
        .key(identity::TRANSACTION_LIST_SUMMARY_KEY)
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

fn transaction_list_row(row: TransactionListRowProjection) -> ui::View<GuiMessage> {
    ui::row([
        ui::passive_badge(row.state.label().to_string())
            .style(transaction_list_state_style(row.state))
            .size(58.0, 20.0),
        ui::text_line(row.label, 22.0).fill_width(),
        ui::text_line(row.action_summary, 22.0).width(150.0),
    ])
    .key(identity::transaction_list_row_key(row.id))
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
