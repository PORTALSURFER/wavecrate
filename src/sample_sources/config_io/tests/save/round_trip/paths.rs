use super::*;

#[test]
fn save_paths_and_library_selection_round_trip() {
    let fixture = settings_round_trip_fixture();

    assert_eq!(
        fixture.actual.core.app_data_dir,
        fixture.expected.core.app_data_dir
    );
    assert_eq!(
        fixture.actual.core.trash_folder,
        fixture.expected.core.trash_folder
    );
    assert_eq!(
        fixture
            .actual
            .core
            .drop_targets
            .iter()
            .map(|target| (&target.path, target.color))
            .collect::<Vec<_>>(),
        fixture
            .expected
            .core
            .drop_targets
            .iter()
            .map(|target| (&target.path, target.color))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        fixture.actual.core.last_selected_source,
        fixture.expected.core.last_selected_source
    );
    assert_eq!(
        fixture.actual.core.upper_folder_pane_source,
        fixture.expected.core.upper_folder_pane_source
    );
    assert_eq!(
        fixture.actual.core.lower_folder_pane_source,
        fixture.expected.core.lower_folder_pane_source
    );
    assert_eq!(
        fixture.actual.core.active_folder_pane,
        fixture.expected.core.active_folder_pane
    );
    assert_eq!(
        fixture.actual.core.collection_names,
        fixture.expected.core.collection_names
    );
    assert_eq!(
        fixture
            .actual
            .sources
            .iter()
            .map(|source| (source.id.as_str(), &source.root))
            .collect::<Vec<_>>(),
        fixture
            .expected
            .sources
            .iter()
            .map(|source| (source.id.as_str(), &source.root))
            .collect::<Vec<_>>()
    );
}
