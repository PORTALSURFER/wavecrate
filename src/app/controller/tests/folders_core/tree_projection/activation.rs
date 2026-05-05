use super::*;

#[test]
fn activating_expandable_folder_row_expands_then_collapses_it() -> Result<(), String> {
    let mut controller = nested_tree_controller()?;

    let drums = folder_row_index(&controller, "drums");
    controller.toggle_folder_expanded(drums);
    let drums = folder_row_index(&controller, "drums");
    controller.activate_folder_row(drums);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/kicks"))
    );

    let drums = folder_row_index(&controller, "drums");
    controller.activate_folder_row(drums);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path != PathBuf::from("drums/kicks"))
    );
    Ok(())
}

#[test]
fn reactivating_folder_after_focusing_elsewhere_keeps_it_expanded() -> Result<(), String> {
    let mut controller = nested_tree_controller()?;

    let drums = folder_row_index(&controller, "drums");
    controller.toggle_folder_expanded(drums);
    let drums = folder_row_index(&controller, "drums");
    controller.activate_folder_row(drums);
    controller.ui.focus.context = FocusContext::Waveform;

    let drums = folder_row_index(&controller, "drums");
    controller.activate_folder_row(drums);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/kicks"))
    );
    Ok(())
}

#[test]
fn activating_leaf_folder_row_keeps_tree_projection_unchanged() -> Result<(), String> {
    let mut controller = nested_tree_controller()?;

    let before: Vec<PathBuf> = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .map(|row| row.path.clone())
        .collect();
    let kicks = folder_row_index(&controller, "drums/kicks");
    controller.activate_folder_row(kicks);

    let after: Vec<PathBuf> = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .map(|row| row.path.clone())
        .collect();
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn activating_folder_search_result_does_not_toggle_expansion() -> Result<(), String> {
    let mut controller = nested_tree_controller()?;

    controller.set_folder_search(String::from("drum"));
    let drums = folder_row_index(&controller, "drums");
    controller.activate_folder_row(drums);
    controller.set_folder_search(String::new());

    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/kicks"))
    );
    Ok(())
}
