use radiant::prelude as ui;
use wavecrate::sample_sources::SampleCollection;

use crate::native_app::app::{GuiMessage, MetadataMessage};

use super::{
    ShortcutHelpBinding, ShortcutHelpEntry, command_shift_press, shortcut_binding,
    shortcut_help_entry,
};

pub(super) fn rating_and_collection_shortcut_help_entries() -> Vec<ShortcutHelpEntry> {
    vec![
        shortcut_help_entry(
            "Ratings & Collections",
            "[",
            "Lower selected rating",
            [shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::OpenBracket),
                GuiMessage::AdjustSelectedRatingWithoutAdvance(-1),
            )],
        ),
        shortcut_help_entry(
            "Ratings & Collections",
            "]",
            "Raise selected rating",
            [shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::CloseBracket),
                GuiMessage::AdjustSelectedRatingWithoutAdvance(1),
            )],
        ),
        shortcut_help_entry(
            "Ratings & Collections",
            "1-6",
            "Toggle selected sample in collection",
            collection_shortcut_bindings(),
        ),
    ]
}

pub(super) fn metadata_shortcut_help_entries() -> Vec<ShortcutHelpEntry> {
    vec![
        shortcut_help_entry(
            "Metadata",
            "`",
            "Focus tag input",
            [shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::Backquote),
                GuiMessage::Metadata(MetadataMessage::FocusMetadataTagInput),
            )],
        ),
        metadata_tag_entry("9", ui::KeyCode::Num9, "one-shot"),
        metadata_tag_entry("0", ui::KeyCode::Num0, "loop"),
    ]
}

pub(super) fn transaction_shortcut_help_entries() -> Vec<ShortcutHelpEntry> {
    vec![
        transaction_entry(
            "Command-Z",
            "Undo",
            ui::KeyPress::with_command(ui::KeyCode::Z),
            GuiMessage::UndoTransaction,
        ),
        transaction_entry(
            "Command-Shift-Z",
            "Redo",
            command_shift_press(ui::KeyCode::Z),
            GuiMessage::RedoTransaction,
        ),
        transaction_entry(
            "Command-Y",
            "Redo",
            ui::KeyPress::with_command(ui::KeyCode::Y),
            GuiMessage::RedoTransaction,
        ),
        transaction_entry(
            "Command-Shift-\\",
            "Toggle transaction list",
            super::super::transaction_list_shortcut(),
            GuiMessage::ToggleTransactionList,
        ),
    ]
}

fn metadata_tag_entry(
    keys: &'static str,
    key: ui::KeyCode,
    tag: &'static str,
) -> ShortcutHelpEntry {
    shortcut_help_entry(
        "Metadata",
        keys,
        if tag == "one-shot" {
            "Tag selected samples one-shot"
        } else {
            "Tag selected samples loop"
        },
        [shortcut_binding(
            ui::KeyPress::new(key),
            GuiMessage::Metadata(MetadataMessage::ToggleMetadataTag(String::from(tag))),
        )],
    )
}

fn transaction_entry(
    keys: &'static str,
    action: &'static str,
    press: ui::KeyPress,
    message: GuiMessage,
) -> ShortcutHelpEntry {
    shortcut_help_entry(
        "Transactions",
        keys,
        action,
        [shortcut_binding(press, message)],
    )
}

fn collection_shortcut_bindings() -> Vec<ShortcutHelpBinding> {
    [
        (ui::KeyCode::Num1, 0),
        (ui::KeyCode::Num2, 1),
        (ui::KeyCode::Num3, 2),
        (ui::KeyCode::Num4, 3),
        (ui::KeyCode::Num5, 4),
        (ui::KeyCode::Num6, 5),
    ]
    .into_iter()
    .map(|(key, index)| {
        let collection = SampleCollection::new(index).expect("collection shortcut index is valid");
        shortcut_binding(
            ui::KeyPress::new(key),
            GuiMessage::AssignSelectedCollection(collection),
        )
    })
    .collect()
}
