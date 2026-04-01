use super::*;

#[test]
fn folder_tree_uses_root_label_and_child_depths() -> Result<(), String> {
    let controller = nested_tree_controller()?;

    let root = controller
        .ui
        .sources
        .folders
        .rows
        .first()
        .expect("root row should be present");
    assert_eq!(root.name, "Root");
    assert_eq!(root.depth, 0);

    let drums = &controller.ui.sources.folders.rows[folder_row_index(&controller, "drums")];
    let kicks = &controller.ui.sources.folders.rows[folder_row_index(&controller, "drums/kicks")];
    assert_eq!(drums.depth, 1);
    assert_eq!(kicks.depth, 2);
    Ok(())
}

#[test]
fn toggling_folder_expansion_hides_and_restores_descendants() -> Result<(), String> {
    let mut controller = nested_tree_controller()?;
    let drums = folder_row_index(&controller, "drums");
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/kicks"))
    );

    controller.toggle_folder_expanded(drums);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path != PathBuf::from("drums/kicks"))
    );

    let drums = folder_row_index(&controller, "drums");
    controller.toggle_folder_expanded(drums);
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
