#[cfg(test)]
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy)]
struct WidgetIdNamespace {
    base: u64,
}

impl WidgetIdNamespace {
    const fn new(base: u64) -> Self {
        Self { base }
    }

    const fn id(self, offset: u16) -> u64 {
        self.base + offset as u64
    }
}

const WAVEFORM: WidgetIdNamespace = WidgetIdNamespace::new(0);
const FOLDER_TREE: WidgetIdNamespace = WidgetIdNamespace::new(29_000);
const SAMPLE_BROWSER: WidgetIdNamespace = WidgetIdNamespace::new(30_000);
const AUDIO_SETTINGS: WidgetIdNamespace = WidgetIdNamespace::new(31_000);
const TRANSACTION_HISTORY: WidgetIdNamespace = WidgetIdNamespace::new(31_200);
const TOOLBAR: WidgetIdNamespace = WidgetIdNamespace::new(32_100);
const FOLDER_FILTERS: WidgetIdNamespace = WidgetIdNamespace::new(0x5743_0000_0000_4600);
const SAMPLE_BROWSER_HEADER: WidgetIdNamespace = WidgetIdNamespace::new(0x5743_0000_0000_4800);
const COLLECTIONS: WidgetIdNamespace = WidgetIdNamespace::new(0x5743_0000_0000_4c00);
const METADATA_TAGS: WidgetIdNamespace = WidgetIdNamespace::new(0x5743_0000_0000_5440);

pub(in crate::native_app) const WAVEFORM_VIEWPORT_STACK_ID: u64 = WAVEFORM.id(10);
pub(in crate::native_app) const WAVEFORM_SIGNAL_WIDGET_ID: u64 = WAVEFORM.id(11);
pub(in crate::native_app) const WAVEFORM_WIDGET_ID: u64 = WAVEFORM.id(12);

pub(in crate::native_app) const FOLDER_TREE_LIST_ID: u64 = FOLDER_TREE.id(0);

pub(in crate::native_app) const SAMPLE_BROWSER_LIST_ID: u64 = SAMPLE_BROWSER.id(0);
pub(in crate::native_app) const SAMPLE_HEADER_SORT_DRAG_ID: u64 = SAMPLE_BROWSER_HEADER.id(1);
pub(in crate::native_app) const SAMPLE_HEADER_RESIZE_ID: u64 = SAMPLE_BROWSER_HEADER.id(2);

pub(in crate::native_app) const VOLUME_SLIDER_ID: u64 = AUDIO_SETTINGS.id(0);
pub(in crate::native_app) const AUDIO_ENGINE_PILL_ID: u64 = AUDIO_SETTINGS.id(100);
pub(in crate::native_app) const GENERAL_SETTINGS_BUTTON_ID: u64 = AUDIO_SETTINGS.id(110);

pub(in crate::native_app) const TRANSACTION_LIST_MODAL_ID: u64 = TRANSACTION_HISTORY.id(0);

pub(in crate::native_app) const TOOLBAR_FOCUS_LOADED_ID: u64 = TOOLBAR.id(0);
pub(in crate::native_app) const TOOLBAR_LOOP_ID: u64 = TOOLBAR.id(1);
pub(in crate::native_app) const TOOLBAR_PLAY_ID: u64 = TOOLBAR.id(2);
pub(in crate::native_app) const TOOLBAR_STOP_ID: u64 = TOOLBAR.id(3);
pub(in crate::native_app) const TOOLBAR_RANDOM_ID: u64 = TOOLBAR.id(4);

#[cfg(test)]
pub(in crate::native_app) const FILTER_SECTION_NODE_ID: u64 = FOLDER_FILTERS.id(1);
pub(in crate::native_app) const NAME_FILTER_INPUT_ID: u64 = FOLDER_FILTERS.id(2);
pub(in crate::native_app) const TAG_FILTER_INPUT_ID: u64 = FOLDER_FILTERS.id(3);

pub(in crate::native_app) const COLLECTIONS_SECTION_NODE_ID: u64 = COLLECTIONS.id(2);
pub(in crate::native_app) const COLLECTIONS_LIST_SCROLL_NODE_ID: u64 = COLLECTIONS.id(3);
#[cfg(test)]
pub(in crate::native_app) const EMPTY_COLLECTION_COUNT_NODE_ID: u64 = COLLECTIONS.id(4);

pub(in crate::native_app) const METADATA_TAG_INPUT_ID: u64 = METADATA_TAGS.id(7);
#[cfg(test)]
pub(in crate::native_app) const METADATA_SIDEBAR_PANEL_ID: u64 = METADATA_TAGS.id(8);
#[cfg(test)]
pub(in crate::native_app) const METADATA_TAG_LIBRARY_TOGGLE_ID: u64 = METADATA_TAGS.id(9);

#[cfg(test)]
#[derive(Clone, Copy)]
struct RegisteredWidgetId {
    name: &'static str,
    value: u64,
}

#[cfg(test)]
const REGISTERED_WIDGET_IDS: &[RegisteredWidgetId] = &[
    RegisteredWidgetId {
        name: "waveform.viewport_stack",
        value: WAVEFORM_VIEWPORT_STACK_ID,
    },
    RegisteredWidgetId {
        name: "waveform.signal_surface",
        value: WAVEFORM_SIGNAL_WIDGET_ID,
    },
    RegisteredWidgetId {
        name: "waveform.interaction_widget",
        value: WAVEFORM_WIDGET_ID,
    },
    RegisteredWidgetId {
        name: "folder_tree.list",
        value: FOLDER_TREE_LIST_ID,
    },
    RegisteredWidgetId {
        name: "sample_browser.list",
        value: SAMPLE_BROWSER_LIST_ID,
    },
    RegisteredWidgetId {
        name: "sample_browser.header_sort_drag",
        value: SAMPLE_HEADER_SORT_DRAG_ID,
    },
    RegisteredWidgetId {
        name: "sample_browser.header_resize",
        value: SAMPLE_HEADER_RESIZE_ID,
    },
    RegisteredWidgetId {
        name: "audio_settings.volume_slider",
        value: VOLUME_SLIDER_ID,
    },
    RegisteredWidgetId {
        name: "audio_settings.engine_pill",
        value: AUDIO_ENGINE_PILL_ID,
    },
    RegisteredWidgetId {
        name: "audio_settings.general_settings_button",
        value: GENERAL_SETTINGS_BUTTON_ID,
    },
    RegisteredWidgetId {
        name: "transaction_history.list_modal",
        value: TRANSACTION_LIST_MODAL_ID,
    },
    RegisteredWidgetId {
        name: "toolbar.focus_loaded",
        value: TOOLBAR_FOCUS_LOADED_ID,
    },
    RegisteredWidgetId {
        name: "toolbar.loop",
        value: TOOLBAR_LOOP_ID,
    },
    RegisteredWidgetId {
        name: "toolbar.play",
        value: TOOLBAR_PLAY_ID,
    },
    RegisteredWidgetId {
        name: "toolbar.stop",
        value: TOOLBAR_STOP_ID,
    },
    RegisteredWidgetId {
        name: "toolbar.random",
        value: TOOLBAR_RANDOM_ID,
    },
    RegisteredWidgetId {
        name: "folder_filters.section",
        value: FILTER_SECTION_NODE_ID,
    },
    RegisteredWidgetId {
        name: "folder_filters.name_input",
        value: NAME_FILTER_INPUT_ID,
    },
    RegisteredWidgetId {
        name: "folder_filters.tag_input",
        value: TAG_FILTER_INPUT_ID,
    },
    RegisteredWidgetId {
        name: "collections.section",
        value: COLLECTIONS_SECTION_NODE_ID,
    },
    RegisteredWidgetId {
        name: "collections.list_scroll",
        value: COLLECTIONS_LIST_SCROLL_NODE_ID,
    },
    RegisteredWidgetId {
        name: "collections.empty_count",
        value: EMPTY_COLLECTION_COUNT_NODE_ID,
    },
    RegisteredWidgetId {
        name: "metadata_tags.input",
        value: METADATA_TAG_INPUT_ID,
    },
    RegisteredWidgetId {
        name: "metadata_tags.sidebar_panel",
        value: METADATA_SIDEBAR_PANEL_ID,
    },
    RegisteredWidgetId {
        name: "metadata_tags.library_toggle",
        value: METADATA_TAG_LIBRARY_TOGGLE_ID,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_native_app_widget_ids_are_unique() {
        let mut seen = BTreeMap::new();
        for id in REGISTERED_WIDGET_IDS {
            assert_ne!(id.value, 0, "{} must not use the zero widget id", id.name);
            if let Some(previous) = seen.insert(id.value, id.name) {
                panic!(
                    "duplicate native app widget id {:#x}: {} and {}",
                    id.value, previous, id.name
                );
            }
        }
    }

    #[test]
    fn native_app_widget_id_constants_are_registry_backed() {
        let native_app_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/native_app");
        let mut violations = Vec::new();
        collect_raw_widget_id_constants(&native_app_dir, &mut violations);
        assert!(
            violations.is_empty(),
            "native app widget id constants must be declared in widget_ids.rs:\n{}",
            violations.join("\n")
        );
    }

    fn collect_raw_widget_id_constants(dir: &Path, violations: &mut Vec<String>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|error| {
            panic!("failed to read {}: {error}", dir.display());
        });
        for entry in entries {
            let entry = entry.expect("native app dir entry");
            let path = entry.path();
            if path.is_dir() {
                collect_raw_widget_id_constants(&path, violations);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            if path.file_name().and_then(|file_name| file_name.to_str()) == Some("widget_ids.rs") {
                continue;
            }
            collect_file_violations(&path, violations);
        }
    }

    fn collect_file_violations(path: &Path, violations: &mut Vec<String>) {
        let source = fs::read_to_string(path).unwrap_or_else(|error| {
            panic!("failed to read {}: {error}", path.display());
        });
        for (line_index, line) in source.lines().enumerate() {
            if !is_widget_id_const_line(line) {
                continue;
            }
            let statement = source
                .lines()
                .skip(line_index)
                .collect::<Vec<_>>()
                .join("\n");
            let statement = statement.split(';').next().unwrap_or(line);
            if statement.contains("widget_ids::") {
                continue;
            }
            violations.push(format!(
                "{}:{}: {}",
                path.display(),
                line_index + 1,
                line.trim()
            ));
        }
    }

    fn is_widget_id_const_line(line: &str) -> bool {
        line.contains("const ")
            && line.contains("_ID")
            && line.contains(": u64")
            && !line.contains("_SCOPE")
    }
}
