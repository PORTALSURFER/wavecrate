use super::*;

#[test]
fn collapsing_leaf_folder_focuses_parent_row() -> Result<(), String> {
    let mut controller = nested_tree_controller()?;
    let kicks = folder_row_index(&controller, "drums/kicks");
    controller.replace_folder_selection(kicks);
    controller.collapse_focused_folder();

    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums")
    );
    Ok(())
}

#[test]
fn nudge_folder_focus_moves_through_visible_rows() -> Result<(), String> {
    let mut controller = nested_tree_controller()?;

    controller.nudge_folder_focus_action(1);
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after first move");
    assert_eq!(controller.ui.sources.folders.rows[focused].path, PathBuf::from(""));

    controller.nudge_folder_focus_action(1);
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after second move");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums")
    );

    controller.nudge_folder_focus_action(1);
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after third move");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums/kicks")
    );

    controller.nudge_folder_focus_action(-1);
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after moving back up");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums")
    );
    Ok(())
}

#[test]
fn nudge_folder_focus_skips_collapsed_descendants() -> Result<(), String> {
    let mut controller = nested_tree_controller()?;

    let drums = folder_row_index(&controller, "drums");
    controller.toggle_folder_expanded(drums);
    controller.nudge_folder_focus_action(1);
    controller.nudge_folder_focus_action(1);

    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after moving through collapsed tree");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums")
    );

    controller.nudge_folder_focus_action(1);
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after clamped move");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums")
    );
    Ok(())
}
