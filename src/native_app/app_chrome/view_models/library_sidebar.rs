use radiant::prelude as ui;
use wavecrate::sample_sources::SourceRole;

use crate::native_app::app::NativeAppState;
use crate::native_app::metadata::MetadataTagDisplayCategory;
use crate::native_app::sample_library::folder_browser::view_contract::{
    CollectionRenameView, FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_OVERSCAN_ROWS,
    FOLDER_TREE_PROJECTED_VIEWPORT_ROWS, SampleCollectionView,
};
use crate::native_app::sample_library::folder_browser::{
    FolderBrowserState,
    model::{
        BROWSER_CURATION_SCOPES, BrowserCurationScope, HARVEST_FILTERS, HarvestFilter,
        PLAYBACK_TYPE_FILTERS, PlaybackTypeFilter, RATING_FILTER_LEVELS, SourceEntry,
        VisibleFolder, playback_type_filter_label, rating_filter_label,
    },
};
use crate::native_app::sample_library::harvest_tracking::HarvestFamilySummary;

pub(in crate::native_app) struct LibrarySidebarViewModel {
    pub(in crate::native_app) sidebar_width: f32,
    pub(in crate::native_app) metadata_panel_height: f32,
    pub(in crate::native_app) source_selector: SourceSelectorViewModel,
    pub(in crate::native_app) folder_tree: FolderTreeViewModel,
    pub(in crate::native_app) collections: CollectionsSectionViewModel,
    pub(in crate::native_app) filter: FilterSectionViewModel,
    pub(in crate::native_app) harvest_family: Option<HarvestFamilyViewModel>,
    pub(in crate::native_app) tag_editor: TagEditorViewModel,
}

pub(in crate::native_app) struct SourceSelectorViewModel {
    pub(in crate::native_app) rows: Vec<SourceRowViewModel>,
    pub(in crate::native_app) missing_count: usize,
    pub(in crate::native_app) help_tooltips_enabled: bool,
}

pub(in crate::native_app) struct SourceRowViewModel {
    pub(in crate::native_app) id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) role: SourceRole,
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) focused: bool,
    pub(in crate::native_app) focus_alpha: u8,
    pub(in crate::native_app) reorder_enabled: bool,
    pub(in crate::native_app) reorder_drag_active: bool,
    pub(in crate::native_app) reorder_drag_source: bool,
    pub(in crate::native_app) reorder_drop_target: bool,
    pub(in crate::native_app) reorder_drop_after: bool,
    pub(in crate::native_app) scanning: bool,
    pub(in crate::native_app) missing: bool,
    pub(in crate::native_app) protected_source_error_flash: bool,
    pub(in crate::native_app) primary_source_acceptance_flash: bool,
    pub(in crate::native_app) drag_active: bool,
    pub(in crate::native_app) drop_candidate: bool,
    pub(in crate::native_app) drop_target: bool,
    pub(in crate::native_app) drop_target_active: bool,
}

pub(in crate::native_app) struct FolderTreeViewModel {
    pub(in crate::native_app) visible_folders: Vec<VisibleFolder>,
    pub(in crate::native_app) window: ui::VirtualListWindow,
    pub(in crate::native_app) selected_folder_status_label: String,
    pub(in crate::native_app) selected_source_missing: bool,
    pub(in crate::native_app) include_subfolders_available: bool,
    pub(in crate::native_app) include_subfolders: bool,
    pub(in crate::native_app) show_empty_folders: bool,
    pub(in crate::native_app) help_tooltips_enabled: bool,
}

pub(in crate::native_app) struct CollectionsSectionViewModel {
    pub(in crate::native_app) rows: Vec<CollectionRowViewModel>,
    pub(in crate::native_app) panel_height: f32,
    pub(in crate::native_app) list_height: f32,
}

pub(in crate::native_app) struct CollectionRowViewModel {
    pub(in crate::native_app) collection: SampleCollectionView,
    pub(in crate::native_app) rename: Option<CollectionRenameView>,
}

pub(in crate::native_app) struct FilterSectionViewModel {
    pub(in crate::native_app) sidebar_width: f32,
    pub(in crate::native_app) help_tooltips_enabled: bool,
    pub(in crate::native_app) name_filter: String,
    pub(in crate::native_app) name_filter_enabled: bool,
    pub(in crate::native_app) tag_filter: String,
    pub(in crate::native_app) tag_filter_enabled: bool,
    pub(in crate::native_app) curation: CurationFilterViewModel,
    pub(in crate::native_app) harvest: HarvestFilterViewModel,
    pub(in crate::native_app) playback_type_enabled: bool,
    pub(in crate::native_app) playback_type_filters: Vec<PlaybackTypeFilterToggleViewModel>,
    pub(in crate::native_app) rating_enabled: bool,
    pub(in crate::native_app) rating_filters: Vec<RatingFilterToggleViewModel>,
    pub(in crate::native_app) panel_height: f32,
}

pub(in crate::native_app) struct HarvestFamilyViewModel {
    pub(in crate::native_app) state_label: String,
    pub(in crate::native_app) origin_count_label: String,
    pub(in crate::native_app) derivative_count_label: String,
    pub(in crate::native_app) origin_detail: Option<String>,
    pub(in crate::native_app) derivative_detail: Option<String>,
    pub(in crate::native_app) can_show_origin: bool,
    pub(in crate::native_app) can_show_derivatives: bool,
    pub(in crate::native_app) can_open_destination: bool,
}

pub(in crate::native_app) struct CurationFilterViewModel {
    pub(in crate::native_app) enabled: bool,
    pub(in crate::native_app) dropdown_open: bool,
    pub(in crate::native_app) selected_scope: BrowserCurationScope,
    pub(in crate::native_app) options: Vec<CurationFilterOptionViewModel>,
    pub(in crate::native_app) help_tooltips_enabled: bool,
}

pub(in crate::native_app) struct CurationFilterOptionViewModel {
    pub(in crate::native_app) scope: BrowserCurationScope,
    pub(in crate::native_app) label: &'static str,
}

pub(in crate::native_app) struct HarvestFilterViewModel {
    pub(in crate::native_app) enabled: bool,
    pub(in crate::native_app) dropdown_open: bool,
    pub(in crate::native_app) selected_filter: Option<HarvestFilter>,
    pub(in crate::native_app) options: Vec<HarvestFilterOptionViewModel>,
    pub(in crate::native_app) family_available: bool,
    pub(in crate::native_app) family_open: bool,
    pub(in crate::native_app) help_tooltips_enabled: bool,
}

pub(in crate::native_app) struct HarvestFilterOptionViewModel {
    pub(in crate::native_app) filter: HarvestFilter,
    pub(in crate::native_app) label: &'static str,
}

pub(in crate::native_app) struct PlaybackTypeFilterToggleViewModel {
    pub(in crate::native_app) filter: PlaybackTypeFilter,
    pub(in crate::native_app) label: &'static str,
    pub(in crate::native_app) active: bool,
}

pub(in crate::native_app) struct RatingFilterToggleViewModel {
    pub(in crate::native_app) level: i8,
    pub(in crate::native_app) label: &'static str,
    pub(in crate::native_app) active: bool,
}

pub(in crate::native_app) struct TagEditorViewModel {
    pub(in crate::native_app) has_selected_file: bool,
    pub(in crate::native_app) help_tooltips_enabled: bool,
    pub(in crate::native_app) tag_library_open: bool,
    pub(in crate::native_app) draft: String,
    pub(in crate::native_app) tokens: Vec<String>,
    pub(in crate::native_app) pending_category_tag: Option<String>,
    pub(in crate::native_app) input_placeholder: String,
    pub(in crate::native_app) completion_suffix: Option<String>,
    pub(in crate::native_app) tags: Vec<String>,
    pub(in crate::native_app) mixed_tags: Vec<String>,
    pub(in crate::native_app) display_categories: Vec<MetadataTagDisplayCategory>,
    pub(in crate::native_app) selected_tag: Option<String>,
}

impl LibrarySidebarViewModel {
    pub(in crate::native_app) fn from_app_state(state: &NativeAppState) -> Self {
        let folder_browser = &state.library.folder_browser;
        let harvest_family = state
            .ui
            .chrome
            .harvest_family_open
            .then(|| {
                state
                    .selected_harvest_family_summary()
                    .map(HarvestFamilyViewModel::from_summary)
            })
            .flatten();
        let mut filter = FilterSectionViewModel::from_folder_browser(
            folder_browser,
            state.ui.chrome.help_tooltips_enabled,
        );
        filter.harvest.family_available =
            harvest_family.is_some() || state.selected_harvest_family_available();
        filter.harvest.family_open = state.ui.chrome.harvest_family_open;
        filter.curation.dropdown_open = state.ui.chrome.curation_filter_dropdown_open;
        filter.harvest.dropdown_open = state.ui.chrome.harvest_filter_dropdown_open;
        filter.sidebar_width = state.ui.chrome.folder_panel.size();
        Self {
            sidebar_width: state.ui.chrome.folder_panel.size(),
            metadata_panel_height: folder_browser.metadata_panel_height(),
            source_selector: SourceSelectorViewModel::from_folder_browser_with_scanning(
                folder_browser,
                state.ui.chrome.help_tooltips_enabled,
                state
                    .library
                    .folder_progress()
                    .filter(|progress| {
                        matches!(
                            progress.lifecycle,
                            crate::native_app::sample_library::folder_browser::scan::FolderScanLifecycle::Scanning
                                | crate::native_app::sample_library::folder_browser::scan::FolderScanLifecycle::ApplyingResults
                                | crate::native_app::sample_library::folder_browser::scan::FolderScanLifecycle::PersistingResults
                        )
                    })
                    .map(|progress| progress.source_id.as_str()),
            ),
            folder_tree: FolderTreeViewModel::from_folder_browser(
                folder_browser,
                state.ui.chrome.help_tooltips_enabled,
            ),
            collections: CollectionsSectionViewModel::from_folder_browser(folder_browser),
            filter,
            harvest_family,
            tag_editor: TagEditorViewModel::from_app_state(state),
        }
    }
}

impl SourceSelectorViewModel {
    #[cfg(test)]
    pub(in crate::native_app) fn from_folder_browser(
        folder_browser: &FolderBrowserState,
        help_tooltips_enabled: bool,
    ) -> Self {
        Self::from_folder_browser_with_scanning(folder_browser, help_tooltips_enabled, None)
    }

    fn from_folder_browser_with_scanning(
        folder_browser: &FolderBrowserState,
        help_tooltips_enabled: bool,
        scanning_source_id: Option<&str>,
    ) -> Self {
        let selected_source_id = folder_browser.selected_source_id();
        let rows: Vec<_> = folder_browser
            .sources()
            .iter()
            .map(|source| {
                SourceRowViewModel::from_source(
                    source,
                    selected_source_id,
                    folder_browser,
                    scanning_source_id,
                )
            })
            .collect();
        let missing_count = rows.iter().filter(|source| source.missing).count();

        Self {
            rows,
            missing_count,
            help_tooltips_enabled,
        }
    }
}

impl SourceRowViewModel {
    fn from_source(
        source: &SourceEntry,
        selected_source_id: &str,
        folder_browser: &FolderBrowserState,
        scanning_source_id: Option<&str>,
    ) -> Self {
        let reorder_drag_source =
            folder_browser.source_reorder_drag_source_id() == Some(source.id.as_str());
        let reorder_drop_after = folder_browser.source_reorder_drop_marker_after(&source.id);
        let selected = selected_source_id == source.id;
        let focus_alpha = if selected && folder_browser.source_keyboard_focus_active() {
            folder_browser.keyboard_focus_alpha()
        } else {
            0
        };
        let focused = focus_alpha > 0;
        Self {
            id: source.id.clone(),
            label: source.label.clone(),
            role: source.role,
            selected,
            focused,
            focus_alpha,
            reorder_enabled: folder_browser.source_reorder_enabled(&source.id),
            reorder_drag_active: folder_browser.source_reorder_drag_active(),
            reorder_drag_source,
            reorder_drop_target: reorder_drop_after.is_some(),
            reorder_drop_after: reorder_drop_after.unwrap_or(false),
            scanning: scanning_source_id == Some(source.id.as_str()),
            missing: source.is_missing(),
            protected_source_error_flash: folder_browser
                .source_protected_error_flash_active(&source.id),
            primary_source_acceptance_flash: source.role == SourceRole::Primary
                && folder_browser.primary_source_acceptance_flash_active(),
            drag_active: folder_browser.drag_active(),
            drop_candidate: folder_browser.can_drop_drag_on_source(&source.id),
            drop_target: folder_browser.hovered_drop_target_source_id().as_deref()
                == Some(source.id.as_str()),
            drop_target_active: folder_browser.hovered_drop_target_source_id().is_some(),
        }
    }
}

impl FolderTreeViewModel {
    fn from_folder_browser(
        folder_browser: &FolderBrowserState,
        help_tooltips_enabled: bool,
    ) -> Self {
        let visible_folders = folder_browser.visible_folders();
        let window = folder_browser.tree_view_window(
            &visible_folders,
            FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
            FOLDER_TREE_OVERSCAN_ROWS,
            FOLDER_TREE_EDGE_CONTEXT_ROWS,
        );

        Self {
            visible_folders,
            window,
            selected_folder_status_label: folder_browser.selected_folder_status_label(),
            selected_source_missing: folder_browser
                .source_is_missing(folder_browser.selected_source_id()),
            include_subfolders_available: folder_browser.folder_subtree_listing_available(),
            include_subfolders: folder_browser.folder_subtree_listing_enabled(),
            show_empty_folders: folder_browser.empty_folder_visibility_enabled(),
            help_tooltips_enabled,
        }
    }
}

impl CollectionsSectionViewModel {
    pub(in crate::native_app) fn from_folder_browser(folder_browser: &FolderBrowserState) -> Self {
        let rows = folder_browser
            .visible_collections()
            .into_iter()
            .map(|collection| {
                let rename = folder_browser.collection_rename_view(collection.collection);
                CollectionRowViewModel { collection, rename }
            })
            .collect::<Vec<_>>();
        Self {
            rows,
            panel_height: folder_browser.collections_panel_height(),
            list_height: folder_browser.collections_list_height(),
        }
    }
}

impl FilterSectionViewModel {
    pub(in crate::native_app) fn from_folder_browser(
        folder_browser: &FolderBrowserState,
        help_tooltips_enabled: bool,
    ) -> Self {
        Self {
            sidebar_width: 240.0,
            help_tooltips_enabled,
            name_filter: folder_browser.name_filter().to_owned(),
            name_filter_enabled: folder_browser.name_filter_enabled(),
            tag_filter: folder_browser.tag_filter().to_owned(),
            tag_filter_enabled: folder_browser.tag_filter_enabled(),
            curation: CurationFilterViewModel {
                enabled: folder_browser.curation_mode_enabled(),
                dropdown_open: false,
                selected_scope: folder_browser.curation_scope(),
                options: BROWSER_CURATION_SCOPES
                    .into_iter()
                    .map(|scope| CurationFilterOptionViewModel {
                        scope,
                        label: curation_scope_dropdown_label(scope),
                    })
                    .collect(),
                help_tooltips_enabled,
            },
            harvest: HarvestFilterViewModel {
                enabled: folder_browser.harvest_filter_enabled(),
                dropdown_open: false,
                selected_filter: Some(folder_browser.selected_harvest_filter()),
                options: HARVEST_FILTERS
                    .into_iter()
                    .map(|filter| HarvestFilterOptionViewModel {
                        filter,
                        label: harvest_filter_dropdown_label(filter),
                    })
                    .collect(),
                family_available: false,
                family_open: false,
                help_tooltips_enabled,
            },
            playback_type_enabled: folder_browser.playback_type_filter_enabled(),
            playback_type_filters: PLAYBACK_TYPE_FILTERS
                .into_iter()
                .map(|filter| PlaybackTypeFilterToggleViewModel {
                    filter,
                    label: playback_type_filter_label(filter),
                    active: folder_browser.playback_type_filter().contains(&filter),
                })
                .collect(),
            rating_enabled: folder_browser.rating_filter_enabled(),
            rating_filters: RATING_FILTER_LEVELS
                .into_iter()
                .map(|level| RatingFilterToggleViewModel {
                    level,
                    label: rating_filter_label(level),
                    active: folder_browser.rating_filter().contains(&level),
                })
                .collect(),
            panel_height: folder_browser.filter_panel_height(),
        }
    }
}

fn harvest_filter_dropdown_label(filter: HarvestFilter) -> &'static str {
    match filter {
        HarvestFilter::New => "New",
        HarvestFilter::NewAndTouched => "New + Touched",
        HarvestFilter::NeedsReview => "Needs Review",
        HarvestFilter::Touched => "Touched",
        HarvestFilter::HasDerivatives => "Has Derivatives",
        HarvestFilter::NoDerivatives => "No Derivatives",
        HarvestFilter::Done => "Done",
        HarvestFilter::Ignored => "Ignored",
        HarvestFilter::All => "All",
    }
}

fn curation_scope_dropdown_label(scope: BrowserCurationScope) -> &'static str {
    match scope {
        BrowserCurationScope::All => "All",
        BrowserCurationScope::Ratings => "Rate",
        BrowserCurationScope::Tags => "Tags",
    }
}

impl HarvestFamilyViewModel {
    fn from_summary(summary: HarvestFamilySummary) -> Self {
        Self {
            state_label: summary.state_label,
            origin_count_label: related_count_display(summary.origin_count),
            derivative_count_label: related_count_display(summary.derivative_count),
            origin_detail: related_detail(summary.first_origin_label, summary.missing_origin_count),
            derivative_detail: related_detail(
                summary.first_derivative_label,
                summary.missing_derivative_count,
            ),
            can_show_origin: summary.origin_count > 0,
            can_show_derivatives: summary.derivative_count > 0,
            can_open_destination: true,
        }
    }
}

impl TagEditorViewModel {
    fn from_app_state(state: &NativeAppState) -> Self {
        Self {
            has_selected_file: state.library.folder_browser.selected_file_id().is_some(),
            help_tooltips_enabled: state.ui.chrome.help_tooltips_enabled,
            tag_library_open: state.metadata.tag_library_open,
            draft: state.metadata.tag_draft.clone(),
            tokens: state.metadata.tag_tokens.clone(),
            pending_category_tag: state
                .pending_metadata_tag_category_tag()
                .map(str::to_string),
            input_placeholder: state.metadata_tag_input_placeholder().to_string(),
            completion_suffix: state.metadata_tag_completion_suffix(),
            tags: state.selected_metadata_tags_for_display(),
            mixed_tags: state.mixed_selected_metadata_tags_for_display(),
            display_categories: state.selected_metadata_tag_display_categories(),
            selected_tag: state.metadata.selected_tag.clone(),
        }
    }
}

fn related_count_display(count: usize) -> String {
    match count {
        0 => String::from("None"),
        1 => String::from("1"),
        count => count.to_string(),
    }
}

fn related_detail(label: Option<String>, missing_count: usize) -> Option<String> {
    match (label, missing_count) {
        (Some(label), 0) => Some(label),
        (Some(label), 1) => Some(format!("{label} | 1 missing")),
        (Some(label), count) => Some(format!("{label} | {count} missing")),
        (None, 0) => None,
        (None, 1) => Some(String::from("1 missing")),
        (None, count) => Some(format!("{count} missing")),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::LibrarySidebarViewModel;
    use crate::native_app::test_support::state::{
        FolderBrowserState, FolderScanProgress, NativeAppStateFixture, SourceProcessingProgress,
    };
    use wavecrate::sample_sources::{SampleSource, SourceId};

    #[test]
    fn closed_harvest_family_drawer_keeps_sidebar_projection_cheap() {
        let source_root = tempfile::tempdir().expect("source root");
        let sample_path = source_root.path().join("kick.wav");
        fs::write(&sample_path, []).expect("sample file");
        let mut state = NativeAppStateFixture::default()
            .with_folder_browser(FolderBrowserState::from_sample_sources(&[
                wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
            ]))
            .build();
        state
            .library
            .folder_browser
            .select_file(sample_path.display().to_string());
        state.ui.chrome.harvest_family_open = false;

        let model = LibrarySidebarViewModel::from_app_state(&state);

        assert!(model.harvest_family.is_none());
        assert!(model.filter.harvest.family_available);
    }

    #[test]
    fn queued_scan_stays_visually_idle_behind_processing_source() {
        let processing_root = tempfile::tempdir().expect("processing source root");
        let queued_root = tempfile::tempdir().expect("queued scan root");
        let processing_source = SampleSource::new_with_id(
            SourceId::from_string("processing-source"),
            processing_root.path().to_path_buf(),
        );
        let queued_source = SampleSource::new_with_id(
            SourceId::from_string("queued-source"),
            queued_root.path().to_path_buf(),
        );
        let mut state = NativeAppStateFixture::default()
            .with_folder_browser(FolderBrowserState::from_sample_sources(&[
                processing_source.clone(),
                queued_source.clone(),
            ]))
            .build();
        let request = state
            .library
            .begin_source_scan(queued_source.id.as_str().to_string(), 17)
            .expect("queue source scan");
        state.library.start_folder_scan(&request);
        state.background.source_processing_progress = Some(SourceProcessingProgress {
            source_id: processing_source.id.as_str().to_string(),
            lifecycle_generation: 1,
            active: true,
            source_row_active: true,
            completed: 2,
            total: 10,
            stage: String::from("Preparing analysis"),
            detail: String::from("kick.wav"),
        });

        let waiting = LibrarySidebarViewModel::from_app_state(&state);
        let queued_row = waiting
            .source_selector
            .rows
            .iter()
            .find(|row| row.id == queued_source.id.as_str())
            .expect("queued row");
        assert!(
            !queued_row.scanning,
            "a scan waiting for the single source lane must not claim to be scanning"
        );

        assert!(
            state
                .library
                .apply_folder_scan_progress(FolderScanProgress::new(
                    request.task_id,
                    request.source_id.clone(),
                    request.label,
                    crate::native_app::sample_library::folder_browser::scan::FolderScanLifecycle::Scanning,
                    1,
                    10,
                    String::from("snare.wav"),
                ))
        );
        let admitted = LibrarySidebarViewModel::from_app_state(&state);
        let admitted_row = admitted
            .source_selector
            .rows
            .iter()
            .find(|row| row.id == queued_source.id.as_str())
            .expect("admitted scan row");
        assert!(
            admitted_row.scanning,
            "the scan label should appear only after worker progress proves admission"
        );

        state.background.source_processing_progress = Some(SourceProcessingProgress {
            source_id: processing_source.id.as_str().to_string(),
            lifecycle_generation: 1,
            active: true,
            source_row_active: false,
            completed: 4,
            total: 10,
            stage: String::from("Scanning source changes"),
            detail: String::from("Checking the source manifest"),
        });
        let maintenance = LibrarySidebarViewModel::from_app_state(&state);
        assert!(
            maintenance
                .source_selector
                .rows
                .iter()
                .any(|row| row.id == processing_source.id.as_str()),
            "manifest maintenance must preserve the source row projection"
        );

        state.background.source_processing_progress = Some(SourceProcessingProgress {
            source_id: processing_source.id.as_str().to_string(),
            lifecycle_generation: 1,
            active: true,
            source_row_active: true,
            completed: 0,
            total: 0,
            stage: String::from("Queueing unfinished work"),
            detail: String::from("128 / 256 readiness targets checked"),
        });
        let sustained_discovery = LibrarySidebarViewModel::from_app_state(&state);
        assert!(
            sustained_discovery
                .source_selector
                .rows
                .iter()
                .any(|row| row.id == processing_source.id.as_str()),
            "grace-surviving discovery must preserve the active source row"
        );
    }
}
