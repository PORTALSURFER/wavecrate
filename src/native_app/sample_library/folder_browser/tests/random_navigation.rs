use super::*;

fn browser_with_samples(name: &str, files: &[&str]) -> (FolderBrowserState, PathBuf) {
    let root = temp_source_root(name);
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    for file in files {
        fs::write(drums.join(file), [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    (browser, root)
}

#[test]
fn random_navigation_forward_avoids_repeats_until_result_set_exhaustion() {
    let (mut browser, root) = browser_with_samples(
        "wavecrate-gui-random-nav-forward",
        &["hat.wav", "kick.wav", "snare.wav", "tom.wav"],
    );
    let drums = root.join("drums");
    let hat = path_id(&drums.join("hat.wav"));
    browser.select_file(hat.clone());
    browser.toggle_random_navigation();

    let first = browser
        .navigate_vertical_matching_tags(1, false, false, &Default::default())
        .expect("first random target");
    let second = browser
        .navigate_vertical_matching_tags(1, false, false, &Default::default())
        .expect("second random target");
    let third = browser
        .navigate_vertical_matching_tags(1, false, false, &Default::default())
        .expect("third random target");

    let visited = [hat, first, second, third]
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        visited.len(),
        4,
        "random navigation should visit each visible sample before repeating"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn random_navigation_previous_walks_history() {
    let (mut browser, root) = browser_with_samples(
        "wavecrate-gui-random-nav-previous",
        &["hat.wav", "kick.wav", "snare.wav"],
    );
    let drums = root.join("drums");
    browser.select_file(path_id(&drums.join("hat.wav")));
    browser.toggle_random_navigation();

    let first = browser
        .navigate_vertical_matching_tags(1, false, false, &Default::default())
        .expect("first random target");
    let second = browser
        .navigate_vertical_matching_tags(1, false, false, &Default::default())
        .expect("second random target");
    assert_ne!(first, second);

    assert_eq!(
        browser.navigate_vertical_matching_tags(-1, false, false, &Default::default()),
        Some(first.clone())
    );
    assert_eq!(browser.selected_file_id(), Some(first.as_str()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn random_navigation_reconciles_when_result_set_changes() {
    let (mut browser, root) = browser_with_samples(
        "wavecrate-gui-random-nav-filter",
        &["hat.wav", "kick.wav", "snare.wav"],
    );
    let drums = root.join("drums");
    let hat = path_id(&drums.join("hat.wav"));
    browser.select_file(hat.clone());
    browser.toggle_random_navigation();
    let first = browser
        .navigate_vertical_matching_tags(1, false, false, &Default::default())
        .expect("first random target");
    assert_ne!(first, hat);

    browser.apply_message(FolderBrowserMessage::NameFilterInput(
        TextInputMessage::Changed {
            value: String::from("hat"),
        },
    ));

    assert_eq!(
        browser.navigate_vertical_matching_tags(-1, false, false, &Default::default()),
        None,
        "history should reset when the visible result set changes"
    );
    assert_eq!(
        browser.navigate_vertical_matching_tags(1, false, false, &Default::default()),
        None,
        "one-item result sets should not random-jump"
    );

    browser.apply_message(FolderBrowserMessage::NameFilterInput(
        TextInputMessage::Changed {
            value: String::from("missing"),
        },
    ));
    assert_eq!(
        browser.navigate_vertical_matching_tags(1, false, false, &Default::default()),
        None,
        "empty result sets should not random-jump"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn linear_navigation_restores_when_random_mode_is_off() {
    let (mut browser, root) = browser_with_samples(
        "wavecrate-gui-random-nav-off",
        &["hat.wav", "kick.wav", "snare.wav"],
    );
    let drums = root.join("drums");
    browser.select_file(path_id(&drums.join("hat.wav")));
    browser.toggle_random_navigation();
    assert!(browser.random_navigation_enabled());
    browser.toggle_random_navigation();
    assert!(!browser.random_navigation_enabled());

    assert_eq!(
        browser.navigate_vertical_matching_tags(1, false, false, &Default::default()),
        Some(path_id(&drums.join("kick.wav")))
    );

    let _ = fs::remove_dir_all(root);
}
