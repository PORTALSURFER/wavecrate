use crate::native_app::app::NativeAppState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct ShortcutHelpItem {
    pub(in crate::native_app) keys: &'static str,
    pub(in crate::native_app) action: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct ShortcutHelpSection {
    pub(in crate::native_app) title: &'static str,
    pub(in crate::native_app) items: Vec<ShortcutHelpItem>,
}

pub(in crate::native_app) fn shortcut_help_sections(
    state: &NativeAppState,
) -> Vec<ShortcutHelpSection> {
    let mut sections = contextual_shortcut_help_sections(state);
    sections.extend(default_shortcut_help_sections(state));
    sections
}

fn contextual_shortcut_help_sections(state: &NativeAppState) -> Vec<ShortcutHelpSection> {
    let mut sections = Vec::new();
    if state.ui.chrome.shortcut_help_open {
        sections.push(shortcut_help_section(
            "Shortcut Help",
            [
                shortcut_help_item("Esc", "Close shortcut help"),
                shortcut_help_item("Command-/", "Close shortcut help"),
            ],
        ));
    }
    if state.library.folder_browser.rename_active() {
        sections.push(shortcut_help_section(
            "Renaming",
            [shortcut_help_item("Esc", "Cancel rename")],
        ));
    }
    if state.library.folder_browser.file_column_drag_active() {
        sections.push(shortcut_help_section(
            "Column Drag",
            [shortcut_help_item("Esc", "Cancel column drag")],
        ));
    }
    if state.ui.browser_interaction.context_menu.is_some()
        || state.ui.browser_interaction.waveform_context_menu.is_some()
    {
        sections.push(shortcut_help_section(
            "Context Menu",
            [shortcut_help_item("Esc", "Close context menu")],
        ));
    }
    if state
        .ui
        .browser_interaction
        .pending_waveform_destructive_edit
        .is_some()
    {
        sections.push(shortcut_help_section(
            "Destructive Edit",
            [
                shortcut_help_item("Enter", "Apply pending edit"),
                shortcut_help_item("Esc", "Cancel pending edit"),
            ],
        ));
    }
    if state.audio_settings_dropdown_open()
        || state.ui.chrome.curation_filter_dropdown_open
        || state.ui.chrome.harvest_filter_dropdown_open
    {
        sections.push(shortcut_help_section(
            "Dropdown",
            [shortcut_help_item("Esc", "Close dropdown")],
        ));
    }
    if state.ui.chrome.job_details_open {
        sections.push(shortcut_help_section(
            "Jobs",
            [shortcut_help_item("Esc", "Close job details")],
        ));
    }
    if state.ui.chrome.transaction_list_open {
        sections.push(shortcut_help_section(
            "Transactions Modal",
            [shortcut_help_item("Esc", "Close transaction list")],
        ));
    }
    if state.metadata_tag_completion_active() {
        sections.push(shortcut_help_section(
            "Tag Completion",
            [
                shortcut_help_item("Esc", "Cancel tag entry"),
                shortcut_help_item("Up / Down", "Move completion selection"),
            ],
        ));
    }
    if state.metadata.selected_tag.is_some() {
        sections.push(shortcut_help_section(
            "Selected Tag",
            [shortcut_help_item(
                "Delete / Backspace",
                "Delete selected tag",
            )],
        ));
    }
    if state.library.folder_browser.collection_focus_active() {
        sections.push(shortcut_help_section(
            "Collection Focus",
            [shortcut_help_item("Esc", "Exit collection focus")],
        ));
    }
    sections
}

fn default_shortcut_help_sections(state: &NativeAppState) -> [ShortcutHelpSection; 7] {
    [
        shortcut_help_section(
            "Samples",
            [
                shortcut_help_item("Space", space_help_label(state)),
                shortcut_help_item("Shift-Space", "Play from current play start"),
                shortcut_help_item("Control-Space / Option-Space", "Play random sample section"),
                shortcut_help_item(
                    "Command-Left / Command-Right",
                    "Step through playback history",
                ),
                shortcut_help_item("X", x_help_label(state)),
                shortcut_help_item("H", "Toggle harvest done"),
                shortcut_help_item("Command-A", "Select all listed samples"),
                shortcut_help_item("Command-C", "Copy play selection or selected file"),
                shortcut_help_item("Command-X", "Cut selected files"),
                shortcut_help_item("Command-V", "Paste cut files into selected folder"),
                shortcut_help_item("N", new_item_help_label(state)),
                shortcut_help_item("F2 / Command-R", "Rename selected item"),
                shortcut_help_item("Delete / Backspace", "Delete selected item"),
            ],
        ),
        shortcut_help_section(
            "Waveform",
            [
                shortcut_help_item("Enter", "Apply edit mark edits"),
                shortcut_help_item("E", "Extract play selection or selected files"),
                shortcut_help_item("W", "Open context menu"),
                shortcut_help_item("Command-E", "Extract and trim selection"),
                shortcut_help_item("Z", "Zoom to play selection"),
                shortcut_help_item("Left / Right", "Slide play selection"),
                shortcut_help_item("C", "Crop selection"),
                shortcut_help_item("D", "Trim selection"),
                shortcut_help_item("R", "Reverse selection"),
                shortcut_help_item("M", "Mute selection"),
                shortcut_help_item("X", "Zoom out"),
                shortcut_help_item("L", "Toggle loop playback"),
            ],
        ),
        shortcut_help_section(
            "Navigation",
            [
                shortcut_help_item("Up / Down", "Move browser selection"),
                shortcut_help_item("Shift-Up / Shift-Down", "Extend sample selection"),
                shortcut_help_item(
                    "Command-Up / Command-Down",
                    "Move focus without changing marks",
                ),
                shortcut_help_item("Left", "Collapse selected folder"),
                shortcut_help_item("F", "Focus selected map node"),
            ],
        ),
        shortcut_help_section(
            "Ratings & Collections",
            [
                shortcut_help_item("[", "Lower selected rating"),
                shortcut_help_item("]", "Raise selected rating"),
                shortcut_help_item("1-6", "Toggle selected sample in collection"),
            ],
        ),
        shortcut_help_section(
            "Metadata",
            [
                shortcut_help_item("`", "Focus tag input"),
                shortcut_help_item("9", "Tag selected samples one-shot"),
                shortcut_help_item("0", "Tag selected samples loop"),
            ],
        ),
        shortcut_help_section(
            "Transactions",
            [
                shortcut_help_item("Command-Z", "Undo"),
                shortcut_help_item("Command-Shift-Z", "Redo"),
                shortcut_help_item("Command-Y", "Redo"),
                shortcut_help_item("Command-Shift-\\", "Toggle transaction list"),
            ],
        ),
        shortcut_help_section(
            "Help",
            [shortcut_help_item("Command-/", "Toggle shortcut help")],
        ),
    ]
}

fn space_help_label(state: &NativeAppState) -> &'static str {
    if state.ui.chrome.sticky_random_sample_range_playback {
        "Play random sample section"
    } else {
        "Play selected sample"
    }
}

fn x_help_label(state: &NativeAppState) -> &'static str {
    if super::waveform_zoom_out_shortcut_active(state) {
        "Zoom out"
    } else {
        "Mark sample and advance"
    }
}

fn shortcut_help_section(
    title: &'static str,
    items: impl IntoIterator<Item = ShortcutHelpItem>,
) -> ShortcutHelpSection {
    ShortcutHelpSection {
        title,
        items: items.into_iter().collect(),
    }
}

fn shortcut_help_item(keys: &'static str, action: &'static str) -> ShortcutHelpItem {
    ShortcutHelpItem { keys, action }
}

fn new_item_help_label(state: &NativeAppState) -> &'static str {
    if state.library.folder_browser.selected_file_id().is_some() {
        "Normalize selected samples"
    } else {
        "Create subfolder"
    }
}
