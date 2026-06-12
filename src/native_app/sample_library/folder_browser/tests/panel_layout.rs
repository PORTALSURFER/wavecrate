use super::*;
use crate::native_app::sample_library::folder_browser::{
    collections::{COLLAPSED_COLLECTIONS_PANEL_HEIGHT, MIN_COLLECTIONS_PANEL_HEIGHT},
    test_support::{COLLAPSED_FILTER_PANEL_HEIGHT, COLLAPSED_METADATA_PANEL_HEIGHT},
};
#[test]
fn collections_panel_splitter_resizes_and_clamps_height() {
    let root = temp_source_root("wavecrate-gui-collections-panel-resize");
    let mut browser = FolderBrowserState::from_root(root.clone());

    assert_eq!(
        browser.panel_layout.collections.size(),
        super::super::DEFAULT_COLLECTIONS_PANEL_HEIGHT
    );

    browser.resize_collections_panel(DragHandleMessage::started(Point::new(0.0, 200.0)));
    browser.resize_collections_panel(DragHandleMessage::moved(Point::new(0.0, 120.0)));
    assert_eq!(
        browser.panel_layout.collections.size(),
        browser.max_collections_panel_height()
    );

    browser.resize_collections_panel(DragHandleMessage::moved(Point::new(0.0, 1_000.0)));
    assert_eq!(
        browser.panel_layout.collections.size(),
        MIN_COLLECTIONS_PANEL_HEIGHT
    );

    browser.resize_collections_panel(DragHandleMessage::ended(Point::new(0.0, -1_000.0)));
    assert_eq!(
        browser.panel_layout.collections.size(),
        browser.max_collections_panel_height()
    );
    assert!(!browser.panel_layout.collections.is_resizing());

    let _ = fs::remove_dir_all(root);
}
#[test]
fn collections_panel_splitter_double_click_collapses_height() {
    let root = temp_source_root("wavecrate-gui-collections-panel-collapse");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let initial_height = browser.panel_layout.collections.size();
    browser.resize_collections_panel(DragHandleMessage::started(Point::new(0.0, 200.0)));

    browser.resize_collections_panel(DragHandleMessage::double_activate(Point::new(0.0, 200.0)));

    assert_eq!(
        browser.panel_layout.collections.size(),
        COLLAPSED_COLLECTIONS_PANEL_HEIGHT
    );
    assert!(!browser.panel_layout.collections.is_resizing());

    browser.resize_collections_panel(DragHandleMessage::double_activate(Point::new(0.0, 200.0)));

    assert_eq!(browser.panel_layout.collections.size(), initial_height);
    assert!(!browser.panel_layout.collections.is_resizing());

    let _ = fs::remove_dir_all(root);
}
#[test]
fn filter_panel_splitter_resizes_and_clamps_height() {
    let root = temp_source_root("wavecrate-gui-filter-panel-resize");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let initial_height = browser.panel_layout.filter.size();

    browser.resize_filter_panel(DragHandleMessage::started(Point::new(0.0, 200.0)));
    browser.resize_filter_panel(DragHandleMessage::moved(Point::new(0.0, 120.0)));

    assert!(browser.panel_layout.filter.size() > initial_height);

    browser.resize_filter_panel(DragHandleMessage::moved(Point::new(0.0, 1_000.0)));

    assert_eq!(
        browser.panel_layout.filter.size(),
        COLLAPSED_FILTER_PANEL_HEIGHT
    );

    browser.resize_filter_panel(DragHandleMessage::ended(Point::new(0.0, 1_000.0)));

    assert!(!browser.panel_layout.filter.is_resizing());
    let _ = fs::remove_dir_all(root);
}
#[test]
fn filter_panel_double_click_collapses_to_header_only_height() {
    let root = temp_source_root("wavecrate-gui-filter-panel-collapse");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let initial_height = browser.panel_layout.filter.size();

    browser.resize_filter_panel(DragHandleMessage::double_activate(Point::new(0.0, 200.0)));

    assert_eq!(
        browser.panel_layout.filter.size(),
        COLLAPSED_FILTER_PANEL_HEIGHT
    );
    assert!(!browser.panel_layout.filter.is_resizing());

    browser.resize_filter_panel(DragHandleMessage::double_activate(Point::new(0.0, 200.0)));

    assert_eq!(browser.panel_layout.filter.size(), initial_height);
    assert!(!browser.panel_layout.filter.is_resizing());
    let _ = fs::remove_dir_all(root);
}
#[test]
fn metadata_panel_splitter_resizes_and_clamps_height() {
    let root = temp_source_root("wavecrate-gui-metadata-panel-resize");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let initial_height = browser.panel_layout.metadata.size();

    browser.resize_metadata_panel(DragHandleMessage::started(Point::new(0.0, 200.0)));
    browser.resize_metadata_panel(DragHandleMessage::moved(Point::new(0.0, 120.0)));

    assert!(browser.panel_layout.metadata.size() > initial_height);

    browser.resize_metadata_panel(DragHandleMessage::moved(Point::new(0.0, 1_000.0)));

    assert_eq!(
        browser.panel_layout.metadata.size(),
        COLLAPSED_METADATA_PANEL_HEIGHT
    );

    browser.resize_metadata_panel(DragHandleMessage::ended(Point::new(0.0, 1_000.0)));

    assert!(!browser.panel_layout.metadata.is_resizing());
    let _ = fs::remove_dir_all(root);
}
#[test]
fn metadata_panel_double_click_collapses_to_header_only_height() {
    let root = temp_source_root("wavecrate-gui-metadata-panel-collapse");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let initial_height = browser.panel_layout.metadata.size();

    browser.resize_metadata_panel(DragHandleMessage::double_activate(Point::new(0.0, 200.0)));

    assert_eq!(
        browser.panel_layout.metadata.size(),
        COLLAPSED_METADATA_PANEL_HEIGHT
    );
    assert!(!browser.panel_layout.metadata.is_resizing());

    browser.resize_metadata_panel(DragHandleMessage::double_activate(Point::new(0.0, 200.0)));

    assert_eq!(browser.panel_layout.metadata.size(), initial_height);
    assert!(!browser.panel_layout.metadata.is_resizing());
    let _ = fs::remove_dir_all(root);
}
