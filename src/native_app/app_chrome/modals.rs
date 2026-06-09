use crate::native_app::{
    app::{FileMoveConflictResolution, GuiMessage, NativeAppState},
    transaction_history::{TRANSACTION_LIST_MODAL_ID, TransactionListItem, TransactionListState},
};
use radiant::prelude as ui;

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
            ui::PanelSectionParts::new("Transactions", content)
                .style(ui::WidgetStyle::strong(ui::WidgetTone::Neutral))
                .padding(8.0)
                .spacing(6.0)
                .title_height(24.0),
            ui::Vector2::new(420.0, 300.0),
        )
        .horizontal(ui::LayerHorizontalAnchor::Center)
        .vertical(ui::LayerVerticalAnchor::Center),
        GuiMessage::CloseTransactionList,
    )
    .key("transaction-list-modal")
    .id(TRANSACTION_LIST_MODAL_ID)
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
    let content = ui::column([
        ui::text_line(summary, 22.0).fill_width(),
        ui::text_line(conflict.file_name, 24.0).fill_width(),
        ui::text_line(
            format!("Destination: {}", conflict.destination_folder),
            20.0,
        )
        .fill_width(),
        ui::row([
            ui::button("Overwrite")
                .danger()
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolution::Overwrite,
                ))
                .width(92.0)
                .height(24.0),
            ui::button("Rename")
                .primary()
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolution::Rename,
                ))
                .width(78.0)
                .height(24.0),
            ui::button("Skip")
                .message(GuiMessage::ResolveFileMoveConflict(
                    FileMoveConflictResolution::Skip,
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
            ui::PanelSectionParts::new("File Move Conflict", content)
                .style(ui::WidgetStyle::strong(ui::WidgetTone::Warning))
                .padding(8.0)
                .spacing(6.0)
                .title_height(24.0),
            ui::Vector2::new(430.0, 180.0),
        )
        .horizontal(ui::LayerHorizontalAnchor::Center)
        .vertical(ui::LayerVerticalAnchor::Center),
        GuiMessage::CancelFileMoveConflicts,
    )
    .key("file-move-conflict-modal")
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
