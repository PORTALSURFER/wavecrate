use super::support::*;

fn folder_row_index(controller: &AppController, path: &str) -> usize {
    controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from(path))
        .unwrap_or_else(|| panic!("missing folder row for {path}"))
}

fn nested_tree_controller() -> Result<AppController, String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    Ok(controller)
}

mod activation;
mod focus_navigation;
mod structure;
