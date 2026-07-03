use std::collections::{BTreeMap, HashMap, HashSet};

use radiant::prelude as ui;
use wavecrate::sample_sources::config::AppSettingsCore;

use crate::native_app::app::SampleNameViewMode;
use crate::native_app::metadata::MetadataTagInputMode;

pub(in crate::native_app) struct MetadataAppState {
    pub(in crate::native_app) tag_draft: String,
    pub(in crate::native_app) tag_tokens: Vec<String>,
    pub(in crate::native_app) tag_input_mode: MetadataTagInputMode,
    pub(in crate::native_app) pending_tag_completion_query: Option<String>,
    pub(in crate::native_app) tag_completion_cycle: ui::CyclicListSelectionCycle,
    pub(in crate::native_app) tag_dictionary: BTreeMap<String, String>,
    pub(in crate::native_app) tag_library_open: bool,
    pub(in crate::native_app) tag_drag: Option<String>,
    pub(in crate::native_app) tag_drop_hover: Option<String>,
    pub(in crate::native_app) selected_tag: Option<String>,
    pub(in crate::native_app) collapsed_tag_categories: HashSet<String>,
    pub(in crate::native_app) persisted_tag_sources_loaded: HashSet<String>,
    pub(in crate::native_app) persisted_tag_sources_pending: HashSet<String>,
    pub(in crate::native_app) tags_by_file: HashMap<String, Vec<String>>,
    pub(in crate::native_app) sample_name_view_mode: SampleNameViewMode,
}

impl MetadataAppState {
    pub(in crate::native_app) fn from_settings(settings: &AppSettingsCore) -> Self {
        Self {
            tag_draft: String::new(),
            tag_tokens: Vec::new(),
            tag_input_mode: Default::default(),
            pending_tag_completion_query: None,
            tag_completion_cycle: ui::CyclicListSelectionCycle::new(),
            tag_dictionary: settings.tag_dictionary.clone(),
            tag_library_open: false,
            tag_drag: None,
            tag_drop_hover: None,
            selected_tag: None,
            collapsed_tag_categories: Default::default(),
            persisted_tag_sources_loaded: Default::default(),
            persisted_tag_sources_pending: Default::default(),
            tags_by_file: HashMap::new(),
            sample_name_view_mode: SampleNameViewMode::DiskFilename,
        }
    }
}
